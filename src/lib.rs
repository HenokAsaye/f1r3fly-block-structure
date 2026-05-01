//! F1r3fly block structure library.

pub mod casper;
pub mod builder;
pub mod genesis;
pub mod hashing;
pub mod serialization;
pub mod storage;
pub mod types;
pub mod validation;

mod proto;

pub use builder::{BlockBuildError, BlockBuilder, UnsignedBlock};
pub use casper::GhostForkChoice;
pub use genesis::{ConfigError, GenesisConfig};
pub use hashing::{
	compute_block_hash, compute_bonds_map_hash, compute_deploy_hash, compute_post_state_hash,
};
pub use serialization::{BlockSerialize, SerializationError};
pub use storage::{BlockStore, InMemoryBlockStore, StoreError};
pub use types::{
	BlockBody, BlockHash, BlockHeader, BlockMessage, Bond, BondedValidatorInfo, BondsHash,
	CommEvent, ConsumeEvent, DeployData, Event, Justification, PCost, ProcessedDeploy,
	ProcessedSystemDeploy, ProduceEvent, PublicKey, Signature, StateHash, StateDagHash,
	SystemDeploy,
};
pub use validation::{BlockLookup, BlockValidator, ValidationError};
