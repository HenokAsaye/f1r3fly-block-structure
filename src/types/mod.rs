//! Core block data structures.

use serde::{Deserialize, Serialize};

/// Blake2b-256 hash of a block header.
pub type BlockHash = [u8; 32];
/// Blake2b-256 hash of tuplespace state root.
pub type StateHash = [u8; 32];
/// Blake2b-256 hash of validator bonds map.
pub type BondsHash = [u8; 32];
/// Blake2b-256 hash of state DAG data.
pub type StateDagHash = [u8; 32];
/// Ed25519 public key bytes.
pub type PublicKey = Vec<u8>;
/// Ed25519 signature bytes.
pub type Signature = Vec<u8>;
/// Cost in phlo units.
pub type PCost = i64;

/// The complete block message — top-level unit of the chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMessage {
    /// Blake2b-256 hash of the header.
    pub block_hash: BlockHash,
    /// Block header data.
    pub header: BlockHeader,
    /// Block body data.
    pub body: BlockBody,
    /// Casper justifications.
    pub justifications: Vec<Justification>,
    /// Validator's Ed25519 public key.
    pub sender: PublicKey,
    /// Ed25519 signature over block_hash.
    pub sig: Signature,
    /// Signature algorithm name.
    pub sig_algorithm: String,
    /// Shard identifier.
    pub shard_id: String,
    /// Reserved extra bytes.
    pub extra_bytes: Vec<u8>,
}

/// Block header — the structural metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// DAG parents (Casper).
    pub parents_hash_list: Vec<BlockHash>,
    /// Tuplespace root after execution.
    pub post_state_hash: StateHash,
    /// Validator bonds after block.
    pub bonds_map_hash: BondsHash,
    /// Hash of state DAG data.
    pub state_dag_hash: StateDagHash,
    /// Number of deploys in the block body.
    pub deploy_count: u32,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
    /// Block version.
    pub version: i64,
    /// Per-validator sequence number.
    pub seq_num: i64,
    /// Shard identifier.
    pub shard_id: String,
}

/// Block body — the actual payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBody {
    /// User deploys.
    pub deploys: Vec<ProcessedDeploy>,
    /// System deploys.
    pub system_deploys: Vec<ProcessedSystemDeploy>,
    /// State DAG bonded validators.
    pub state_dag: Vec<BondedValidatorInfo>,
}

/// A processed user deploy (transaction).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessedDeploy {
    /// Raw deploy payload.
    pub deploy: DeployData,
    /// Execution cost.
    pub cost: PCost,
    /// Deploy log events.
    pub deploy_log: Vec<Event>,
    /// Payment results events.
    pub payments_results: Vec<Event>,
    /// Whether execution failed.
    pub is_failed: bool,
}

/// A processed system deploy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessedSystemDeploy {
    /// Raw deploy payload.
    pub deploy: DeployData,
    /// Execution cost.
    pub cost: PCost,
    /// Deploy log events.
    pub deploy_log: Vec<Event>,
    /// Whether execution failed.
    pub is_failed: bool,
}

/// The raw deploy submitted by a user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployData {
    /// Deployer public key.
    pub deployer: PublicKey,
    /// Rholang source code.
    pub term: String,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
    /// Signature over deploy payload.
    pub sig: Signature,
    /// Signature algorithm.
    pub sig_algorithm: String,
    /// Price per phlo unit.
    pub phlo_price: i64,
    /// Maximum phlo limit.
    pub phlo_limit: i64,
    /// Minimum block number after which deploy is valid.
    pub valid_after_block_number: i64,
    /// Shard identifier.
    pub shard_id: String,
}

/// Casper justification — links to latest messages from each validator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Justification {
    /// Validator public key.
    pub validator: PublicKey,
    /// Latest block hash by validator.
    pub latest_block_hash: BlockHash,
}

/// Validator bond information.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    /// Validator public key.
    pub validator: PublicKey,
    /// Stake amount.
    pub stake: i64,
}

/// State DAG validator info.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BondedValidatorInfo {
    /// Validator public key.
    pub validator: PublicKey,
    /// Stake amount.
    pub stake: i64,
}

/// Execution event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Event name.
    pub name: String,
    /// Event payload.
    pub payload: Vec<u8>,
}
