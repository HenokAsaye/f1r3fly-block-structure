
use std::fs;

use thiserror::Error;

use crate::builder::BlockBuildError;
use crate::hashing::{compute_block_hash, compute_body_hash, compute_post_state_hash};
use crate::types::{BlockBody, BlockHeader, BlockMessage, Bond, RChainState};

#[derive(Clone, Debug)]
pub struct GenesisConfig {
    pub shard_id: String,
    pub validators: Vec<Bond>,
    pub timestamp: i64,
}

impl GenesisConfig {
    pub fn build_genesis_block(&self) -> Result<BlockMessage, BlockBuildError> {
        if self.validators.is_empty() {
            return Err(BlockBuildError::MissingBonds);
        }
        if self.shard_id.is_empty() {
            return Err(BlockBuildError::MissingShardId);
        }

        let body = BlockBody {
            deploys: Vec::new(),
            system_deploys: Vec::new(),
            state: RChainState {
                pre_state_hash: compute_post_state_hash(b"genesis-pre"),
                post_state_hash: compute_post_state_hash(b"genesis-post"),
                bonds: self.validators.clone(),
                block_number: 0,
            },
        };
        let body_hash = compute_body_hash(&body);
        let mut header = BlockHeader {
            parents: Vec::new(),
            sender: vec![0u8; 32],
            sig_algorithm: "ed25519".to_string(),
            sig: vec![0u8; 64],
            shard_id: self.shard_id.clone(),
            seq_num: 0,
            version: 1,
            body_hash,
            block_hash: [0u8; 32],
            dag_level: 0,
            justifications: Vec::new(),
        };

        let block_hash = compute_block_hash(&header);
        header.block_hash = block_hash;

        Ok(BlockMessage {
            block_hash,
            header,
            body,
            justifications: Vec::new(),
            sender: vec![0u8; 32],
            seq_num: 0,
            sig: vec![0u8; 64],
            sig_algorithm: "ed25519".to_string(),
            shard_id: self.shard_id.clone(),
            extra_bytes: Vec::new(),
        })
    }

    pub fn from_bonds_file(path: &str) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        Self::from_bonds_str("f1r3fly", &content)
    }

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

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid bonds line '{0}': expected 'hex_pubkey stake'")]
    InvalidBondsLine(String),
    #[error("Invalid hex pubkey: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("Invalid stake amount: {0}")]
    InvalidStake(String),
    #[error("Validator set is empty")]
    EmptyValidatorSet,
}

