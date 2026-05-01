//! F1r3fly block structure library.

pub mod builder;
pub mod genesis;
pub mod hashing;
pub mod serialization;
pub mod storage;
pub mod types;
pub mod validation;

mod proto;

pub use builder::{BlockBuildError, BlockBuilder, UnsignedBlock};
pub use genesis::{GenesisConfig, WalletEntry};
pub use hashing::{compute_block_hash, compute_bonds_map_hash, compute_deploy_hash, compute_header_hash, compute_post_state_hash};
pub use serialization::{BlockSerialize, SerializationError};
pub use storage::{BlockStore, InMemoryBlockStore, StoreError};
pub use types::*;
pub use validation::{BlockLookup, BlockValidator, ValidationError};
