use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::hashing::{compute_block_hash, compute_body_hash};
use crate::types::{BlockHash, BlockMessage};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Block hash mismatch: expected {expected}, got {actual}")]
    InvalidBlockHash { expected: String, actual: String },
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Body hash mismatch")]
    InvalidBodyHash,
    #[error("Sender not bonded in parents")]
    SenderNotBonded,
    #[error("Invalid sequence number")]
    InvalidSequenceNumber,
    #[error("Post-state hash is zero")]
    ZeroPostStateHash,
    #[error("Shard id mismatch")]
    ShardIdMismatch,
    #[error("Duplicate deploy signature in block")]
    DuplicateDeploySignature,
    #[error("Parent hashes are invalid")]
    InvalidParents,
    #[error("Unknown justification hash")]
    UnknownJustification,
    #[error("Sender public key is empty")]
    EmptySender,
}

pub struct ValidationContext {
    pub known_validators: HashSet<Vec<u8>>,
    pub parent_blocks: HashMap<BlockHash, BlockMessage>,
    pub shard_id: String,
}

pub struct BlockValidator;

impl BlockValidator {
    pub fn validate_structure(block: &BlockMessage) -> Result<(), ValidationError> {
        if block.sender.is_empty() {
            return Err(ValidationError::EmptySender);
        }
        Ok(())
    }

    pub fn validate_hash(block: &BlockMessage) -> Result<(), ValidationError> {
        if compute_block_hash(&block.header) != block.block_hash {
            return Err(ValidationError::InvalidBlockHash {
                expected: hex::encode(compute_block_hash(&block.header)),
                actual: hex::encode(block.block_hash),
            });
        }
        if compute_body_hash(&block.body) != block.header.body_hash {
            return Err(ValidationError::InvalidBodyHash);
        }
        Ok(())
    }

    pub fn validate_signature(block: &BlockMessage) -> Result<(), ValidationError> {
        let key_bytes: [u8; 32] = block
            .sender
            .as_slice()
            .try_into()
            .map_err(|_| ValidationError::InvalidSignature)?;
        let sig_bytes: [u8; 64] = block
            .sig
            .as_slice()
            .try_into()
            .map_err(|_| ValidationError::InvalidSignature)?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|_| ValidationError::InvalidSignature)?;
        let signature = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&block.block_hash, &signature)
            .map_err(|_| ValidationError::InvalidSignature)
    }

    pub fn validate_full(block: &BlockMessage) -> Result<(), ValidationError> {
        Self::validate_structure(block)?;
        Self::validate_hash(block)?;
        Self::validate_signature(block)
    }

    pub fn validate_casper_invariants(
        block: &BlockMessage,
        lookup: &dyn BlockLookup,
    ) -> Result<(), ValidationError> {
        for parent_hash in &block.header.parents {
            if !lookup.contains(parent_hash) {
                return Err(ValidationError::InvalidParents);
            }
        }
        Ok(())
    }
}

pub fn validate_block(block: &BlockMessage, context: &ValidationContext) -> Result<(), ValidationError> {
    BlockValidator::validate_hash(block)?;
    BlockValidator::validate_signature(block)?;
    if block.body.state.post_state_hash == [0u8; 32] {
        return Err(ValidationError::ZeroPostStateHash);
    }
    if block.shard_id.is_empty() || block.shard_id != context.shard_id {
        return Err(ValidationError::ShardIdMismatch);
    }
    if block.body.deploys.iter().any(|d| d.deploy.shard_id != block.shard_id) {
        return Err(ValidationError::ShardIdMismatch);
    }
    let mut parent_set = HashSet::new();
    if block
        .header
        .parents
        .iter()
        .any(|h| *h == [0u8; 32] || !parent_set.insert(*h))
    {
        return Err(ValidationError::InvalidParents);
    }
    let max_parent_seq = context
        .parent_blocks
        .values()
        .map(|b| b.seq_num)
        .max()
        .unwrap_or(-1);
    if block.seq_num != max_parent_seq + 1 {
        return Err(ValidationError::InvalidSequenceNumber);
    }
    let bonded_in_parent = context.parent_blocks.values().any(|parent| {
        parent
            .body
            .state
            .bonds
            .iter()
            .any(|bond| bond.validator == block.sender)
    });
    if !context.parent_blocks.is_empty() && !bonded_in_parent {
        return Err(ValidationError::SenderNotBonded);
    }
    let mut sigs = HashSet::new();
    if block.body.deploys.iter().any(|d| !sigs.insert(d.deploy.sig.clone())) {
        return Err(ValidationError::DuplicateDeploySignature);
    }
    for j in &block.justifications {
        if !context.parent_blocks.contains_key(&j.latest_block_hash) {
            return Err(ValidationError::UnknownJustification);
        }
    }
    Ok(())
}

pub trait BlockLookup: Send + Sync {
    fn get_block(&self, hash: &BlockHash) -> Option<BlockMessage>;
    fn contains(&self, hash: &BlockHash) -> bool;
}
