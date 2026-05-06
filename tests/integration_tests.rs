use f1r3fly_block_structure::{
    BlockBuilder, BlockMessage, BlockSerialize, BlockStore, Bond, GenesisConfig, InMemoryBlockStore,
};

#[test]
fn test_proto_roundtrip() {
    let block = GenesisConfig {
        shard_id: "root".into(),
        validators: vec![Bond {
            validator: vec![1; 32],
            stake: 100,
        }],
        timestamp: 0,
    }
    .build_genesis_block()
    .expect("genesis");
    let bytes = block.to_proto_bytes().expect("bytes");
    let decoded = BlockMessage::from_proto_bytes(&bytes).expect("decode");
    assert_eq!(block, decoded);
}

#[test]
fn test_builder_hashes_body_and_header() {
    let block = BlockBuilder::new()
        .with_parents(vec![[1u8; 32]])
        .with_post_state_hash([2u8; 32])
        .with_bonds(vec![Bond {
            validator: vec![3u8; 32],
            stake: 10,
        }])
        .with_shard_id("root".into())
        .with_sender(vec![4u8; 32])
        .with_seq_num(1)
        .build_unsigned()
        .expect("build")
        .sign(|_| vec![0u8; 64]);
    assert_eq!(block.header.block_hash, block.block_hash);
    assert_ne!(block.header.body_hash, [0u8; 32]);
}

#[tokio::test]
async fn test_store_get_by_hash() {
    let block = GenesisConfig {
        shard_id: "root".into(),
        validators: vec![Bond {
            validator: vec![1; 32],
            stake: 100,
        }],
        timestamp: 0,
    }
    .build_genesis_block()
    .expect("genesis");
    let store = InMemoryBlockStore::new();
    store.put(&block).await.expect("put");
    assert!(store.contains(&block.block_hash).await.expect("contains"));
    let fetched = store.get_by_hash(&block.block_hash).await.expect("get");
    assert_eq!(Some(block), fetched);
}
