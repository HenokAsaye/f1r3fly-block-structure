//! Block builder for constructing valid blocks.

use chrono::Utc;
use thiserror::Error;

use crate::hashing::{compute_block_hash, compute_bonds_map_hash};
use crate::types::{
    BlockBody, BlockHash, BlockHeader, BlockMessage, Bond, BondedValidatorInfo, Justification,
    ProcessedDeploy, PublicKey, Signature, StateDagHash, StateHash,
};

/// Errors that can occur while building a block.
#[derive(Debug, Error)]
pub enum BlockBuildError {
    /// Missing parent hashes.
    #[error("Missing parent hashes")]
    MissingParents,
    /// Missing post-state hash.
    #[error("Missing post-state hash")]
    MissingPostStateHash,
    /// Missing bonds.
    #[error("Missing bonds")]
    MissingBonds,
    /// Missing shard id.
    #[error("Missing shard id")]
    MissingShardId,
    /// Missing sender.
    #[error("Missing sender")]
    MissingSender,
    /// Missing sequence number.
    #[error("Missing sequence number")]
    MissingSeqNum,
}

/// Builder for assembling blocks with fluent setters.
#[derive(Default)]
pub struct BlockBuilder {
    parents: Vec<BlockHash>,
    deploys: Vec<ProcessedDeploy>,
    post_state_hash: Option<StateHash>,
    bonds: Vec<Bond>,
    justifications: Vec<Justification>,
    shard_id: Option<String>,
    sender: Option<PublicKey>,
    seq_num: Option<i64>,
    timestamp: Option<i64>,
}

impl BlockBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a parent hash.
    pub fn with_parent(mut self, parent_hash: BlockHash) -> Self {
        self.parents.push(parent_hash);
        self
    }

    /// Add parent hashes.
    pub fn with_parents(mut self, parent_hashes: Vec<BlockHash>) -> Self {
        self.parents.extend(parent_hashes);
        self
    }

    /// Add a deploy.
    pub fn with_deploy(mut self, deploy: ProcessedDeploy) -> Self {
        self.deploys.push(deploy);
        self
    }

    /// Add deploys.
    pub fn with_deploys(mut self, deploys: Vec<ProcessedDeploy>) -> Self {
        self.deploys.extend(deploys);
        self
    }

    /// Set post-state hash.
    pub fn with_post_state_hash(mut self, hash: StateHash) -> Self {
        self.post_state_hash = Some(hash);
        self
    }

    /// Set bonds.
    pub fn with_bonds(mut self, bonds: Vec<Bond>) -> Self {
        self.bonds = bonds;
        self
    }

    /// Set justifications.
    pub fn with_justifications(mut self, justifications: Vec<Justification>) -> Self {
        self.justifications = justifications;
        self
    }

    /// Set shard id.
    pub fn with_shard_id(mut self, shard_id: String) -> Self {
        self.shard_id = Some(shard_id);
        self
    }

    /// Set sender public key.
    pub fn with_sender(mut self, public_key: PublicKey) -> Self {
        self.sender = Some(public_key);
        self
    }

    /// Set sequence number.
    pub fn with_seq_num(mut self, seq_num: i64) -> Self {
        self.seq_num = Some(seq_num);
        self
    }

    /// Set timestamp in milliseconds since epoch.
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Finalizes, computes block_hash, ready to sign.
    pub fn build_unsigned(self) -> Result<UnsignedBlock, BlockBuildError> {
        if self.parents.is_empty() {
            return Err(BlockBuildError::MissingParents);
        }
        let post_state_hash = self.post_state_hash.ok_or(BlockBuildError::MissingPostStateHash)?;
        if self.bonds.is_empty() {
            return Err(BlockBuildError::MissingBonds);
        }
        let shard_id = self.shard_id.ok_or(BlockBuildError::MissingShardId)?;
        let sender = self.sender.ok_or(BlockBuildError::MissingSender)?;
        let seq_num = self.seq_num.ok_or(BlockBuildError::MissingSeqNum)?;
        let timestamp = self.timestamp.unwrap_or_else(|| Utc::now().timestamp_millis());

        let bonds_map_hash = compute_bonds_map_hash(&self.bonds);
        let state_dag = bonds_to_state_dag(&self.bonds);
        let state_dag_hash = compute_state_dag_hash(&state_dag);

        let header = BlockHeader {
            parents_hash_list: self.parents,
            post_state_hash,
            bonds_map_hash,
            state_dag_hash,
            deploy_count: self.deploys.len() as u32,
            timestamp,
            version: 1,
            seq_num,
            shard_id: shard_id.clone(),
        };

        let body = BlockBody {
            deploys: self.deploys,
            system_deploys: Vec::new(),
            state_dag,
        };

        let block_hash = compute_block_hash(&header);

        let block = BlockMessage {
            block_hash,
            header,
            body,
            justifications: self.justifications,
            sender,
            sig: Vec::new(),
            sig_algorithm: "ed25519".to_string(),
            shard_id,
            extra_bytes: Vec::new(),
        };

        Ok(UnsignedBlock { block })
    }

    /// Builds and signs with provided signing function.
    pub fn build_and_sign<F>(self, sign_fn: F) -> Result<BlockMessage, BlockBuildError>
    where
        F: Fn(&[u8]) -> Signature,
    {
        Ok(self.build_unsigned()?.sign(sign_fn))
    }
}

/// An unsigned block with empty signature.
#[derive(Debug)]
pub struct UnsignedBlock {
    /// Block message with empty signature.
    pub block: BlockMessage,
}

impl UnsignedBlock {
    /// Sign the block with a signing function over the block hash.
    pub fn sign<F>(mut self, sign_fn: F) -> BlockMessage
    where
        F: Fn(&[u8]) -> Signature,
    {
        self.block.sig = sign_fn(&self.block.block_hash);
        self.block
    }
}

fn bonds_to_state_dag(bonds: &[Bond]) -> Vec<BondedValidatorInfo> {
    bonds
        .iter()
        .map(|bond| BondedValidatorInfo {
            validator: bond.validator.clone(),
            stake: bond.stake,
        })
        .collect()
}

fn compute_state_dag_hash(state_dag: &[BondedValidatorInfo]) -> StateDagHash {
    let mut bytes = Vec::new();
    for entry in state_dag {
        bytes.extend_from_slice(&(entry.validator.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&entry.validator);
        bytes.extend_from_slice(&entry.stake.to_le_bytes());
    }
    crate::hashing::compute_post_state_hash(&bytes)
}
