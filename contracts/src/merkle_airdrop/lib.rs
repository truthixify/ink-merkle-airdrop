#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # Merkle Airdrop Contract
///
/// This contract enables ERC20-style token distributions using a Merkle root.
/// Eligible recipients and claim amounts are committed to in a Merkle tree
/// off-chain. Each recipient proves eligibility on-chain by providing a Merkle
/// proof for their `(address, amount)` leaf.
///
/// ## Key Features
/// - Efficient distribution: only the root of the Merkle tree is stored.
/// - Trustless claims: recipients self-claim with Merkle proofs.
/// - Double-claim protection: each recipient can only claim once.
/// - Claim window: contract owner can configure an end time.
/// - Sweep: owner can recover unclaimed tokens after the campaign ends.
///
/// ## Storage
/// - `asset_contract`: reference to an ERC20-compatible token contract.
/// - `root`: Merkle root committing to `(address, amount)` pairs.
/// - `claimed`: mapping to track which addresses have claimed.
/// - `owner`: deployer of the contract, authorized for admin actions.
/// - `campaign_end_time`: block timestamp after which claiming stops.
pub use self::merke_airdrop::*;

#[ink::contract]
mod merke_airdrop {
    use assets::asset_hub_precompile::{AssetHubPrecompileRef, Erc20};
    use assets::AssetId;
    use ink::env::hash_bytes;
    use ink::env::{
        call::FromAddr,
        hash::{HashOutput, Keccak256},
    };
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use ink::U256;

    /// Compute `keccak256(left || right)`.
    fn hash(left: &[u8], right: &[u8]) -> [u8; 32] {
        let mut input = Vec::with_capacity(left.len() + right.len());
        input.extend_from_slice(left);
        input.extend_from_slice(right);
        let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
        hash_bytes::<Keccak256>(&input, &mut output);

        output
    }

    /// Verify that a leaf is part of a Merkle tree with the given root.
    fn verify_proof<'a>(leaf: [u8; 32], proof: &'a [[u8; 32]], index: u64, root: [u8; 32]) -> bool {
        let mut computed = leaf;
        let mut index = index;

        for sibling in proof.iter() {
            if index % 2 == 0 {
                computed = hash(&computed, sibling); // current node is left child
            } else {
                computed = hash(sibling, &computed); // current node is right child
            }
            index /= 2;
        }

        computed == root
    }

    /// Event emitted when a recipient successfully claims their airdrop.
    #[ink(event)]
    pub struct Claimed {
        /// The address of the recipient.
        #[ink(topic)]
        recipient: Address,
        /// Amount of tokens claimed.
        value: U256,
    }

    /// Errors that can occur when funding, claiming, or sweeping.
    #[derive(Debug, PartialEq, Eq, ink::SolErrorDecode, ink::SolErrorEncode)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Token transfer failed.
        TransferFailed,
        /// Merkle proof did not validate against the stored root.
        InvalidProof,
        /// Recipient has already claimed their allocation.
        AlreadyClaimed,
        /// Cannot fund with an amount of zero.
        AmountCannotBeZero,
        /// Caller is not the owner.
        Unauthorized,
        /// Claiming is no longer allowed (campaign ended).
        ClaimPeriodOver,
        /// Claim period is still active (sweep not yet allowed).
        ClaimPeriodActive,
    }

    /// Standard `Result` type for contract operations.
    pub type Result<T> = core::result::Result<T, Error>;

    /// Merkle-based ERC20 token airdrop contract.
    #[ink(storage)]
    pub struct MerkleAirdrop {
        /// Reference to the ERC20-compatible asset contract.
        pub asset_contract: AssetHubPrecompileRef,
        /// Merkle root committing to `(address, amount)` pairs.
        pub root: [u8; 32],
        /// Tracks whether an address has already claimed.
        pub claimed: Mapping<Address, bool>,
        /// Owner authorized for administrative functions.
        pub owner: Address,
        /// Block timestamp after which claims are rejected.
        pub campaign_end_time: u64,
    }

    impl MerkleAirdrop {
        /// Create a new Merkle airdrop contract.
        ///
        /// Initializes the distribution campaign by:
        /// - setting the ERC20 asset contract reference,
        /// - committing to the Merkle root,
        /// - configuring the claim window,
        /// - recording the contract owner.
        ///
        /// **Note:** This constructor does not transfer in the campaign tokens.
        /// The caller must invoke [`fund`] immediately after deployment
        /// to lock the tokens needed for the campaign.
        ///
        /// # Arguments
        /// - `asset_contract_address`: address of the asset contract code.
        /// - `root`: Merkle root of the distribution tree.
        /// - `campaign_end_time`: block timestamp when claiming stops.
        ///
        /// # Panics
        /// - If the provided `campaign_end_time` is already in the past.
        #[ink(constructor, payable)]
        pub fn new(
            asset_contract_address: Address,
            root: [u8; 32],
            campaign_end_time: u64,
        ) -> Self {
            let now = Self::env().block_timestamp();
            // Fail if campaign already ended or ends immediately
            assert!(
                campaign_end_time > now,
                "Campaign end time must be in the future"
            );

            let caller = Self::env().caller();
            let asset_contract = AssetHubPrecompileRef::from_addr(asset_contract_address);

            Self {
                asset_contract,
                root,
                claimed: Mapping::new(),
                owner: caller,
                campaign_end_time,
            }
        }

        /// Fund the Merkle airdrop campaign.
        ///
        /// Locks the specified amount of ERC20-compatible tokens
        /// into the contract, so they can later be claimed by recipients.
        ///
        /// # Arguments
        /// - `total_airdrop_amount`: amount of tokens to transfer from the caller
        ///   into the contract for distribution.
        ///
        /// # Behavior
        /// - Transfers `total_airdrop_amount` tokens from the caller into this contract.
        /// - Requires the caller to have approved this contract to spend
        ///   at least `total_airdrop_amount` tokens beforehand.
        ///
        /// # Errors
        /// - [`Error::AmountCannotBeZero`]: if the amount is zero.
        /// - [`Error::TransferFailed`]: if the token transfer fails.
        #[ink(message)]
        pub fn fund(&mut self, total_airdrop_amount: U256) -> Result<()> {
            if total_airdrop_amount.is_zero() {
                return Err(Error::AmountCannotBeZero);
            }

            let caller = self.env().caller();
            let contract = self.env().address();

            let transferred =
                self.asset_contract
                    .transferFrom(caller, contract, total_airdrop_amount);

            match transferred {
                Ok(true) => Ok(()),
                _ => Err(Error::TransferFailed),
            }
        }

        /// Claim tokens from the Merkle airdrop.
        ///
        /// # Arguments
        /// - `value`: claim amount for the recipient.
        /// - `proof`: Merkle proof for `(recipient, value)`.
        /// - `index`: leaf index in the Merkle tree.
        ///
        /// # Errors
        /// - [`Error::AlreadyClaimed`]: if recipient already claimed.
        /// - [`Error::InvalidProof`]: if Merkle proof does not validate.
        /// - [`Error::TransferFailed`]: if token transfer fails.
        /// - [`Error::ClaimPeriodOver`]: if campaign already ended.
        #[ink(message)]
        pub fn claim(&mut self, value: U256, proof: Vec<[u8; 32]>, index: u64) -> Result<()> {
            self.check_campaign_ongoing()?;

            let recipient = self.env().caller();
            let already_claimed = self.is_claimed(recipient);

            if already_claimed {
                return Err(Error::AlreadyClaimed);
            }

            let recipient_bytes = recipient.as_bytes();
            let value_bytes = value.to_big_endian();
            let leaf = hash(recipient_bytes, &value_bytes);
            let verified = verify_proof(leaf, &proof, index, self.root);

            if !verified {
                return Err(Error::InvalidProof);
            }

            self.claimed.insert(recipient, &true);

            let transferred = self.asset_contract.transfer(recipient, value);

            if transferred.is_err() {
                return Err(Error::TransferFailed);
            }

            self.env().emit_event(Claimed { recipient, value });

            Ok(())
        }

        /// Sweep unclaimed tokens after the campaign has ended.
        ///
        /// Transfers the remaining balance from the contract back to the owner.
        ///
        /// # Errors
        /// - [`Error::Unauthorized`]: if caller is not the owner.
        /// - [`Error::ClaimPeriodActive`]: if the claim window is still open.
        #[ink(message)]
        pub fn sweep_unclaimed(&mut self) -> Result<()> {
            self.check_owner()?;
            self.check_campaign_ended()?;

            let contract = self.env().address();
            let caller = self.env().caller();
            let balance = self.asset_contract.balanceOf(contract);

            let transferred = self.asset_contract.transfer(caller, balance);

            if transferred.is_err() {
                return Err(Error::TransferFailed);
            }

            Ok(())
        }

        /// Get the token asset id of the asset contract.
        #[ink(message)]
        pub fn asset_id(&self) -> AssetId {
            self.asset_contract.assetId()
        }

        /// Get the Merkle root.
        #[ink(message)]
        pub fn root(&self) -> [u8; 32] {
            self.root
        }

        /// Check if a recipient has already claimed.
        #[ink(message)]
        pub fn is_claimed(&self, recipient: Address) -> bool {
            self.claimed.get(recipient).unwrap_or(false)
        }

        /// Internal: ensure caller is owner.
        fn check_owner(&self) -> Result<()> {
            if self.owner != self.env().caller() {
                return Err(Error::Unauthorized);
            }

            Ok(())
        }

        /// Internal: ensure campaign has not yet ended.
        fn check_campaign_ongoing(&self) -> Result<()> {
            if self.env().block_timestamp() > self.campaign_end_time {
                return Err(Error::ClaimPeriodOver);
            }

            Ok(())
        }

        /// Internal: ensure campaign has ended.
        fn check_campaign_ended(&self) -> Result<()> {
            if self.env().block_timestamp() <= self.campaign_end_time {
                return Err(Error::ClaimPeriodActive);
            }

            Ok(())
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests;
