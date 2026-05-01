//! Genesis block configuration and builder.

use std::fs;

use thiserror::Error;

use crate::builder::BlockBuildError;
use crate::hashing::{compute_block_hash, compute_bonds_map_hash, compute_post_state_hash};
use crate::types::{
    BlockBody, BlockHeader, BlockMessage, Bond, BondedValidatorInfo, StateDagHash,
};

/// Genesis configuration for building a genesis block.
#[derive(Clone, Debug)]
pub struct GenesisConfig {
    /// Shard identifier.
    pub shard_id: String,
    /// Initial validator set with stakes.
    pub validators: Vec<Bond>,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
}

impl GenesisConfig {
    /// Build a genesis BlockMessage.
    ///
    /// Genesis has empty parents, seq_num = 0, and empty justifications.
    /// The post-state hash is Blake2b256(b"genesis").
    /// Returns `BlockBuildError` when required fields are missing.
    pub fn build_genesis_block(&self) -> Result<BlockMessage, BlockBuildError> {
        if self.validators.is_empty() {
            return Err(BlockBuildError::MissingBonds);
        }
        if self.shard_id.is_empty() {
            return Err(BlockBuildError::MissingShardId);
        }

        let bonds_map_hash = compute_bonds_map_hash(&self.validators);
        let state_dag = bonds_to_state_dag(&self.validators);
        let state_dag_hash = compute_state_dag_hash(&state_dag);
        let post_state_hash = compute_post_state_hash(b"genesis");

        let header = BlockHeader {
            parents_hash_list: Vec::new(),
            post_state_hash,
            bonds_map_hash,
            state_dag_hash,
            deploy_count: 0,
            timestamp: self.timestamp,
            version: 1,
            seq_num: 0,
            shard_id: self.shard_id.clone(),
        };

        let body = BlockBody {
            deploys: Vec::new(),
            system_deploys: Vec::new(),
            state_dag,
        };

        let block_hash = compute_block_hash(&header);

        Ok(BlockMessage {
            block_hash,
            header,
            body,
            justifications: Vec::new(),
            sender: vec![0u8; 32],
            sig: vec![0u8; 64],
            sig_algorithm: "ed25519".to_string(),
            shard_id: self.shard_id.clone(),
            extra_bytes: Vec::new(),
        })
    }

    /// Load from bonds.txt format: one line per validator, "hex_pubkey stake".
    ///
    /// Returns `ConfigError` on IO or parse failures.
    pub fn from_bonds_file(path: &str) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        Self::from_bonds_str("f1r3fly", &content)
    }

    /// Parse bonds.txt content directly (for tests without filesystem).
    ///
    /// Returns `ConfigError` on parse failures or empty validator set.
    pub fn from_bonds_str(shard_id: &str, content: &str) -> Result<Self, ConfigError> {
        let mut validators = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let pubkey = parts
                .next()
                .ok_or_else(|| ConfigError::InvalidBondsLine(line.to_string()))?;
            let stake = parts
                .next()
                .ok_or_else(|| ConfigError::InvalidBondsLine(line.to_string()))?;
            if parts.next().is_some() {
                return Err(ConfigError::InvalidBondsLine(line.to_string()));
            }
            let validator = hex::decode(pubkey)?;
            let stake_value: i64 = stake
                .parse()
                .map_err(|_| ConfigError::InvalidStake(stake.to_string()))?;
            validators.push(Bond {
                validator,
                stake: stake_value,
            });
        }

        if validators.is_empty() {
            return Err(ConfigError::EmptyValidatorSet);
        }

        Ok(GenesisConfig {
            shard_id: shard_id.to_string(),
            validators,
            timestamp: 0,
        })
    }
}

/// Errors returned while parsing genesis config files.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File read error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Bonds line is invalid.
    #[error("Invalid bonds line '{0}': expected 'hex_pubkey stake'")]
    InvalidBondsLine(String),
    /// Hex decoding error.
    #[error("Invalid hex pubkey: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    /// Stake parsing error.
    #[error("Invalid stake amount: {0}")]
    InvalidStake(String),
    /// Validator set is empty.
    #[error("Validator set is empty")]
    EmptyValidatorSet,
}

fn bonds_to_state_dag(bonds: &[Bond]) -> Vec<BondedValidatorInfo> {
    bonds
        .iter()
        .map(|bond| BondedValidatorInfo {
            validator: bond.validator.clone(),
            free_stake: bond.stake,
        })
        .collect()
}

fn compute_state_dag_hash(state_dag: &[BondedValidatorInfo]) -> StateDagHash {
    let mut bytes = Vec::new();
    for entry in state_dag {
        bytes.extend_from_slice(&(entry.validator.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&entry.validator);
        bytes.extend_from_slice(&entry.free_stake.to_le_bytes());
    }
    compute_post_state_hash(&bytes)
}

