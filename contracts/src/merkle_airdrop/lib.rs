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
/// - Funded by transferring ERC20-compatible tokens into the contract.
///
/// ## Storage
/// - `asset_contract`: reference to an ERC20-compatible token contract.
/// - `root`: Merkle root committing to `(address, amount)` pairs.
/// - `claimed`: mapping to track which addresses have claimed.
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
    ///
    /// # Arguments
    /// - `left`: left-hand side bytes.
    /// - `right`: right-hand side bytes.
    ///
    /// # Returns
    /// - `[u8; 32]`: Keccak256 hash of the concatenated input.
    fn hash(left: &[u8], right: &[u8]) -> [u8; 32] {
        let mut input = Vec::with_capacity(left.len() + right.len());
        input.extend_from_slice(left);
        input.extend_from_slice(right);
        let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
        hash_bytes::<Keccak256>(&input, &mut output);

        output
    }

    /// Verify that a leaf is part of a Merkle tree with the given root.
    ///
    /// # Arguments
    /// - `leaf`: 32-byte leaf value (`keccak256(address || amount)`).
    /// - `proof`: list of sibling nodes from leaf up to the root (bottom → top).
    /// - `index`: leaf index (0-based) in the tree, determines left/right order.
    /// - `root`: expected Merkle root.
    ///
    /// # Returns
    /// - `bool`: true if proof is valid and recomputed root equals `root`.
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

    /// Errors that can occur when funding or claiming.
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
    }

    /// Standard `Result` type for contract operations.
    pub type Result<T> = core::result::Result<T, Error>;

    /// Merkle-based ERC20 token airdrop contract.
    ///
    /// ## Storage
    /// - `asset_contract`: reference to the token precompile.
    /// - `root`: Merkle root of the distribution tree.
    /// - `claimed`: mapping of address → bool to track claimed status.
    #[ink(storage)]
    pub struct MerkleAirdrop {
        pub asset_contract: AssetHubPrecompileRef,
        pub root: [u8; 32],
        pub claimed: Mapping<Address, bool>,
    }

    impl MerkleAirdrop {
        /// Create a new Merkle airdrop contract.
        ///
        /// # Arguments
        /// - `asset_id`: asset identifier for the ERC20-compatible token.
        /// - `root`: Merkle root of the distribution tree.
        #[ink(constructor, payable)]
        pub fn new(asset_id: AssetId, asset_contract_code_hash: H256, root: [u8; 32]) -> Self {
            let asset_contract = AssetHubPrecompileRef::new(asset_id)
                .code_hash(asset_contract_code_hash)
                .endowment(0.into())
                .salt_bytes(Some([1u8; 32]))
                .instantiate();
            let claimed = Mapping::new();

            Self {
                asset_contract,
                root,
                claimed,
            }
        }

        /// Fund the airdrop contract by transferring tokens in.
        ///
        /// # Arguments
        /// - `amount`: amount of tokens to transfer into the contract.
        ///
        /// # Errors
        /// - [`Error::AmountCannotBeZero`]: if `amount == 0`.
        /// - [`Error::TransferFailed`]: if ERC20 transfer fails.
        #[ink(message)]
        pub fn fund(&mut self, amount: U256) -> Result<()> {
            let caller = self.env().caller();
            let contract = self.env().address();

            if amount == U256::zero() {
                return Err(Error::AmountCannotBeZero);
            }

            let transferred = self.asset_contract.transferFrom(caller, contract, amount);

            if transferred.is_err() {
                return Err(Error::TransferFailed);
            }

            Ok(())
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
        /// - [`Error::TransferFailed`]: if ERC20 transfer fails.
        #[ink(message)]
        pub fn claim(&mut self, value: U256, proof: Vec<[u8; 32]>, index: u64) -> Result<()> {
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

        /// Get the token asset id of the ERC20 contract.
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
        ///
        /// # Arguments
        /// - `recipient`: address to check.
        ///
        /// # Returns
        /// - `bool`: true if recipient already claimed, false otherwise.
        #[ink(message)]
        pub fn is_claimed(&self, recipient: Address) -> bool {
            self.claimed.get(recipient).unwrap_or(false)
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests;
