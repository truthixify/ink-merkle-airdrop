use super::*;
use assets::asset_hub_precompile::{AssetHubPrecompile, AssetHubPrecompileRef, Erc20};
use assets::AssetId;
use ink::env::hash::{HashOutput, Keccak256};
use ink::env::hash_bytes;
use ink::prelude::vec::Vec;
use ink::Address;
use ink::U256;
use ink_e2e::ContractsBackend;

type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Helper function to replicate the contract's hashing logic in the test environment.
// This is crucial for creating the leaves and root correctly.
fn hash_leaf(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(left.len() + right.len());
    input.extend_from_slice(left);
    input.extend_from_slice(right);
    let mut output = <Keccak256 as HashOutput>::Type::default();
    hash_bytes::<Keccak256>(&input, &mut output);
    output
}

#[derive(Debug)]
struct Setup {
    pub alice_account: Address,
    pub bob_account: Address,
    pub airdrop_amount_alice: U256,
    pub airdrop_amount_bob: U256,
    pub leaf_alice: [u8; 32],
    pub leaf_bob: [u8; 32],
    pub total_supply: U256,
    pub proof_for_alice: Vec<[u8; 32]>,
    pub proof_for_bob: Vec<[u8; 32]>,
    pub index_alice: u64,
    pub index_bob: u64,
    pub root: [u8; 32],
    pub creator: Address,
    pub asset_id: AssetId,
    pub campaign_end_time: u64,
}

impl Setup {
    fn new() -> Self {
        let bob_account =
            ink_e2e::address::<ink::env::DefaultEnvironment>(ink_e2e::Sr25519Keyring::Bob);
        let airdrop_amount_bob = U256::from(500_000_000);
        let alice_account =
            ink_e2e::address::<ink::env::DefaultEnvironment>(ink_e2e::Sr25519Keyring::Alice);
        let airdrop_amount_alice = U256::from(100_000_000);

        // Create leaves by hashing account and value, just like the contract does.
        let leaf_alice = hash_leaf(
            alice_account.as_bytes(),
            &airdrop_amount_alice.to_big_endian(),
        );

        let leaf_bob = hash_leaf(bob_account.as_bytes(), &airdrop_amount_bob.to_big_endian());

        // Our tree has two leaves. The root is the hash of both leaves.
        let root = hash_leaf(&leaf_alice, &leaf_bob);

        // To claim, Bob needs to provide the sibling leaf (Alice's) as proof.
        let proof_for_bob = vec![leaf_alice];
        let index_bob = 1; // Bob is the second leaf (0-indexed).
        let proof_for_alice = vec![leaf_bob];
        let index_alice = 0; // Bob is the second leaf (0-indexed).
        let total_supply = U256::from(1_000_000_000);
        let creator =
            ink_e2e::address::<ink::env::DefaultEnvironment>(ink_e2e::Sr25519Keyring::Charlie);
        let asset_id = 1;
        let campaign_end_time = 3;

        Self {
            alice_account,
            bob_account,
            airdrop_amount_alice,
            airdrop_amount_bob,
            leaf_alice,
            leaf_bob,
            total_supply,
            proof_for_alice,
            proof_for_bob,
            index_alice,
            index_bob,
            root,
            creator,
            asset_id,
            campaign_end_time,
        }
    }
}

#[ink_e2e::test]
async fn instantiate<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
    // given
    let assets_contract_code = client
        .upload("assets", &ink_e2e::charlie())
        .submit()
        .await
        .expect("assets upload failed");

    let setup = Setup::new();

    let mut constructor = AssetHubPrecompileRef::new(setup.asset_id);
    let asset_hub_contract = client
        .instantiate(
            "asset_hub_precompile",
            &ink_e2e::charlie(),
            &mut constructor,
        )
        .submit()
        .await
        .expect("failed");

    // Approve tokens BEFORE contract instantiation, so constructor can pull them in
    let mut assets_call_builder = asset_hub_contract.call_builder::<AssetHubPrecompile>();
    let approve_call = assets_call_builder.approve(setup.creator, setup.total_supply);
    let approve_result = client
        .call(&ink_e2e::charlie(), &approve_call)
        .submit()
        .await
        .expect("Calling `approve` failed")
        .return_value();
    assert!(approve_result.is_ok(), "Approve failed");

    // when
    // let mut constructor = MerkleAirdropRef::new(
    //     setup.asset_id,
    //     assets_contract_code.code_hash,
    //     setup.root,
    //     setup.campaign_end_time,
    //     setup.total_supply,
    // );
    // let contract = client
    //     .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
    //     .submit()
    //     .await;

    // // then
    // assert!(contract.is_ok(), "{}", contract.err().unwrap());

    Ok(())
}

// #[ink_e2e::test]
// async fn fund<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
//     // given
//     let assets_contract_code = client
//         .upload("assets", &ink_e2e::charlie())
//         .submit()
//         .await
//         .expect("assets upload failed");

//     let setup = Setup::new();
//     let mut constructor = MerkleAirdropRef::new(
//         setup.asset_id,
//         assets_contract_code.code_hash,
//         setup.root,
//         setup.campaign_end_time,
//         setup.total_supply,
//     );
//     let contract = client
//         .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
//         .submit()
//         .await
//         .expect("merkle_airdrop instantiate failed");
//     let mut call_builder = contract.call_builder::<MerkleAirdrop>();

//     let call = call_builder.asset_id();
//     let asset_id = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `asset_id` failed")
//         .return_value();

//     let mut assets_call_builder = ink_e2e::create_call_builder::<AssetHubPrecompile>(asset_id);
//     let creator_balance_call = assets_call_builder.balance_of(setup.creator);
//     let creator_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_call = assets_call_builder.balance_of(contract.addr);
//     let contract_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(creator_balance_before_fund, setup.total_supply);
//     assert_eq!(contract_balance_before_fund, U256::zero());

//     let approve_call = assets_call_builder.approve(contract.addr, setup.total_supply);
//     let approve_result = client
//         .call(&ink_e2e::charlie(), &approve_call)
//         .submit()
//         .await
//         .expect("Calling `approve` failed")
//         .return_value();
//     assert!(approve_result.is_ok(), "Approve failed");

//     // when
//     let call = call_builder.fund(setup.total_supply);
//     let result = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();
//     assert!(result.is_ok(), "Fund failed");
//     // then
//     let creator_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(
//         creator_balance_after_fund,
//         U256::zero(),
//         "Creator balance should be zero after funding"
//     );
//     assert_eq!(
//         contract_balance_after_fund, setup.total_supply,
//         "Contract balance should equal total supply after funding"
//     );

//     Ok(())
// }

// #[ink_e2e::test]
// async fn bob_claim<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
//     // given
//     let assets_contract_code = client
//         .upload("assets", &ink_e2e::charlie())
//         .submit()
//         .await
//         .expect("assets upload failed");

//     let setup = Setup::new();
//     let mut constructor = MerkleAirdropRef::new(
//         setup.asset_id,
//         assets_contract_code.code_hash,
//         setup.root,
//         setup.campaign_end_time,
//         setup.total_supply,
//     );
//     let contract = client
//         .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
//         .submit()
//         .await
//         .expect("merkle_airdrop instantiate failed");
//     let mut call_builder = contract.call_builder::<MerkleAirdrop>();

//     let call = call_builder.asset_id();
//     let asset_id = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `asset_id` failed")
//         .return_value();

//     let mut assets_call_builder = ink_e2e::create_call_builder::<AssetHubPrecompile>(asset_id);
//     let creator_balance_call = assets_call_builder.balance_of(setup.creator);
//     let creator_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_call = assets_call_builder.balance_of(contract.addr);
//     let contract_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(creator_balance_before_fund, setup.total_supply);
//     assert_eq!(contract_balance_before_fund, U256::zero());

//     let approve_call = assets_call_builder.approve(contract.addr, setup.total_supply);
//     let approve_result = client
//         .call(&ink_e2e::charlie(), &approve_call)
//         .submit()
//         .await
//         .expect("Calling `approve` failed")
//         .return_value();
//     assert!(approve_result.is_ok(), "Approve failed");

//     // when
//     let call = call_builder.fund(setup.total_supply);
//     let result = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();
//     assert!(result.is_ok(), "Fund failed");
//     // then
//     let creator_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(
//         creator_balance_after_fund,
//         U256::zero(),
//         "Creator balance should be zero after funding"
//     );
//     assert_eq!(
//         contract_balance_after_fund, setup.total_supply,
//         "Contract balance should equal total supply after funding"
//     );

//     let bob_balance_call = assets_call_builder.balance_of(setup.bob_account);
//     let bob_balance_before_claim = client
//         .call(&ink_e2e::bob(), &bob_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(bob_balance_before_claim, U256::zero());
//     let call = call_builder.claim(
//         setup.airdrop_amount_bob,
//         setup.proof_for_bob.clone(),
//         setup.index_bob,
//     );
//     let result = client
//         .call(&ink_e2e::bob(), &call)
//         .submit()
//         .await
//         .expect("Calling `claim` failed")
//         .return_value();
//     assert!(result.is_ok(), "Claim failed");
//     let bob_balance_after_claim = client
//         .call(&ink_e2e::bob(), &bob_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         bob_balance_after_claim, setup.airdrop_amount_bob,
//         "Bob's balance should equal his airdrop amount after claiming"
//     );
//     let contract_balance_after_claim = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         contract_balance_after_claim,
//         setup.total_supply - setup.airdrop_amount_bob,
//         "Contract balance should decrease by Bob's airdrop amount after he claims"
//     );

//     Ok(())
// }

// #[ink_e2e::test]
// async fn alice_claim<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
//     // given
//     let assets_contract_code = client
//         .upload("assets", &ink_e2e::charlie())
//         .submit()
//         .await
//         .expect("assets upload failed");

//     let setup = Setup::new();
//     let mut constructor = MerkleAirdropRef::new(
//         setup.asset_id,
//         assets_contract_code.code_hash,
//         setup.root,
//         setup.campaign_end_time,
//         setup.total_supply,
//     );
//     let contract = client
//         .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
//         .submit()
//         .await
//         .expect("merkle_airdrop instantiate failed");
//     let mut call_builder = contract.call_builder::<MerkleAirdrop>();

//     let call = call_builder.asset_id();
//     let asset_id = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `asset_id` failed")
//         .return_value();

//     let mut assets_call_builder = ink_e2e::create_call_builder::<AssetHubPrecompile>(asset_id);
//     let creator_balance_call = assets_call_builder.balance_of(setup.creator);
//     let creator_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_call = assets_call_builder.balance_of(contract.addr);
//     let contract_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(creator_balance_before_fund, setup.total_supply);
//     assert_eq!(contract_balance_before_fund, U256::zero());

//     let approve_call = assets_call_builder.approve(contract.addr, setup.total_supply);
//     let approve_result = client
//         .call(&ink_e2e::charlie(), &approve_call)
//         .submit()
//         .await
//         .expect("Calling `approve` failed")
//         .return_value();
//     assert!(approve_result.is_ok(), "Approve failed");

//     // when
//     let call = call_builder.fund(setup.total_supply);
//     let result = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();
//     assert!(result.is_ok(), "Fund failed");
//     // then
//     let creator_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         creator_balance_after_fund,
//         U256::zero(),
//         "Creator balance should be zero after funding"
//     );
//     assert_eq!(
//         contract_balance_after_fund, setup.total_supply,
//         "Contract balance should equal total supply after funding"
//     );
//     let alice_balance_call = assets_call_builder.balance_of(setup.alice_account);
//     let alice_balance_before_claim = client
//         .call(&ink_e2e::alice(), &alice_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(alice_balance_before_claim, U256::zero());
//     let call = call_builder.claim(
//         setup.airdrop_amount_alice,
//         setup.proof_for_alice.clone(),
//         setup.index_alice,
//     );
//     let result = client
//         .call(&ink_e2e::alice(), &call)
//         .submit()
//         .await
//         .expect("Calling `claim` failed")
//         .return_value();
//     assert!(result.is_ok(), "Claim failed");
//     let alice_balance_after_claim = client
//         .call(&ink_e2e::alice(), &alice_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         alice_balance_after_claim, setup.airdrop_amount_alice,
//         "Alice's balance should equal her airdrop amount after claiming"
//     );
//     let contract_balance_after_claim = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         contract_balance_after_claim,
//         setup.total_supply - setup.airdrop_amount_alice,
//         "Contract balance should decrease by Alice's airdrop amount after she claims"
//     );

//     Ok(())
// }

// #[ink_e2e::test]
// async fn bob_and_alice_claim<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
//     // given
//     let assets_contract_code = client
//         .upload("assets", &ink_e2e::charlie())
//         .submit()
//         .await
//         .expect("assets upload failed");

//     let setup = Setup::new();
//     let mut constructor = MerkleAirdropRef::new(
//         setup.asset_id,
//         assets_contract_code.code_hash,
//         setup.root,
//         setup.campaign_end_time,
//         setup.total_supply,
//     );
//     let contract = client
//         .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
//         .submit()
//         .await
//         .expect("merkle_airdrop instantiate failed");
//     let mut call_builder = contract.call_builder::<MerkleAirdrop>();

//     let call = call_builder.asset_id();
//     let asset_id = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `asset_id` failed")
//         .return_value();

//     let mut assets_call_builder = ink_e2e::create_call_builder::<AssetHubPrecompile>(asset_id);
//     let creator_balance_call = assets_call_builder.balance_of(setup.creator);
//     let creator_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_call = assets_call_builder.balance_of(contract.addr);
//     let contract_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(creator_balance_before_fund, setup.total_supply);
//     assert_eq!(contract_balance_before_fund, U256::zero());

//     let approve_call = assets_call_builder.approve(contract.addr, setup.total_supply);
//     let approve_result = client
//         .call(&ink_e2e::charlie(), &approve_call)
//         .submit()
//         .await
//         .expect("Calling `approve` failed")
//         .return_value();
//     assert!(approve_result.is_ok(), "Approve failed");

//     // when
//     let call = call_builder.fund(setup.total_supply);
//     let result = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();
//     assert!(result.is_ok(), "Fund failed");
//     // then
//     let creator_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_after_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         creator_balance_after_fund,
//         U256::zero(),
//         "Creator balance should be zero after funding"
//     );
//     assert_eq!(
//         contract_balance_after_fund, setup.total_supply,
//         "Contract balance should equal total supply after funding"
//     );
//     let bob_balance_call = assets_call_builder.balance_of(setup.bob_account);
//     let bob_balance_before_claim = client
//         .call(&ink_e2e::bob(), &bob_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(bob_balance_before_claim, U256::zero());
//     let call = call_builder.claim(
//         setup.airdrop_amount_bob,
//         setup.proof_for_bob.clone(),
//         setup.index_bob,
//     );
//     let result = client
//         .call(&ink_e2e::bob(), &call)
//         .submit()
//         .await
//         .expect("Calling `claim` failed")
//         .return_value();
//     assert!(result.is_ok(), "Claim failed");
//     let bob_balance_after_claim = client
//         .call(&ink_e2e::bob(), &bob_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         bob_balance_after_claim, setup.airdrop_amount_bob,
//         "Bob's balance should equal his airdrop amount after claiming"
//     );
//     let contract_balance_after_bob_claim = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         contract_balance_after_bob_claim,
//         setup.total_supply - setup.airdrop_amount_bob,
//         "Contract balance should decrease by Bob's airdrop amount after he claims"
//     );
//     let alice_balance_call = assets_call_builder.balance_of(setup.alice_account);
//     let alice_balance_before_claim = client
//         .call(&ink_e2e::alice(), &alice_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(alice_balance_before_claim, U256::zero());
//     let call = call_builder.claim(
//         setup.airdrop_amount_alice,
//         setup.proof_for_alice.clone(),
//         setup.index_alice,
//     );
//     let result = client
//         .call(&ink_e2e::alice(), &call)
//         .submit()
//         .await
//         .expect("Calling `claim` failed")
//         .return_value();
//     assert!(result.is_ok(), "Claim failed");
//     let alice_balance_after_claim = client
//         .call(&ink_e2e::alice(), &alice_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         alice_balance_after_claim, setup.airdrop_amount_alice,
//         "Alice's balance should equal her airdrop amount after claiming"
//     );
//     let contract_balance_after_alice_claim = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     assert_eq!(
//         contract_balance_after_alice_claim,
//         setup.total_supply - setup.airdrop_amount_bob - setup.airdrop_amount_alice,
//         "Contract balance should decrease by Alice's airdrop amount after she claims"
//     );
//     Ok(())
// }

// #[ink_e2e::test]
// async fn cannot_claim_twice<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
//     // given
//     let assets_contract_code = client
//         .upload("assets", &ink_e2e::charlie())
//         .submit()
//         .await
//         .expect("assets upload failed");

//     let setup = Setup::new();
//     let mut constructor = MerkleAirdropRef::new(
//         setup.asset_id,
//         assets_contract_code.code_hash,
//         setup.root,
//         setup.campaign_end_time,
//         setup.total_supply,
//     );
//     let contract = client
//         .instantiate("merkle_airdrop", &ink_e2e::charlie(), &mut constructor)
//         .submit()
//         .await
//         .expect("merkle_airdrop instantiate failed");
//     let mut call_builder = contract.call_builder::<MerkleAirdrop>();

//     let call = call_builder.asset_id();
//     let asset_id = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();

//     let mut assets_call_builder = ink_e2e::create_call_builder::<AssetHubPrecompile>(asset_id);
//     let creator_balance_call = assets_call_builder.balance_of(setup.creator);
//     let creator_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &creator_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();
//     let contract_balance_call = assets_call_builder.balance_of(contract.addr);
//     let contract_balance_before_fund = client
//         .call(&ink_e2e::charlie(), &contract_balance_call)
//         .submit()
//         .await
//         .expect("Calling `balance_of` failed")
//         .return_value();

//     assert_eq!(creator_balance_before_fund, setup.total_supply);
//     assert_eq!(contract_balance_before_fund, U256::zero());

//     let approve_call = assets_call_builder.approve(contract.addr, setup.total_supply);
//     let approve_result = client
//         .call(&ink_e2e::charlie(), &approve_call)
//         .submit()
//         .await
//         .expect("Calling `approve` failed")
//         .return_value();
//     assert!(approve_result.is_ok(), "Approve failed");

//     // when
//     let call = call_builder.fund(setup.total_supply);
//     let result = client
//         .call(&ink_e2e::charlie(), &call)
//         .submit()
//         .await
//         .expect("Calling `fund` failed")
//         .return_value();
//     assert!(result.is_ok(), "Fund failed");
//     // then
//     let call = call_builder.claim(
//         setup.airdrop_amount_bob,
//         setup.proof_for_bob.clone(),
//         setup.index_bob,
//     );
//     let result = client
//         .call(&ink_e2e::bob(), &call)
//         .submit()
//         .await
//         .expect("Calling `claim` failed")
//         .return_value();
//     assert!(result.is_ok(), "Claim failed");

//     let result = client.call(&ink_e2e::bob(), &call).dry_run().await?;
//     assert!(result.is_err(), "Calling claim again should fail");

//     Ok(())
// }
