use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;

use f1r3fly_block_structure::{
    compute_block_hash, compute_bonds_map_hash, compute_deploy_hash, BlockBuilder, BlockSerialize,
    BlockStore, BlockValidator, Bond, BlockMessage, DeployData, Event, GenesisConfig,
    GhostForkChoice, InMemoryBlockStore, Justification, PCost, ProduceEvent, ProcessedDeploy,
    ValidationError,
};

fn sample_bond() -> Bond {
    Bond {
        validator: vec![1u8; 32],
        stake: 100,
    }
}

fn sample_deploy(shard_id: &str) -> DeployData {
    DeployData {
        deployer: vec![9u8; 32],
        term: "new x in { x }".to_string(),
        timestamp: 1_700_000_000_000,
        sig: vec![0u8; 64],
        sig_algorithm: "ed25519".to_string(),
        phlo_price: 1,
        phlo_limit: 10_000,
        valid_after_block_number: 0,
        shard_id: shard_id.to_string(),
    }
}

fn sample_event() -> Event {
    Event::Produce(ProduceEvent {
        channel_hash: vec![1u8; 32],
        data: vec![1, 2, 3],
        persistent: false,
    })
}

fn sample_processed_deploy(shard_id: &str) -> ProcessedDeploy {
    ProcessedDeploy {
        deploy: sample_deploy(shard_id),
        cost: PCost { cost: 10 },
        deploy_log: vec![sample_event()],
        payments_results: Vec::new(),
        is_failed: false,
    }
}

fn build_block(signing_key: &SigningKey, shard_id: &str, deploy: ProcessedDeploy) -> BlockMessage {
    let post_state_hash = f1r3fly_block_structure::compute_post_state_hash(b"state");
    let unsigned = BlockBuilder::new()
        .with_parent([2u8; 32])
        .with_post_state_hash(post_state_hash)
        .with_bonds(vec![sample_bond()])
        .with_deploy(deploy)
        .with_justifications(vec![Justification {
            validator: vec![1u8; 32],
            latest_block_hash: [3u8; 32],
        }])
        .with_shard_id(shard_id.to_string())
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
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let expected = compute_block_hash(&block.header);
    assert_eq!(block.block_hash, expected);
}

#[test]
fn test_block_hash_determinism() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let hash1 = compute_block_hash(&block.header);
    let hash2 = compute_block_hash(&block.header);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_signature_validation() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    BlockValidator::validate_signature(&block).expect("signature valid");
}

#[test]
fn test_invalid_signature_rejected() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    block.sig[0] ^= 0xFF;
    let err = BlockValidator::validate_signature(&block).unwrap_err();
    assert!(matches!(err, ValidationError::InvalidSignature));
}

#[test]
fn test_roundtrip_protobuf_serialization() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let bytes = block.to_proto_bytes().expect("to proto");
    let decoded = BlockMessage::from_proto_bytes(&bytes).expect("from proto");
    assert_eq!(block, decoded);
}

#[test]
fn test_roundtrip_json_serialization() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let json = block.to_json().expect("to json");
    let decoded = BlockMessage::from_json(&json).expect("from json");
    assert_eq!(block, decoded);
}

#[tokio::test]
async fn test_in_memory_store_put_get() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
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
    let mut deploy1 = sample_deploy("test-shard");
    let mut deploy2 = sample_deploy("test-shard");
    deploy2.term = "new y in { y }".to_string();
    let hash1 = compute_deploy_hash(&deploy1);
    let hash2 = compute_deploy_hash(&deploy2);
    assert_ne!(hash1, hash2);
    deploy1.term = "new z in { z }".to_string();
    let hash3 = compute_deploy_hash(&deploy1);
    assert_ne!(hash1, hash3);
}

// --- Validation tests ---

#[test]
fn test_validate_hash_correct() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    BlockValidator::validate_hash(&block).expect("hash valid");
}

#[test]
fn test_validate_hash_tampered() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    block.block_hash[0] ^= 0xAA;
    let err = BlockValidator::validate_hash(&block).unwrap_err();
    assert!(matches!(err, ValidationError::InvalidBlockHash { .. }));
}

#[test]
fn test_validate_signature_correct() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    BlockValidator::validate_signature(&block).expect("signature valid");
}

#[test]
fn test_validate_signature_tampered() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    block.sig[0] ^= 0x11;
    let err = BlockValidator::validate_signature(&block).unwrap_err();
    assert!(matches!(err, ValidationError::InvalidSignature));
}

#[test]
fn test_validate_structure_empty_sender_fails() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    block.sender.clear();
    let err = BlockValidator::validate_structure(&block).unwrap_err();
    assert!(matches!(err, ValidationError::EmptySender));
}

#[test]
fn test_validate_structure_zero_phlo_limit_fails() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mut deploy = sample_deploy("test-shard");
    deploy.phlo_limit = 0;
    let processed = ProcessedDeploy {
        deploy,
        cost: PCost { cost: 10 },
        deploy_log: vec![sample_event()],
        payments_results: Vec::new(),
        is_failed: false,
    };
    let block = build_block(&signing_key, "test-shard", processed);
    let err = BlockValidator::validate_structure(&block).unwrap_err();
    assert!(matches!(err, ValidationError::InvalidPhloLimit(0)));
}

#[test]
fn test_validate_structure_shard_id_mismatch_fails() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let processed = sample_processed_deploy("shard-B");
    let block = build_block(&signing_key, "shard-A", processed);
    let err = BlockValidator::validate_structure(&block).unwrap_err();
    assert!(matches!(err, ValidationError::ShardIdMismatch { .. }));
}

// --- Storage + DAG tests ---

#[tokio::test]
async fn test_store_put_updates_children_index() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let parent = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));

    let mut child = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    child.header.parents_hash_list = vec![parent.block_hash];
    child.block_hash = compute_block_hash(&child.header);

    let store = InMemoryBlockStore::new();
    store.put(&parent).await.expect("put parent");
    store.put(&child).await.expect("put child");

    let children = store.get_children(&parent.block_hash).await.expect("children");
    assert_eq!(children, vec![child.block_hash]);
}

#[tokio::test]
async fn test_genesis_block_stored_and_retrieved() {
    let config = GenesisConfig {
        shard_id: "test-shard".to_string(),
        validators: vec![sample_bond()],
        timestamp: 1_700_000_000_000,
    };
    let genesis = config.build_genesis_block().expect("genesis");
    let store = InMemoryBlockStore::new();
    store.put(&genesis).await.expect("put");
    let fetched = store.get_genesis().await.expect("get");
    assert_eq!(Some(genesis), fetched);
}

#[tokio::test]
async fn test_latest_messages_updated_on_put() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let store = InMemoryBlockStore::new();
    store.put(&block).await.expect("put");

    let latest = store
        .get_latest_message(&block.sender)
        .await
        .expect("latest");
    assert_eq!(latest, Some(block.block_hash));
}

#[tokio::test]
async fn test_latest_message_updates_on_newer_block() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let block1 = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    let mut block2 = build_block(&signing_key, "test-shard", sample_processed_deploy("test-shard"));
    block2.header.seq_num = 2;
    block2.block_hash = compute_block_hash(&block2.header);

    let store = InMemoryBlockStore::new();
    store.put(&block1).await.expect("put");
    store.put(&block2).await.expect("put");

    let latest = store
        .get_latest_message(&block1.sender)
        .await
        .expect("latest");
    assert_eq!(latest, Some(block2.block_hash));
}

// --- Genesis tests ---

#[test]
fn test_genesis_block_has_no_parents() {
    let config = GenesisConfig {
        shard_id: "test-shard".to_string(),
        validators: vec![Bond { validator: vec![1u8; 32], stake: 100 }],
        timestamp: 1_700_000_000_000,
    };
    let genesis = config.build_genesis_block().unwrap();
    assert!(genesis.header.parents_hash_list.is_empty());
    assert_eq!(genesis.header.seq_num, 0);
    assert!(genesis.justifications.is_empty());
}

#[test]
fn test_genesis_from_bonds_str() {
    let content = "0101010101010101010101010101010101010101010101010101010101010101 100\n\
                   0202020202020202020202020202020202020202020202020202020202020202 200";
    let config = GenesisConfig::from_bonds_str("test-shard", content).unwrap();
    assert_eq!(config.validators.len(), 2);
    assert_eq!(config.validators[0].stake, 100);
    assert_eq!(config.validators[1].stake, 200);
}

// --- GHOST fork-choice tests ---

#[tokio::test]
async fn test_ghost_single_block_is_tip() {
    let config = GenesisConfig {
        shard_id: "test-shard".to_string(),
        validators: vec![sample_bond()],
        timestamp: 1_700_000_000_000,
    };
    let genesis = config.build_genesis_block().expect("genesis");

    let store = InMemoryBlockStore::new();
    store.put(&genesis).await.expect("put genesis");

    let tip = GhostForkChoice::find_tip(&store, &config.validators)
        .await
        .expect("tip");
    assert_eq!(tip, Some(genesis.block_hash));
}

#[tokio::test]
async fn test_ghost_prefers_heavier_branch() {
    let validator_a = vec![1u8; 32];
    let validator_b = vec![2u8; 32];
    let bonds = vec![
        Bond {
            validator: validator_a.clone(),
            stake: 100,
        },
        Bond {
            validator: validator_b.clone(),
            stake: 200,
        },
    ];

    let config = GenesisConfig {
        shard_id: "test-shard".to_string(),
        validators: bonds.clone(),
        timestamp: 1_700_000_000_000,
    };
    let genesis = config.build_genesis_block().expect("genesis");

    let signing_key_a = SigningKey::generate(&mut OsRng);
    let signing_key_b = SigningKey::generate(&mut OsRng);

    let mut block_a = build_block(&signing_key_a, "test-shard", sample_processed_deploy("test-shard"));
    block_a.header.parents_hash_list = vec![genesis.block_hash];
    block_a.sender = validator_a.clone();
    block_a.block_hash = compute_block_hash(&block_a.header);

    let mut block_b = build_block(&signing_key_b, "test-shard", sample_processed_deploy("test-shard"));
    block_b.header.parents_hash_list = vec![genesis.block_hash];
    block_b.sender = validator_b.clone();
    block_b.block_hash = compute_block_hash(&block_b.header);

    let store = InMemoryBlockStore::new();
    store.put(&genesis).await.expect("put genesis");
    store.put(&block_a).await.expect("put a");
    store.put(&block_b).await.expect("put b");

    let tip = GhostForkChoice::find_tip(&store, &bonds).await.expect("tip");
    assert_eq!(tip, Some(block_b.block_hash));
}

// --- PCost and Event model tests ---

#[test]
fn test_pcost_serialization() {
    let cost = PCost { cost: 42 };
    let json = serde_json::to_string(&cost).unwrap();
    let back: PCost = serde_json::from_str(&json).unwrap();
    assert_eq!(cost, back);
}

#[test]
fn test_event_produce_roundtrip_proto() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let deploy = ProcessedDeploy {
        deploy: sample_deploy("test-shard"),
        cost: PCost { cost: 10 },
        deploy_log: vec![Event::Produce(ProduceEvent {
            channel_hash: vec![1u8; 32],
            data: vec![0xDE, 0xAD],
            persistent: false,
        })],
        payments_results: Vec::new(),
        is_failed: false,
    };
    let block = build_block(&signing_key, "test-shard", deploy);

    let bytes = block.to_proto_bytes().expect("to proto");
    let decoded = BlockMessage::from_proto_bytes(&bytes).expect("from proto");
    assert_eq!(block.body.deploys[0].deploy_log[0], decoded.body.deploys[0].deploy_log[0]);
}

// --- bonds_map_hash determinism ---

#[test]
fn test_bonds_map_hash_deterministic() {
    let bonds = vec![
        Bond { validator: vec![2u8; 32], stake: 200 },
        Bond { validator: vec![1u8; 32], stake: 100 },
    ];
    let bonds2 = vec![
        Bond { validator: vec![1u8; 32], stake: 100 },
        Bond { validator: vec![2u8; 32], stake: 200 },
    ];
    assert_eq!(compute_bonds_map_hash(&bonds), compute_bonds_map_hash(&bonds2));
}
