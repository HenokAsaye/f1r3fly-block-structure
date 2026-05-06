
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;

#[cfg(feature = "storage-rocksdb")]
use crate::serialization::BlockSerialize;
use crate::types::{BlockHash, BlockMessage};
use crate::validation::BlockLookup;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Store error: {0}")]
    Store(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DagRepresentation {
    pub children: HashMap<BlockHash, Vec<BlockHash>>,
}

#[async_trait]
pub trait BlockStore: Send + Sync {
    async fn put(&self, block: &BlockMessage) -> Result<(), StoreError>;
    async fn get_by_hash(&self, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError>;
    async fn contains(&self, hash: &BlockHash) -> Result<bool, StoreError>;
    async fn get_children(&self, hash: &BlockHash) -> Result<Vec<BlockHash>, StoreError>;
    async fn get_genesis(&self) -> Result<Option<BlockMessage>, StoreError>;
    async fn get_dag_representation(&self) -> Result<DagRepresentation, StoreError>;
    async fn delete(&self, hash: &BlockHash) -> Result<(), StoreError>;
    async fn height(&self) -> Result<u64, StoreError>;

    async fn update_latest_message(
        &self,
        validator: &[u8],
        block_hash: BlockHash,
    ) -> Result<(), StoreError>;
    async fn get_latest_message(&self, validator: &[u8]) -> Result<Option<BlockHash>, StoreError>;
    async fn get_all_latest_messages(&self) -> Result<Vec<(Vec<u8>, BlockHash)>, StoreError>;
}

pub struct InMemoryBlockStore {
    blocks: Arc<RwLock<HashMap<BlockHash, BlockMessage>>>,
    children: Arc<RwLock<HashMap<BlockHash, Vec<BlockHash>>>>,
    genesis: Arc<RwLock<Option<BlockHash>>>,
    latest_messages: Arc<RwLock<HashMap<Vec<u8>, BlockHash>>>,
}

impl InMemoryBlockStore {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            children: Arc::new(RwLock::new(HashMap::new())),
            genesis: Arc::new(RwLock::new(None)),
            latest_messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryBlockStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockStore for InMemoryBlockStore {
    async fn put(&self, block: &BlockMessage) -> Result<(), StoreError> {
        let mut blocks = self.blocks.write().await;
        blocks.insert(block.block_hash, block.clone());

        if block.header.parents.is_empty() {
            let mut genesis = self.genesis.write().await;
            *genesis = Some(block.block_hash);
        }

        let mut children = self.children.write().await;
        for parent in &block.header.parents {
            children.entry(*parent).or_default().push(block.block_hash);
        }

        if !block.sender.is_empty() {
            let mut latest = self.latest_messages.write().await;
            latest.insert(block.sender.clone(), block.block_hash);
        }
        Ok(())
    }

    async fn get_by_hash(&self, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError> {
        let blocks = self.blocks.read().await;
        Ok(blocks.get(hash).cloned())
    }

    async fn contains(&self, hash: &BlockHash) -> Result<bool, StoreError> {
        let blocks = self.blocks.read().await;
        Ok(blocks.contains_key(hash))
    }

    async fn get_children(&self, hash: &BlockHash) -> Result<Vec<BlockHash>, StoreError> {
        let children = self.children.read().await;
        Ok(children.get(hash).cloned().unwrap_or_default())
    }

    async fn get_genesis(&self) -> Result<Option<BlockMessage>, StoreError> {
        let genesis = self.genesis.read().await;
        match *genesis {
            Some(hash) => self.get_by_hash(&hash).await,
            None => Ok(None),
        }
    }

    async fn get_dag_representation(&self) -> Result<DagRepresentation, StoreError> {
        Ok(DagRepresentation {
            children: self.children.read().await.clone(),
        })
    }

    async fn delete(&self, hash: &BlockHash) -> Result<(), StoreError> {
        let mut blocks = self.blocks.write().await;
        blocks.remove(hash);
        Ok(())
    }

    async fn height(&self) -> Result<u64, StoreError> {
        let blocks = self.blocks.read().await;
        Ok(blocks.len() as u64)
    }

    async fn update_latest_message(
        &self,
        validator: &[u8],
        block_hash: BlockHash,
    ) -> Result<(), StoreError> {
        let mut latest = self.latest_messages.write().await;
        latest.insert(validator.to_vec(), block_hash);
        Ok(())
    }

    async fn get_latest_message(&self, validator: &[u8]) -> Result<Option<BlockHash>, StoreError> {
        let latest = self.latest_messages.read().await;
        Ok(latest.get(validator).copied())
    }

    async fn get_all_latest_messages(&self) -> Result<Vec<(Vec<u8>, BlockHash)>, StoreError> {
        let latest = self.latest_messages.read().await;
        Ok(latest.iter().map(|(k, v)| (k.clone(), *v)).collect())
    }
}

#[cfg(feature = "storage-rocksdb")]
pub struct RocksDbBlockStore {
    db: Arc<rocksdb::DB>,
}

#[cfg(feature = "storage-rocksdb")]
impl Clone for RocksDbBlockStore {
    fn clone(&self) -> Self {
        Self { db: self.db.clone() }
    }
}

#[cfg(feature = "storage-rocksdb")]
impl RocksDbBlockStore {
    pub fn open(path: &str) -> Result<Self, StoreError> {
        use rocksdb::{ColumnFamilyDescriptor, Options};

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(cf::BLOCKS, Options::default()),
            ColumnFamilyDescriptor::new(cf::CHILDREN, Options::default()),
            ColumnFamilyDescriptor::new(cf::META, Options::default()),
            ColumnFamilyDescriptor::new(cf::LATEST, Options::default()),
        ];

        let db = rocksdb::DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| StoreError::Store(format!("Failed to open RocksDB: {e}")))?;

        Ok(Self { db: Arc::new(db) })
    }

    fn cf_handle(&self, name: &str) -> Result<&rocksdb::ColumnFamily, StoreError> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| StoreError::Store(format!("Missing RocksDB column family '{name}'")))
    }
}

#[cfg(feature = "storage-rocksdb")]
#[async_trait]
impl BlockStore for RocksDbBlockStore {
    async fn put(&self, block: &BlockMessage) -> Result<(), StoreError> {
        let db = self.db.clone();
        let block = block.clone();
        tokio::task::spawn_blocking(move || put_block_sync(&db, &block))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn get_by_hash(&self, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError> {
        let db = self.db.clone();
        let hash = *hash;
        tokio::task::spawn_blocking(move || get_block_sync(&db, &hash))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn contains(&self, hash: &BlockHash) -> Result<bool, StoreError> {
        Ok(self.get_by_hash(hash).await?.is_some())
    }
    async fn get_children(&self, hash: &BlockHash) -> Result<Vec<BlockHash>, StoreError> {
        let db = self.db.clone();
        let hash = *hash;
        tokio::task::spawn_blocking(move || get_children_sync(&db, &hash))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn get_genesis(&self) -> Result<Option<BlockMessage>, StoreError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || get_genesis_sync(&db))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn get_dag_representation(&self) -> Result<DagRepresentation, StoreError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || get_dag_representation_sync(&db))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn delete(&self, hash: &BlockHash) -> Result<(), StoreError> {
        let db = self.db.clone();
        let hash = *hash;
        tokio::task::spawn_blocking(move || delete_block_sync(&db, &hash))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn height(&self) -> Result<u64, StoreError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || height_sync(&db))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn update_latest_message(&self, validator: &[u8], block_hash: BlockHash) -> Result<(), StoreError> {
        let db = self.db.clone();
        let validator = validator.to_vec();
        tokio::task::spawn_blocking(move || update_latest_message_sync(&db, &validator, &block_hash))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn get_latest_message(&self, validator: &[u8]) -> Result<Option<BlockHash>, StoreError> {
        let db = self.db.clone();
        let validator = validator.to_vec();
        tokio::task::spawn_blocking(move || get_latest_message_sync(&db, &validator))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
    async fn get_all_latest_messages(&self) -> Result<Vec<(Vec<u8>, BlockHash)>, StoreError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || get_all_latest_messages_sync(&db))
            .await
            .map_err(|e| StoreError::Store(format!("RocksDB task join error: {e}")))?
    }
}

#[cfg(feature = "storage-rocksdb")]
mod cf {
    pub const BLOCKS: &str = "blocks";
    pub const CHILDREN: &str = "children";
    pub const META: &str = "meta";
    pub const LATEST: &str = "latest";
}

#[cfg(feature = "storage-rocksdb")]
mod meta_keys {
    pub const GENESIS_HASH: &[u8] = b"genesis_hash";
}

#[cfg(feature = "storage-rocksdb")]
fn put_block_sync(db: &rocksdb::DB, block: &BlockMessage) -> Result<(), StoreError> {
    let blocks_cf = db
        .cf_handle(cf::BLOCKS)
        .ok_or_else(|| StoreError::Store("Missing CF blocks".to_string()))?;
    let children_cf = db
        .cf_handle(cf::CHILDREN)
        .ok_or_else(|| StoreError::Store("Missing CF children".to_string()))?;
    let meta_cf = db
        .cf_handle(cf::META)
        .ok_or_else(|| StoreError::Store("Missing CF meta".to_string()))?;

    let bytes = block
        .to_proto_bytes()
        .map_err(|e| StoreError::Store(format!("Serialize block: {e}")))?;

    db.put_cf(blocks_cf, block.block_hash, bytes)
        .map_err(|e| StoreError::Store(format!("Put block: {e}")))?;

    if block.header.parents.is_empty() {
        db.put_cf(meta_cf, meta_keys::GENESIS_HASH, block.block_hash)
            .map_err(|e| StoreError::Store(format!("Put genesis pointer: {e}")))?;
    }

    for parent in &block.header.parents {
        let mut current = get_children_bytes_sync(db, children_cf, parent)?;
        append_child(&mut current, &block.block_hash);
        db.put_cf(children_cf, parent, current)
            .map_err(|e| StoreError::Store(format!("Put children index: {e}")))?;
    }

    if !block.sender.is_empty() {
        let latest_cf = db
            .cf_handle(cf::LATEST)
            .ok_or_else(|| StoreError::Store("Missing CF latest".to_string()))?;
        db.put_cf(latest_cf, &block.sender, block.block_hash)
            .map_err(|e| StoreError::Store(format!("Put latest message: {e}")))?;
    }

    Ok(())
}

#[cfg(feature = "storage-rocksdb")]
fn get_block_sync(db: &rocksdb::DB, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError> {
    let blocks_cf = db
        .cf_handle(cf::BLOCKS)
        .ok_or_else(|| StoreError::Store("Missing CF blocks".to_string()))?;
    let bytes_opt = db
        .get_cf(blocks_cf, hash)
        .map_err(|e| StoreError::Store(format!("Get block: {e}")))?;
    let Some(bytes) = bytes_opt else {
        return Ok(None);
    };
    let block = BlockMessage::from_proto_bytes(&bytes)
        .map_err(|e| StoreError::Store(format!("Decode block: {e}")))?;
    Ok(Some(block))
}

#[cfg(feature = "storage-rocksdb")]
fn delete_block_sync(db: &rocksdb::DB, hash: &BlockHash) -> Result<(), StoreError> {
    let blocks_cf = db
        .cf_handle(cf::BLOCKS)
        .ok_or_else(|| StoreError::Store("Missing CF blocks".to_string()))?;
    db.delete_cf(blocks_cf, hash)
        .map_err(|e| StoreError::Store(format!("Delete block: {e}")))?;
    Ok(())
}

#[cfg(feature = "storage-rocksdb")]
fn height_sync(db: &rocksdb::DB) -> Result<u64, StoreError> {
    let blocks_cf = db
        .cf_handle(cf::BLOCKS)
        .ok_or_else(|| StoreError::Store("Missing CF blocks".to_string()))?;
    let mut count: u64 = 0;
    let iter = db.iterator_cf(blocks_cf, rocksdb::IteratorMode::Start);
    for item in iter {
        item.map_err(|e| StoreError::Store(format!("Iter blocks: {e}")))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(feature = "storage-rocksdb")]
fn get_children_sync(db: &rocksdb::DB, hash: &BlockHash) -> Result<Vec<BlockHash>, StoreError> {
    let children_cf = db
        .cf_handle(cf::CHILDREN)
        .ok_or_else(|| StoreError::Store("Missing CF children".to_string()))?;
    let bytes = get_children_bytes_sync(db, children_cf, hash)?;
    decode_hash_list(&bytes)
}

#[cfg(feature = "storage-rocksdb")]
fn get_children_bytes_sync(
    db: &rocksdb::DB,
    children_cf: &rocksdb::ColumnFamily,
    parent: &BlockHash,
) -> Result<Vec<u8>, StoreError> {
    Ok(db
        .get_cf(children_cf, parent)
        .map_err(|e| StoreError::Store(format!("Get children: {e}")))?
        .unwrap_or_default())
}

#[cfg(feature = "storage-rocksdb")]
fn get_genesis_sync(db: &rocksdb::DB) -> Result<Option<BlockMessage>, StoreError> {
    let meta_cf = db
        .cf_handle(cf::META)
        .ok_or_else(|| StoreError::Store("Missing CF meta".to_string()))?;
    let hash_opt = db
        .get_cf(meta_cf, meta_keys::GENESIS_HASH)
        .map_err(|e| StoreError::Store(format!("Get genesis pointer: {e}")))?;
    let Some(hash_bytes) = hash_opt else {
        return Ok(None);
    };
    let hash = bytes_to_hash32(&hash_bytes)?;
    get_block_sync(db, &hash)
}

#[cfg(feature = "storage-rocksdb")]
fn update_latest_message_sync(
    db: &rocksdb::DB,
    validator: &[u8],
    block_hash: &BlockHash,
) -> Result<(), StoreError> {
    let latest_cf = db
        .cf_handle(cf::LATEST)
        .ok_or_else(|| StoreError::Store("Missing CF latest".to_string()))?;
    db.put_cf(latest_cf, validator, block_hash)
        .map_err(|e| StoreError::Store(format!("Put latest message: {e}")))?;
    Ok(())
}

#[cfg(feature = "storage-rocksdb")]
fn get_latest_message_sync(
    db: &rocksdb::DB,
    validator: &[u8],
) -> Result<Option<BlockHash>, StoreError> {
    let latest_cf = db
        .cf_handle(cf::LATEST)
        .ok_or_else(|| StoreError::Store("Missing CF latest".to_string()))?;
    let hash_opt = db
        .get_cf(latest_cf, validator)
        .map_err(|e| StoreError::Store(format!("Get latest message: {e}")))?;
    let Some(hash_bytes) = hash_opt else {
        return Ok(None);
    };
    Ok(Some(bytes_to_hash32(&hash_bytes)?))
}

#[cfg(feature = "storage-rocksdb")]
fn get_all_latest_messages_sync(db: &rocksdb::DB) -> Result<Vec<(Vec<u8>, BlockHash)>, StoreError> {
    let latest_cf = db
        .cf_handle(cf::LATEST)
        .ok_or_else(|| StoreError::Store("Missing CF latest".to_string()))?;
    let mut out = Vec::new();
    let iter = db.iterator_cf(latest_cf, rocksdb::IteratorMode::Start);
    for item in iter {
        let (k, v) = item.map_err(|e| StoreError::Store(format!("Iter latest: {e}")))?;
        out.push((k.to_vec(), bytes_to_hash32(&v)?));
    }
    Ok(out)
}

#[cfg(feature = "storage-rocksdb")]
fn get_dag_representation_sync(db: &rocksdb::DB) -> Result<DagRepresentation, StoreError> {
    let children_cf = db
        .cf_handle(cf::CHILDREN)
        .ok_or_else(|| StoreError::Store("Missing CF children".to_string()))?;
    let mut out = HashMap::new();
    let iter = db.iterator_cf(children_cf, rocksdb::IteratorMode::Start);
    for item in iter {
        let (k, v) = item.map_err(|e| StoreError::Store(format!("Iter children: {e}")))?;
        let parent = bytes_to_hash32(&k)?;
        let children = decode_hash_list(&v)?;
        out.insert(parent, children);
    }
    Ok(DagRepresentation { children: out })
}

#[cfg(feature = "storage-rocksdb")]
fn bytes_to_hash32(bytes: &[u8]) -> Result<[u8; 32], StoreError> {
    if bytes.len() != 32 {
        return Err(StoreError::Store(format!(
            "Expected 32-byte hash, got {} bytes",
            bytes.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[cfg(feature = "storage-rocksdb")]
fn decode_hash_list(bytes: &[u8]) -> Result<Vec<[u8; 32]>, StoreError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() % 32 != 0 {
        return Err(StoreError::Store(format!(
            "Invalid children encoding length {}",
            bytes.len()
        )));
    }
    let mut out = Vec::with_capacity(bytes.len() / 32);
    for chunk in bytes.chunks_exact(32) {
        out.push(bytes_to_hash32(chunk)?);
    }
    Ok(out)
}

#[cfg(feature = "storage-rocksdb")]
fn append_child(buf: &mut Vec<u8>, child: &BlockHash) {
    buf.extend_from_slice(child);
}

impl BlockLookup for InMemoryBlockStore {
    fn get_block(&self, hash: &BlockHash) -> Option<BlockMessage> {
        self.blocks.blocking_read().get(hash).cloned()
    }

    fn contains(&self, hash: &BlockHash) -> bool {
        self.blocks.blocking_read().contains_key(hash)
    }
}
