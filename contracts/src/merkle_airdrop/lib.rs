#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::merke_airdrop::*;

#[ink::contract]
mod merke_airdrop {
    use erc20::Erc20Ref;
    use ink::env::hash::{HashOutput, Keccak256};
    use ink::env::hash_bytes;
    use ink::prelude::vec::Vec;
    use ink::{H256, U256};

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
    /// - `leaf` is the 32-byte leaf value (already hashed if your protocol pre-hashes leaves).
    /// - `proof` is the list of sibling nodes from leaf level up to the root (bottom → top).
    /// - `index` is the leaf index (0-based) in the tree; index determines left/right order.
    /// - `root` is the expected 32-byte Merkle root.
    ///
    /// Returns `true` when the computed root equals `root`.
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

    /// Event emitted when a recipient claims airdrop.
    #[ink(event)]
    pub struct Claimed {
        #[ink(topic)]
        recipient: Address,
        value: U256,
    }

    #[ink(storage)]
    pub struct MerkleAirdrop {
        erc20_contract: Erc20Ref,
        root: [u8; 32],
    }

    impl MerkleAirdrop {
        /// Initializes the contract by instantiating the code at the given code hash via
        /// `instantiate` host function with the supplied weight and storage
        /// limits.
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

            Self {
                erc20_contract,
                root,
            }
        }

        /// Initializes the contract by instantiating the code at the given code hash via
        /// the `instantiate` host function with no weight or storage limits.
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

            Self {
                erc20_contract,
                root,
            }
        }

        #[ink(message)]
        pub fn fund(&mut self, amount: U256) {
            let caller = self.env().caller();
            let contract = self.env().address();

            let transferred = self.erc20_contract.transfer_from(caller, contract, amount);

            assert!(transferred.is_ok(), "Transfer failed");
        }

        #[ink(message)]
        pub fn claim(&mut self, value: U256, proof: Vec<[u8; 32]>, index: u64) {
            let recipient = self.env().caller();
            let encoded = (recipient, value);
            let mut leaf = <Keccak256 as HashOutput>::Type::default();
            ink::env::hash_encoded::<Keccak256, _>(&encoded, &mut leaf);
            let verified = verify_proof(leaf, &proof, index, self.root);

            assert!(verified, "Invalid proof");

            let transferred = self.erc20_contract.transfer(recipient, value);

            assert!(transferred.is_ok(), "Transfer failed");

            self.env().emit_event(Claimed { recipient, value });
        }

        /// Get the address of the other contract after it has been instantiated. We can
        /// use this so we can call the other contract on the frontend.
        #[ink(message)]
        pub fn erc20_account_id(&mut self) -> AccountId {
            self.erc20_contract.account_id()
        }

        #[ink(message)]
        pub fn erc20_address(&mut self) -> Address {
            self.erc20_contract.address()
        }
    }
}

#[cfg(all(test, feature = "e2e-tests"))]
mod e2e_tests;
