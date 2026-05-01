//! Block validation routines.

use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use thiserror::Error;

use crate::hashing::compute_block_hash;
use crate::types::{BlockHash, BlockMessage};

/// Errors returned during block validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Block hash mismatch.
    #[error("Block hash mismatch: expected {expected}, got {actual}")]
    InvalidBlockHash { expected: String, actual: String },
    /// Signature is invalid.
    #[error("Signature verification failed")]
    InvalidSignature,
    /// Missing parents for non-genesis blocks.
    #[error("Block has no parents and is not genesis")]
    MissingParents,
    /// Shard ID mismatch between deploy and block.
    #[error("Deploy shard_id '{deploy}' does not match block shard_id '{block}'")]
    ShardIdMismatch { deploy: String, block: String },
    /// Phlo limit must be positive.
    #[error("Invalid phlo limit {0}: must be > 0")]
    InvalidPhloLimit(i64),
    /// Phlo price must be positive.
    #[error("Invalid phlo price {0}: must be > 0")]
    InvalidPhloPrice(i64),
    /// Bonds map is empty.
    #[error("Bonds map is empty")]
    EmptyBondsMap,
    /// Deploy timestamp is in the future.
    #[error("Deploy timestamp {0} is in the future")]
    DeployTimestampInFuture(i64),
    /// Sender key is empty.
    #[error("Sender public key is empty")]
    EmptySender,
    /// Signature is empty.
    #[error("Signature is empty")]
    EmptySignature,
    /// Sequence number invalid.
    #[error("Seq num {0} is not positive")]
    InvalidSeqNum(i64),
    /// Justification refers to unknown validator.
    #[error("Justification references unknown validator")]
    JustificationValidatorMismatch,
}

/// Block validator for structural checks.
pub struct BlockValidator;

impl BlockValidator {
    /// Runs all structural checks. Does not verify signature.
    ///
    /// Returns `ValidationError` for any structural invariant violation.
    pub fn validate_structure(block: &BlockMessage) -> Result<(), ValidationError> {
        if block.sender.is_empty() {
            return Err(ValidationError::EmptySender);
        }
        if block.sig.is_empty() {
            return Err(ValidationError::EmptySignature);
        }
        if block.shard_id.is_empty() {
            return Err(ValidationError::ShardIdMismatch {
                deploy: String::new(),
                block: block.shard_id.clone(),
            });
        }
        if block.header.seq_num < 0 || (block.header.seq_num == 0 && !block.header.parents_hash_list.is_empty()) {
            return Err(ValidationError::InvalidSeqNum(block.header.seq_num));
        }
        if block.header.seq_num > 0 && block.header.parents_hash_list.is_empty() {
            return Err(ValidationError::MissingParents);
        }
        if block.header.bonds_map_hash == [0u8; 32] || block.body.state_dag.is_empty() {
            return Err(ValidationError::EmptyBondsMap);
        }

        let now = Utc::now().timestamp_millis();
        for deploy in &block.body.deploys {
            if deploy.deploy.phlo_limit <= 0 {
                return Err(ValidationError::InvalidPhloLimit(deploy.deploy.phlo_limit));
            }
            if deploy.deploy.phlo_price <= 0 {
                return Err(ValidationError::InvalidPhloPrice(deploy.deploy.phlo_price));
            }
            if deploy.deploy.shard_id != block.shard_id {
                return Err(ValidationError::ShardIdMismatch {
                    deploy: deploy.deploy.shard_id.clone(),
                    block: block.shard_id.clone(),
                });
            }
            if deploy.deploy.timestamp > now {
                return Err(ValidationError::DeployTimestampInFuture(deploy.deploy.timestamp));
            }
        }

        let validators: Vec<&[u8]> = block
            .body
            .state_dag
            .iter()
            .map(|b| b.validator.as_slice())
            .collect();
        for justification in &block.justifications {
            let known = validators.contains(&justification.validator.as_slice());
            if !known {
                return Err(ValidationError::JustificationValidatorMismatch);
            }
        }

        Ok(())
    }

    /// Verify block_hash == Blake2b256(canonical_header_bytes).
    ///
    /// Returns `ValidationError::InvalidBlockHash` on mismatch.
    pub fn validate_hash(block: &BlockMessage) -> Result<(), ValidationError> {
        let computed = compute_block_hash(&block.header);
        if computed != block.block_hash {
            return Err(ValidationError::InvalidBlockHash {
                expected: hex::encode(computed),
                actual: hex::encode(block.block_hash),
            });
        }
        Ok(())
    }

    /// Verify Ed25519 signature over block_hash.
    ///
    /// Returns `ValidationError::InvalidSignature` on failure.
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

    /// Full validation: structure + hash + signature.
    ///
    /// Returns the first `ValidationError` encountered.
    pub fn validate_full(block: &BlockMessage) -> Result<(), ValidationError> {
        Self::validate_structure(block)?;
        Self::validate_hash(block)?;
        Self::validate_signature(block)
    }

    /// Casper invariants: check that all parents are known.
    ///
    /// Returns `ValidationError::MissingParents` if any parent is unknown.
    pub fn validate_casper_invariants(
        block: &BlockMessage,
        lookup: &dyn BlockLookup,
    ) -> Result<(), ValidationError> {
        for parent_hash in &block.header.parents_hash_list {
            if !lookup.contains(parent_hash) {
                return Err(ValidationError::MissingParents);
            }
        }
        Ok(())
    }
}

/// Block lookup interface for validation checks.
pub trait BlockLookup: Send + Sync {
    /// Get a block by hash.
    ///
    /// Infallible; returns `None` when missing.
    fn get_block(&self, hash: &BlockHash) -> Option<BlockMessage>;
    /// Check if block exists by hash.
    ///
    /// Infallible.
    fn contains(&self, hash: &BlockHash) -> bool;
}
