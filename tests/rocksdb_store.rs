#![cfg(feature = "storage-rocksdb")]

use f1r3fly_block_structure::{BlockStore, Bond, GenesisConfig, RocksDbBlockStore};

#[tokio::test]
async fn rocksdb_persists_across_reopen() {
    let dir = std::env::temp_dir().join(format!(
        "f1r3fly-block-structure-rocksdb-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let path = dir.to_string_lossy().to_string();

    let genesis = GenesisConfig {
        shard_id: "root".into(),
        validators: vec![Bond {
            validator: vec![1; 32],
            stake: 100,
        }],
        timestamp: 0,
    }
    .build_genesis_block()
    .expect("genesis");

    {
        let store = RocksDbBlockStore::open(&path).expect("open");
        store.put(&genesis).await.expect("put");
        let got = store.get_genesis().await.expect("get_genesis");
        assert_eq!(got, Some(genesis.clone()));
        let children = store.get_children(&genesis.block_hash).await.expect("children");
        assert!(children.is_empty());
    }

    {
        let store = RocksDbBlockStore::open(&path).expect("reopen");
        let got = store.get_by_hash(&genesis.block_hash).await.expect("get_by_hash");
        assert_eq!(got, Some(genesis.clone()));
        let got_genesis = store.get_genesis().await.expect("get_genesis");
        assert_eq!(got_genesis, Some(genesis));
    }
}

