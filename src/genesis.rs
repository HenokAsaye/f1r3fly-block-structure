//! Genesis block configuration and builder.

use std::fs;

use chrono::Utc;
use thiserror::Error;

use crate::builder::BlockBuildError;
use crate::hashing::{compute_block_hash, compute_bonds_map_hash, compute_post_state_hash};
use crate::types::{
    BlockBody, BlockHeader, BlockMessage, Bond, BondedValidatorInfo, PublicKey, StateDagHash,
    StateHash,
};

/// Genesis configuration for building a genesis block.
#[derive(Clone, Debug)]
pub struct GenesisConfig {
    /// Shard identifier.
    pub shard_id: String,
    /// Initial validator set with stakes.
    pub validators: Vec<Bond>,
    /// Pre-funded accounts.
    pub wallets: Vec<WalletEntry>,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
}

/// Wallet entry for genesis configuration.
#[derive(Clone, Debug)]
pub struct WalletEntry {
    /// Wallet address public key bytes.
    pub address: PublicKey,
    /// Initial balance.
    pub initial_balance: i64,
}

/// Errors returned while parsing genesis config files.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File read error.
    #[error("Config IO error: {0}")]
    Io(String),
    /// Parsing error.
    #[error("Config parse error: {0}")]
    Parse(String),
}

impl GenesisConfig {
    /// Build a genesis BlockMessage from config.
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
        let post_state_hash = compute_wallets_state_hash(&self.wallets);

        let header = BlockHeader {
            parents_hash_list: Vec::new(),
            post_state_hash,
            bonds_map_hash,
            state_dag_hash,
            deploy_count: 0,
            timestamp: if self.timestamp == 0 { Utc::now().timestamp_millis() } else { self.timestamp },
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
            sender: Vec::new(),
            sig: Vec::new(),
            sig_algorithm: "ed25519".to_string(),
            shard_id: self.shard_id.clone(),
            extra_bytes: Vec::new(),
        })
    }

    /// Load from bonds.txt and wallets.txt format.
    pub fn from_files(bonds_path: &str, wallets_path: &str) -> Result<Self, ConfigError> {
        let bonds_txt = fs::read_to_string(bonds_path).map_err(|e| ConfigError::Io(e.to_string()))?;
        let wallets_txt = fs::read_to_string(wallets_path).map_err(|e| ConfigError::Io(e.to_string()))?;

        let validators = parse_bonds(&bonds_txt)?;
        let wallets = parse_wallets(&wallets_txt)?;

        Ok(GenesisConfig {
            shard_id: "f1r3fly".to_string(),
            validators,
            wallets,
            timestamp: 0,
        })
    }
}

fn parse_bonds(contents: &str) -> Result<Vec<Bond>, ConfigError> {
    let mut out = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = split_line(line);
        if parts.len() != 2 {
            return Err(ConfigError::Parse(format!("Invalid bonds line {}", idx + 1)));
        }
        let key = hex::decode(parts[0]).map_err(|_| ConfigError::Parse("Invalid hex in bonds".to_string()))?;
        let stake: i64 = parts[1]
            .parse()
            .map_err(|_| ConfigError::Parse("Invalid stake".to_string()))?;
        out.push(Bond { validator: key, stake });
    }
    Ok(out)
}

fn parse_wallets(contents: &str) -> Result<Vec<WalletEntry>, ConfigError> {
    let mut out = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = split_line(line);
        if parts.len() != 2 {
            return Err(ConfigError::Parse(format!("Invalid wallets line {}", idx + 1)));
        }
        let addr = hex::decode(parts[0]).map_err(|_| ConfigError::Parse("Invalid hex in wallets".to_string()))?;
        let balance: i64 = parts[1]
            .parse()
            .map_err(|_| ConfigError::Parse("Invalid balance".to_string()))?;
        out.push(WalletEntry { address: addr, initial_balance: balance });
    }
    Ok(out)
}

fn split_line(line: &str) -> Vec<&str> {
    line.split(|c| c == ',' || c == ' ' || c == '\t')
        .filter(|s| !s.is_empty())
        .collect()
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
    compute_post_state_hash(&bytes)
}

fn compute_wallets_state_hash(wallets: &[WalletEntry]) -> StateHash {
    let mut bytes = Vec::new();
    for entry in wallets {
        bytes.extend_from_slice(&(entry.address.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&entry.address);
        bytes.extend_from_slice(&entry.initial_balance.to_le_bytes());
    }
    compute_post_state_hash(&bytes)
}
