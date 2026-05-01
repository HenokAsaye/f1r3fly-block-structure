//! Block validation routines.

use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use thiserror::Error;

use crate::hashing::compute_header_hash;
use crate::types::{BlockHash, BlockMessage};

/// Errors returned during block validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Block hash mismatch.
    #[error("Invalid block hash: expected {expected}, got {actual}")]
    InvalidBlockHash { expected: String, actual: String },
    /// Signature is invalid.
    #[error("Invalid signature")]
    InvalidSignature,
    /// Parent hashes missing or unknown.
    #[error("Missing parent hashes")]
    MissingParents,
    /// Deploy timestamp invalid.
    #[error("Deploy timestamp out of range")]
    DeployTimestampInvalid,
    /// Shard ID invalid.
    #[error("Invalid shard ID")]
    InvalidShardId,
    /// Sequence number invalid.
    #[error("Seq num is not monotonically increasing")]
    InvalidSeqNum,
    /// Phlo limit invalid.
    #[error("Zero or negative phlo limit")]
    InvalidPhloLimit,
    /// Bonds map empty.
    #[error("Bonds map is empty")]
    EmptyBondsMap,
}

/// Block validator for structural checks.
pub struct BlockValidator;

impl BlockValidator {
    /// Full structural validation (does not check chain history).
    pub fn validate_structure(block: &BlockMessage) -> Result<(), ValidationError> {
        if block.header.parents_hash_list.is_empty() {
            return Err(ValidationError::MissingParents);
        }
        if block.header.shard_id.is_empty() || block.shard_id.is_empty() {
            return Err(ValidationError::InvalidShardId);
        }
        if block.header.seq_num < 0 {
            return Err(ValidationError::InvalidSeqNum);
        }
        if block.header.bonds_map_hash == [0u8; 32] {
            return Err(ValidationError::EmptyBondsMap);
        }
        Self::validate_deploys(block)
    }

    /// Verify Ed25519 signature over block_hash using sender's public key.
    pub fn validate_signature(block: &BlockMessage) -> Result<(), ValidationError> {
        if block.sender.len() != 32 || block.sig.len() != 64 {
            return Err(ValidationError::InvalidSignature);
        }
        let key_bytes: [u8; 32] = block
            .sender
            .as_slice()
            .try_into()
            .map_err(|_| ValidationError::InvalidSignature)?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| ValidationError::InvalidSignature)?;
        let sig = DalekSignature::from_slice(&block.sig).map_err(|_| ValidationError::InvalidSignature)?;
        verifying_key
            .verify_strict(&block.block_hash, &sig)
            .map_err(|_| ValidationError::InvalidSignature)
    }

    /// Verify block hash matches computed hash of header.
    pub fn validate_hash(block: &BlockMessage) -> Result<(), ValidationError> {
        let expected = compute_header_hash(&block.header);
        if expected != block.block_hash {
            return Err(ValidationError::InvalidBlockHash {
                expected: hex::encode(expected),
                actual: hex::encode(block.block_hash),
            });
        }
        Ok(())
    }

    /// Validate all deploys in the block body.
    pub fn validate_deploys(block: &BlockMessage) -> Result<(), ValidationError> {
        for deploy in &block.body.deploys {
            if deploy.deploy.phlo_limit <= 0 {
                return Err(ValidationError::InvalidPhloLimit);
            }
            if deploy.deploy.timestamp > block.header.timestamp || deploy.deploy.timestamp < 0 {
                return Err(ValidationError::DeployTimestampInvalid);
            }
        }
        Ok(())
    }

    /// Check Casper-specific invariants (parents, justifications).
    pub fn validate_casper_invariants(
        block: &BlockMessage,
        known_blocks: &dyn BlockLookup,
    ) -> Result<(), ValidationError> {
        if block.header.parents_hash_list.is_empty() {
            return Err(ValidationError::MissingParents);
        }
        for parent in &block.header.parents_hash_list {
            if !known_blocks.contains(parent) {
                return Err(ValidationError::MissingParents);
            }
        }
        for just in &block.justifications {
            if !known_blocks.contains(&just.latest_block_hash) {
                return Err(ValidationError::MissingParents);
            }
        }
        Ok(())
    }
}

/// Block lookup interface for validation checks.
pub trait BlockLookup {
    /// Get a block by hash.
    fn get_block(&self, hash: &BlockHash) -> Option<&BlockMessage>;
    /// Check if block exists by hash.
    fn contains(&self, hash: &BlockHash) -> bool;
}
