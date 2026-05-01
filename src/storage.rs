//! Block storage trait and in-memory implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;

use crate::types::{BlockHash, BlockMessage};
use crate::validation::BlockLookup;

/// Errors returned by block storage implementations.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Storage operation failed.
    #[error("Store error: {0}")]
    Store(String),
}

/// Block storage interface compatible with F1r3fly block-storage patterns.
#[async_trait]
pub trait BlockStore: Send + Sync {
    /// Store a block.
    ///
    /// Returns `StoreError` on failure.
    async fn put(&self, block: &BlockMessage) -> Result<(), StoreError>;
    /// Get a block by hash.
    ///
    /// Returns `StoreError` on failure.
    async fn get(&self, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError>;
    /// Check if a block exists by hash.
    ///
    /// Returns `StoreError` on failure.
    async fn contains(&self, hash: &BlockHash) -> Result<bool, StoreError>;
    /// Get children of a block hash.
    ///
    /// Returns `StoreError` on failure.
    async fn get_children(&self, hash: &BlockHash) -> Result<Vec<BlockHash>, StoreError>;
    /// Get genesis block if present.
    ///
    /// Returns `StoreError` on failure.
    async fn get_genesis(&self) -> Result<Option<BlockMessage>, StoreError>;
    /// Delete a block.
    ///
    /// Returns `StoreError` on failure.
    async fn delete(&self, hash: &BlockHash) -> Result<(), StoreError>;
    /// Get number of stored blocks.
    ///
    /// Returns `StoreError` on failure.
    async fn height(&self) -> Result<u64, StoreError>;

    /// Update latest message for a validator.
    ///
    /// Returns `StoreError` on failure.
    async fn update_latest_message(
        &self,
        validator: &[u8],
        block_hash: BlockHash,
    ) -> Result<(), StoreError>;
    /// Get latest message for a validator.
    ///
    /// Returns `StoreError` on failure.
    async fn get_latest_message(&self, validator: &[u8]) -> Result<Option<BlockHash>, StoreError>;
    /// Get all latest messages.
    ///
    /// Returns `StoreError` on failure.
    async fn get_all_latest_messages(&self) -> Result<Vec<(Vec<u8>, BlockHash)>, StoreError>;
}

/// In-memory implementation for testing.
pub struct InMemoryBlockStore {
    blocks: Arc<RwLock<HashMap<BlockHash, BlockMessage>>>,
    children: Arc<RwLock<HashMap<BlockHash, Vec<BlockHash>>>>,
    genesis: Arc<RwLock<Option<BlockHash>>>,
    latest_messages: Arc<RwLock<HashMap<Vec<u8>, BlockHash>>>,
}

impl InMemoryBlockStore {
    /// Create a new in-memory store.
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

        if block.header.parents_hash_list.is_empty() {
            let mut genesis = self.genesis.write().await;
            *genesis = Some(block.block_hash);
        }

        let mut children = self.children.write().await;
        for parent in &block.header.parents_hash_list {
            children.entry(*parent).or_default().push(block.block_hash);
        }

        if !block.sender.is_empty() {
            let mut latest = self.latest_messages.write().await;
            latest.insert(block.sender.clone(), block.block_hash);
        }
        Ok(())
    }

    async fn get(&self, hash: &BlockHash) -> Result<Option<BlockMessage>, StoreError> {
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
            Some(hash) => self.get(&hash).await,
            None => Ok(None),
        }
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

impl BlockLookup for InMemoryBlockStore {
    fn get_block(&self, hash: &BlockHash) -> Option<BlockMessage> {
        self.blocks.blocking_read().get(hash).cloned()
    }

    fn contains(&self, hash: &BlockHash) -> bool {
        self.blocks.blocking_read().contains_key(hash)
    }
}
