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
    use assets::{
        asset_hub_precompile::{AssetHubPrecompileRef, Erc20},
        AssetId,
    };
    use ink::env::hash::{HashOutput, Keccak256};
    use ink::env::hash_bytes;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use ink::{H256, U256};

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
        /// Create a new Merkle airdrop contract and fund it.
        ///
        /// Initializes the distribution campaign by deploying the asset contract
        /// reference, setting the Merkle root, configuring the claim window, and
        /// transferring the airdrop tokens into the contract.
        ///
        /// # Arguments
        /// - `asset_id`: asset identifier for the ERC20-compatible token.
        /// - `asset_contract_code_hash`: hash of the asset contract code.
        /// - `root`: Merkle root of the distribution tree.
        /// - `campaign_end_time`: block timestamp when claiming stops.
        /// - `total_airdrop_amount`: amount of tokens to lock in this campaign.
        ///
        /// # Behavior
        /// - The caller becomes the contract owner.
        /// - The contract transfers `total_airdrop_amount` tokens from the caller
        ///   into itself during deployment.
        ///
        /// # Panics
        /// - If `total_airdrop_amount == 0`.
        /// - If the token transfer from caller to contract fails.
        /// - If the provided `campaign_end_time` is already in the past.
        #[ink(constructor, payable)]
        pub fn new(
            asset_id: AssetId,
            asset_contract_code_hash: H256,
            root: [u8; 32],
            campaign_end_time: u64,
            total_airdrop_amount: U256,
        ) -> Self {
            // Fail if campaign already ended or ends immediately
            let now = Self::env().block_timestamp();
            assert!(
                campaign_end_time > now,
                "Campaign end time must be in the future"
            );

            // Fail if trying to fund with 0 tokens
            assert!(
                !total_airdrop_amount.is_zero(),
                "Total airdrop amount cannot be zero"
            );

            let caller = Self::env().caller();
            let contract = Self::env().address();

            // Deploy the asset contract reference
            let mut asset_contract = AssetHubPrecompileRef::new(asset_id)
                .code_hash(asset_contract_code_hash)
                .endowment(0.into())
                .salt_bytes(Some([1u8; 32]))
                .instantiate();

            // Transfer in the total campaign tokens
            let transferred = asset_contract.transferFrom(caller, contract, total_airdrop_amount);
            assert!(transferred.is_ok(), "Funding transfer failed");

            Self {
                asset_contract,
                root,
                claimed: Mapping::new(),
                owner: caller,
                campaign_end_time,
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
            let already_claimed = self.claimed.get(recipient).unwrap_or(false);

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
