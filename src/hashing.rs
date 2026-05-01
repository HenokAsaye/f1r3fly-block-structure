//! Deterministic hashing utilities.

use blake2::{Blake2b512, Digest};
use prost::Message;

use crate::proto::f1r3fly_block as proto;
use crate::types::{BlockHash, BlockHeader, BondsHash, Bond, DeployData, StateHash};

/// Compute the block hash (Blake2b-256) from the header.
pub fn compute_block_hash(header: &BlockHeader) -> BlockHash {
    compute_header_hash(header)
}

/// Compute the header hash (Blake2b-256) from the header.
pub fn compute_header_hash(header: &BlockHeader) -> [u8; 32] {
    let bytes = serialize_header_for_hashing(header);
    hash_bytes(&bytes)
}

/// Compute the deploy hash (Blake2b-256) from a deploy.
pub fn compute_deploy_hash(deploy: &DeployData) -> [u8; 32] {
    let proto_deploy = to_proto_deploy(deploy);
    let mut buf = Vec::new();
    proto_deploy.encode(&mut buf).unwrap_or_default();
    hash_bytes(&buf)
}

/// Compute the bonds map hash (Blake2b-256) from bonds list.
pub fn compute_bonds_map_hash(bonds: &[Bond]) -> BondsHash {
    let mut sorted = bonds.to_vec();
    sorted.sort_by(|a, b| a.validator.cmp(&b.validator));
    let proto_bonds = proto::BondsMap {
        bonds: sorted.into_iter().map(to_proto_bond).collect(),
    };
    let mut buf = Vec::new();
    proto_bonds.encode(&mut buf).unwrap_or_default();
    hash_bytes(&buf)
}

/// Compute post-state hash (Blake2b-256) from raw state root bytes.
pub fn compute_post_state_hash(state_root: &[u8]) -> StateHash {
    hash_bytes(state_root)
}

/// Serialize header fields in canonical order before hashing.
fn serialize_header_for_hashing(header: &BlockHeader) -> Vec<u8> {
    let proto_header = to_proto_header(header);
    let mut buf = Vec::new();
    proto_header.encode(&mut buf).unwrap_or_default();
    buf
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..32]);
    out
}

fn to_proto_header(header: &BlockHeader) -> proto::BlockHeader {
    proto::BlockHeader {
        parents_hash_list: header.parents_hash_list.iter().map(|h| h.to_vec()).collect(),
        post_state_hash: header.post_state_hash.to_vec(),
        bonds_map_hash: header.bonds_map_hash.to_vec(),
        state_dag_hash: header.state_dag_hash.to_vec(),
        deploy_count: header.deploy_count,
        timestamp: header.timestamp,
        version: header.version,
        seq_num: header.seq_num,
        shard_id: header.shard_id.clone(),
    }
}

fn to_proto_deploy(deploy: &DeployData) -> proto::DeployData {
    proto::DeployData {
        deployer: deploy.deployer.clone(),
        term: deploy.term.clone(),
        timestamp: deploy.timestamp,
        sig: deploy.sig.clone(),
        sig_algorithm: deploy.sig_algorithm.clone(),
        phlo_price: deploy.phlo_price,
        phlo_limit: deploy.phlo_limit,
        valid_after_block_number: deploy.valid_after_block_number,
        shard_id: deploy.shard_id.clone(),
    }
}

fn to_proto_bond(bond: Bond) -> proto::Bond {
    proto::Bond {
        validator: bond.validator,
        stake: bond.stake,
    }
}
