use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;

use f1r3fly_block_structure::{
    compute_block_hash, compute_deploy_hash, BlockBuilder, BlockSerialize, BlockValidator,
    GenesisConfig, InMemoryBlockStore, ValidationError, Bond, BlockMessage, DeployData,
    Justification, ProcessedDeploy, Event, BlockStore,
};

fn sample_bond() -> Bond {
    Bond {
        validator: vec![1u8; 32],
        stake: 100,
    }
}

fn sample_deploy() -> DeployData {
    DeployData {
        deployer: vec![9u8; 32],
        term: "new x in { x }".to_string(),
        timestamp: 1_700_000_000_000,
        sig: vec![0u8; 64],
        sig_algorithm: "ed25519".to_string(),
        phlo_price: 1,
        phlo_limit: 10_000,
        valid_after_block_number: 0,
        shard_id: "test-shard".to_string(),
    }
}

fn sample_processed_deploy() -> ProcessedDeploy {
    ProcessedDeploy {
        deploy: sample_deploy(),
        cost: 10,
        deploy_log: vec![Event {
            name: "ok".to_string(),
            payload: vec![1, 2, 3],
        }],
        payments_results: Vec::new(),
        is_failed: false,
    }
}

fn build_block(signing_key: &SigningKey) -> BlockMessage {
    let post_state_hash = f1r3fly_block_structure::compute_post_state_hash(b"state");
    let unsigned = BlockBuilder::new()
        .with_parent([2u8; 32])
        .with_post_state_hash(post_state_hash)
        .with_bonds(vec![sample_bond()])
        .with_deploy(sample_processed_deploy())
        .with_justifications(vec![Justification {
            validator: vec![1u8; 32],
            latest_block_hash: [3u8; 32],
        }])
        .with_shard_id("test-shard".to_string())
        .with_sender(signing_key.verifying_key().to_bytes().to_vec())
        .with_seq_num(1)
        .with_timestamp(1_700_000_000_001)
        .build_unsigned()
        .expect("build unsigned");

    unsigned.sign(|hash| signing_key.sign(hash).to_bytes().to_vec())
}

#[test]
fn test_build_valid_block() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    let expected = compute_block_hash(&block.header);
    assert_eq!(block.block_hash, expected);
}

#[test]
fn test_block_hash_determinism() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    let hash1 = compute_block_hash(&block.header);
    let hash2 = compute_block_hash(&block.header);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_signature_validation() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    BlockValidator::validate_signature(&block).expect("signature valid");
}

#[test]
fn test_invalid_signature_rejected() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut block = build_block(&signing_key);
    block.sig = vec![0u8; 64];
    let err = BlockValidator::validate_signature(&block).unwrap_err();
    assert!(matches!(err, ValidationError::InvalidSignature));
}

#[test]
fn test_genesis_block_has_no_parents() {
    let config = GenesisConfig {
        shard_id: "test-shard".to_string(),
        validators: vec![sample_bond()],
        wallets: Vec::new(),
        timestamp: 0,
    };
    let genesis = config.build_genesis_block().expect("genesis");
    assert!(genesis.header.parents_hash_list.is_empty());
}

#[test]
fn test_roundtrip_protobuf_serialization() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    let bytes = block.to_proto_bytes().expect("to proto");
    let decoded = BlockMessage::from_proto_bytes(&bytes).expect("from proto");
    assert_eq!(block, decoded);
}

#[test]
fn test_roundtrip_json_serialization() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    let json = block.to_json().expect("to json");
    let decoded = BlockMessage::from_json(&json).expect("from json");
    assert_eq!(block, decoded);
}

#[tokio::test]
async fn test_in_memory_store_put_get() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key);
    let store = InMemoryBlockStore::new();
    store.put(&block).await.expect("put");
    let fetched = store.get(&block.block_hash).await.expect("get");
    assert_eq!(Some(block), fetched);
}

#[test]
fn test_block_builder_missing_field_errors() {
    let err = BlockBuilder::new().build_unsigned().unwrap_err();
    assert!(matches!(err, f1r3fly_block_structure::BlockBuildError::MissingParents));
}

#[test]
fn test_deploy_hash_uniqueness() {
    let mut deploy1 = sample_deploy();
    let mut deploy2 = sample_deploy();
    deploy2.term = "new y in { y }".to_string();
    let hash1 = compute_deploy_hash(&deploy1);
    let hash2 = compute_deploy_hash(&deploy2);
    assert_ne!(hash1, hash2);
    deploy1.term = "new z in { z }".to_string();
    let hash3 = compute_deploy_hash(&deploy1);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_genesis_from_bonds_wallets_file() {
    let config = GenesisConfig::from_files("tests/fixtures/bonds.txt", "tests/fixtures/wallets.txt")
        .expect("config");
    assert_eq!(config.validators.len(), 2);
    assert_eq!(config.wallets.len(), 2);
}
