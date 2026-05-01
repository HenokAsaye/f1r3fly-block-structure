# f1r3fly-block-structure

A Rust crate that defines, builds, validates, serializes, and stores blocks compatible with the F1r3fly blockchain architecture.

## Overview

This crate mirrors the F1r3fly node's protobuf-based models and provides a block builder, hashing utilities, validation routines, serialization (Protobuf + JSON), and a storage trait with an in-memory implementation.

F1r3fly node reference: https://github.com/F1R3FLY-io/f1r3node (rust/dev branch)

## Quick Start

```rust
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;

use f1r3fly_block_structure::{
    BlockBuilder, BlockSerialize, BlockStore, InMemoryBlockStore, Justification, Bond, Event,
    ProcessedDeploy, DeployData,
};

#[tokio::main]
async fn main() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let post_state_hash = f1r3fly_block_structure::compute_post_state_hash(b"state");

    let deploy = DeployData {
        deployer: vec![9u8; 32],
        term: "new x in { x }".to_string(),
        timestamp: 1_700_000_000_000,
        sig: vec![0u8; 64],
        sig_algorithm: "ed25519".to_string(),
        phlo_price: 1,
        phlo_limit: 10000,
        valid_after_block_number: 0,
        shard_id: "test-shard".to_string(),
    };

    let processed = ProcessedDeploy {
        deploy,
        cost: 10,
        deploy_log: vec![Event { name: "ok".to_string(), payload: vec![1, 2, 3] }],
        payments_results: Vec::new(),
        is_failed: false,
    };

    let unsigned = BlockBuilder::new()
        .with_parent([2u8; 32])
        .with_post_state_hash(post_state_hash)
        .with_bonds(vec![Bond { validator: vec![1u8; 32], stake: 100 }])
        .with_deploy(processed)
        .with_justifications(vec![Justification { validator: vec![1u8; 32], latest_block_hash: [3u8; 32] }])
        .with_shard_id("test-shard".to_string())
        .with_sender(signing_key.verifying_key().to_bytes().to_vec())
        .with_seq_num(1)
        .build_unsigned()
        .expect("build unsigned");

    let block = unsigned.sign(|hash| signing_key.sign(hash).to_bytes().to_vec());

    let store = InMemoryBlockStore::new();
    store.put(&block).await.unwrap();

    let bytes = block.to_proto_bytes().unwrap();
    let roundtrip = f1r3fly_block_structure::BlockMessage::from_proto_bytes(&bytes).unwrap();
    assert_eq!(block, roundtrip);
}
```

## Architecture Fit

- Protobuf models mirror the node's `models/` directory.
- Hashing uses Blake2b-256 (same as the F1r3fly crypto crate).
- Ed25519 signatures match validator signing.
- Block DAG supports multiple parents for Casper consensus.
- Storage trait aligns with the `block-storage` crate pattern.

## Related Crates

- `f1r3fly-models` provides protobuf data models.
- `f1r3fly-crypto` provides hashing and signature utilities.
- `f1r3fly-rholang` executes Rholang smart contracts.
