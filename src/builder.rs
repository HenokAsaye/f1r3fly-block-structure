use chrono::Utc;
use thiserror::Error;

use crate::hashing::{compute_block_hash, compute_body_hash};
use crate::types::{
    BlockBody, BlockHash, BlockHeader, BlockMessage, Bond, Justification, ProcessedDeploy,
    PublicKey, RChainState, Signature, StateHash,
};

#[derive(Debug, Error)]
pub enum BlockBuildError {
    #[error("Missing parent hashes")]
    MissingParents,
    #[error("Missing post-state hash")]
    MissingPostStateHash,
    #[error("Missing bonds")]
    MissingBonds,
    #[error("Missing shard id")]
    MissingShardId,
    #[error("Missing sender")]
    MissingSender,
    #[error("Missing sequence number")]
    MissingSeqNum,
}

#[derive(Default)]
pub struct BlockBuilder {
    parents: Vec<BlockHash>,
    deploys: Vec<ProcessedDeploy>,
    system_deploys: Vec<crate::types::ProcessedSystemDeploy>,
    post_state_hash: Option<StateHash>,
    pre_state_hash: Option<StateHash>,
    bonds: Vec<Bond>,
    justifications: Vec<Justification>,
    shard_id: Option<String>,
    sender: Option<PublicKey>,
    seq_num: Option<i64>,
    timestamp: Option<i64>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(self, parent_hash: BlockHash) -> Self {
        self.with_parents(vec![parent_hash])
    }

    pub fn with_parents(mut self, parent_hashes: Vec<BlockHash>) -> Self {
        self.parents.extend(parent_hashes);
        self
    }

    pub fn with_deploy(mut self, deploy: ProcessedDeploy) -> Self {
        self.deploys.push(deploy);
        self
    }

    pub fn with_deploys(mut self, deploys: Vec<ProcessedDeploy>) -> Self {
        self.deploys.extend(deploys);
        self
    }

    pub fn with_post_state_hash(mut self, hash: StateHash) -> Self {
        self.post_state_hash = Some(hash);
        self
    }

    pub fn with_bonds(mut self, bonds: Vec<Bond>) -> Self {
        self.bonds = bonds;
        self
    }

    pub fn with_justifications(mut self, justifications: Vec<Justification>) -> Self {
        self.justifications = justifications;
        self
    }

    pub fn with_shard_id(mut self, shard_id: String) -> Self {
        self.shard_id = Some(shard_id);
        self
    }

    pub fn with_sender(mut self, public_key: PublicKey) -> Self {
        self.sender = Some(public_key);
        self
    }

    pub fn with_seq_num(mut self, seq_num: i64) -> Self {
        self.seq_num = Some(seq_num);
        self
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

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
        let _timestamp = self.timestamp.unwrap_or_else(|| Utc::now().timestamp_millis());

        let body = BlockBody {
            deploys: self.deploys,
            system_deploys: self.system_deploys,
            state: RChainState {
                pre_state_hash: self.pre_state_hash.unwrap_or([0u8; 32]),
                post_state_hash,
                bonds: self.bonds,
                block_number: seq_num,
            },
        };

        let body_hash = compute_body_hash(&body);
        let mut header = BlockHeader {
            parents: self.parents,
            sender: sender.clone(),
            sig_algorithm: "ed25519".to_string(),
            sig: Vec::new(),
            shard_id: shard_id.clone(),
            seq_num,
            version: 1,
            body_hash,
            block_hash: [0u8; 32],
            dag_level: seq_num,
            justifications: self.justifications.clone(),
        };
        let block_hash = compute_block_hash(&header);
        header.block_hash = block_hash;

        let block = BlockMessage {
            block_hash,
            header,
            body,
            justifications: self.justifications,
            sender,
            seq_num,
            sig: Vec::new(),
            sig_algorithm: "ed25519".to_string(),
            shard_id,
            extra_bytes: Vec::new(),
        };

        Ok(UnsignedBlock { block })
    }

    pub fn build_and_sign<F>(self, sign_fn: F) -> Result<BlockMessage, BlockBuildError>
    where
        F: Fn(&[u8]) -> Signature,
    {
        Ok(self.build_unsigned()?.sign(sign_fn))
    }
}

#[derive(Debug)]
pub struct UnsignedBlock {
    pub block: BlockMessage,
}

impl UnsignedBlock {
    pub fn sign<F>(mut self, sign_fn: F) -> BlockMessage
    where
        F: Fn(&[u8]) -> Signature,
    {
        self.block.sig = sign_fn(&self.block.block_hash);
        self.block
    }
}
