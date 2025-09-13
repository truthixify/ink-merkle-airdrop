#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::merke_airdrop::*;

#[ink::contract]
mod merke_airdrop {
    use erc20::Erc20Ref;
    use ink::env::hash::{HashOutput, Keccak256};
    use ink::env::hash_bytes;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use ink::{H256, U256};

    /// Compute `keccak256(left || right)`.
    ///
    /// Arguments:
    /// - left: left-hand side bytes.
    /// - right: right-hand side bytes.
    ///
    /// Returns:
    /// - [u8; 32]: keccak256 hash of concatenated input.
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
    /// Arguments:
    /// - leaf: 32-byte leaf value (`keccak256(address || amount)`).
    /// - proof: list of sibling nodes from leaf up to the root (bottom → top).
    /// - index: leaf index (0-based) in the tree, determines left/right order.
    /// - root: expected Merkle root.
    ///
    /// Returns:
    /// - bool: true if proof is valid and recomputed root equals `root`.
    fn verify_proof<'a>(leaf: [u8; 32], proof: &'a [[u8; 32]], index: u64, root: [u8; 32]) -> bool {
        let mut computed = leaf;
        let mut index = index;

        for sibling in proof.iter() {
            if index % 2 == 0 {
                // current node is a left child
                computed = hash(&computed, sibling);
            } else {
                // current node is a right child
                computed = hash(sibling, &computed);
            }
            index /= 2;
        }

        computed == root
    }

    /// Event emitted when a recipient successfully claims their airdrop.
    #[ink(event)]
    pub struct Claimed {
        #[ink(topic)]
        recipient: Address,
        value: U256,
    }

    /// Errors that can occur when funding or claiming.
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        TransferFailed,
        InvalidProof,
        AlreadyClaimed,
        AmountCannotBeZero,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    /// Merkle-based ERC20 token airdrop contract.
    ///
    /// Storage:
    /// - erc20_contract: reference to the deployed ERC20 contract.
    /// - root: Merkle root representing eligible recipients and amounts.
    /// - claimed: mapping of address → bool to track claimed status.
    #[ink(storage)]
    pub struct MerkleAirdrop {
        pub erc20_contract: Erc20Ref,
        pub root: [u8; 32],
        pub claimed: Mapping<Address, bool>,
    }

    impl MerkleAirdrop {
        /// Constructor: deploys a new ERC20 contract with resource limits.
        ///
        /// Arguments:
        /// - erc20_contract_code_hash: code hash of ERC20 contract.
        /// - ref_time_limit: gas ref time limit for instantiation.
        /// - proof_size_limit: proof size limit.
        /// - storage_deposit_limit: storage deposit limit.
        /// - root: Merkle root of eligible claims.
        /// - total_supply: total supply of the ERC20 token.
        ///
        /// Returns:
        /// - MerkleAirdrop instance with ERC20 deployed and root set.
        #[ink(constructor)]
        pub fn new_with_limits(
            erc20_contract_code_hash: H256,
            ref_time_limit: u64,
            proof_size_limit: u64,
            storage_deposit_limit: U256,
            root: [u8; 32],
            total_supply: U256,
        ) -> Self {
            let caller = Self::env().caller();
            let erc20_contract = Erc20Ref::new_with_recipient(total_supply, caller)
                .code_hash(erc20_contract_code_hash)
                .endowment(0.into())
                .salt_bytes(Some([1u8; 32]))
                .ref_time_limit(ref_time_limit)
                .proof_size_limit(proof_size_limit)
                .storage_deposit_limit(storage_deposit_limit)
                .instantiate();
            let claimed = Mapping::new();

            Self {
                erc20_contract,
                root,
                claimed,
            }
        }

        /// Constructor: deploys a new ERC20 contract without resource limits.
        ///
        /// Arguments:
        /// - erc20_contract_code_hash: code hash of ERC20 contract.
        /// - root: Merkle root of eligible claims.
        /// - total_supply: total supply of the ERC20 token.
        ///
        /// Returns:
        /// - MerkleAirdrop instance with ERC20 deployed and root set.
        #[ink(constructor)]
        pub fn new_no_limits(
            erc20_contract_code_hash: H256,
            root: [u8; 32],
            total_supply: U256,
        ) -> Self {
            let caller = Self::env().caller();
            let erc20_contract = Erc20Ref::new_with_recipient(total_supply, caller)
                .code_hash(erc20_contract_code_hash)
                .endowment(0.into())
                .salt_bytes(Some([1u8; 32]))
                .instantiate();
            let claimed = Mapping::new();

            Self {
                erc20_contract,
                root,
                claimed,
            }
        }

        /// Fund the airdrop contract by transferring tokens in.
        ///
        /// Arguments:
        /// - amount: amount of tokens to transfer into contract.
        ///
        /// Errors:
        /// - `AmountCannotBeZero`: if `amount == 0`.
        /// - `TransferFailed`: if ERC20 transfer_from fails.
        #[ink(message)]
        pub fn fund(&mut self, amount: U256) -> Result<()> {
            let caller = self.env().caller();
            let contract = self.env().address();

            if amount == U256::zero() {
                return Err(Error::AmountCannotBeZero);
            }

            let transferred = self.erc20_contract.transfer_from(caller, contract, amount);

            if transferred.is_err() {
                return Err(Error::TransferFailed);
            }

            Ok(())
        }

        /// Claim tokens from the Merkle airdrop.
        ///
        /// Arguments:
        /// - value: claim amount for recipient.
        /// - proof: Merkle proof for `(recipient, value)`.
        /// - index: leaf index in the Merkle tree.
        ///
        /// Errors:
        /// - `AlreadyClaimed`: if recipient already claimed.
        /// - `InvalidProof`: if Merkle proof does not validate against root.
        /// - `TransferFailed`: if ERC20 transfer fails.
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

            let transferred = self.erc20_contract.transfer(recipient, value);

            if transferred.is_err() {
                return Err(Error::TransferFailed);
            }

            self.env().emit_event(Claimed { recipient, value });

            Ok(())
        }

        /// Get the AccountId of the ERC20 contract.
        ///
        /// Returns:
        /// - AccountId: ERC20 contract account id.
        #[ink(message)]
        pub fn erc20_account_id(&mut self) -> AccountId {
            self.erc20_contract.account_id()
        }

        /// Get the Address of the ERC20 contract.
        ///
        /// Returns:
        /// - Address: ERC20 contract address.
        #[ink(message)]
        pub fn erc20_address(&mut self) -> Address {
            self.erc20_contract.address()
        }

        /// Get the Merkle root.
        ///
        /// Returns:
        /// - [u8; 32]: current Merkle root.
        #[ink(message)]
        pub fn root(&self) -> [u8; 32] {
            self.root
        }

        /// Check if a recipient has already claimed.
        ///
        /// Arguments:
        /// - recipient: address to check.
        ///
        /// Returns:
        /// - bool: true if recipient already claimed, false otherwise.
        #[ink(message)]
        pub fn is_claimed(&self, recipient: Address) -> bool {
            self.claimed.get(recipient).unwrap_or(false)
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests;
