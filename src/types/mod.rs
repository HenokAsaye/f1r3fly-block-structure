use serde::{Deserialize, Serialize};

pub type BlockHash = [u8; 32];
pub type BodyHash = [u8; 32];
pub type StateHash = [u8; 32];
pub type PublicKey = Vec<u8>;
pub type Signature = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMessage {

    pub block_hash: BlockHash,
    pub header: BlockHeader,
    pub body: BlockBody,
    pub justifications: Vec<Justification>,
    pub sender: PublicKey,
    pub seq_num: i64,
    pub sig: Signature,
    pub sig_algorithm: String,
    pub shard_id: String,
    pub extra_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parents: Vec<BlockHash>,
    pub sender: PublicKey,
    pub sig_algorithm: String,
    pub sig: Signature,
    pub shard_id: String,
    pub seq_num: i64,
    pub version: i32,
    pub body_hash: BodyHash,
    pub block_hash: BlockHash,
    pub dag_level: i64,
    pub justifications: Vec<Justification>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBody {
    pub deploys: Vec<ProcessedDeploy>,
    pub system_deploys: Vec<ProcessedSystemDeploy>,
    pub state: RChainState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RChainState {
    pub pre_state_hash: StateHash,
    pub post_state_hash: StateHash,
    pub bonds: Vec<Bond>,
    pub block_number: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessedDeploy {
    pub deploy: DeployData,
    pub cost: PCost,
    pub deploy_log: Vec<Event>,
    pub payments_results: Vec<Event>,
    pub is_failed: bool,
    pub system_deploy_error: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessedSystemDeploy {
    CloseBlockDeploy(CloseBlockDeploy),
    SlashSystemDeploy(SlashSystemDeploy),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CloseBlockDeploy {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashSystemDeploy {
    pub invalid_block_hash: Vec<u8>,
    pub issuer_public_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PCost {
    pub cost: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployData {
    pub deployer: PublicKey,
    pub term: String,
    pub timestamp: i64,
    pub sig: Signature,
    pub sig_algorithm: String,
    pub phlo_price: i64,
    pub phlo_limit: i64,
    pub valid_after_block_number: i64,
    pub shard_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub validator: PublicKey,
    pub stake: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Justification {
    pub validator: PublicKey,
    pub latest_block_hash: BlockHash,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Produce(ProduceEvent),
    Consume(ConsumeEvent),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProduceEvent {
    pub channel_hash: Vec<u8>,
    pub data: Vec<u8>,
    pub persistent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumeEvent {
    pub channel_hashes: Vec<Vec<u8>>,
    pub data: Vec<u8>,
    pub persistent: bool,
}
