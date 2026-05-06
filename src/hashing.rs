
use blake2::{digest::consts::U32, Blake2b, Digest};
use prost::Message;

use crate::proto::block as proto;
use crate::types::{BlockBody, BlockHash, BlockHeader, Bond, DeployData, StateHash};

pub fn compute_block_hash(header: &BlockHeader) -> BlockHash {
    let mut hashed_header = header.clone();
    hashed_header.sig.clear();
    hashed_header.block_hash = [0u8; 32];
    let proto_header = to_proto_header(&hashed_header);
    hash_bytes(&proto_header.encode_to_vec())
}

pub fn compute_body_hash(body: &BlockBody) -> [u8; 32] {
    let proto_body = to_proto_body(body);
    hash_bytes(&proto_body.encode_to_vec())
}

pub fn compute_deploy_hash(deploy: &DeployData) -> [u8; 32] {
    let proto_deploy = to_proto_deploy(deploy);
    let mut buf = Vec::new();
    proto_deploy.encode(&mut buf).unwrap_or_default();
    hash_bytes(&buf)
}

pub fn compute_bonds_map_hash(bonds: &[Bond]) -> [u8; 32] {
    let mut sorted = bonds.to_vec();
    sorted.sort_by(|a, b| a.validator.cmp(&b.validator));

    let mut bytes = Vec::new();
    for bond in sorted {
        bytes.extend_from_slice(&(bond.validator.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&bond.validator);
        bytes.extend_from_slice(&bond.stake.to_le_bytes());
    }
    hash_bytes(&bytes)
}

pub fn compute_post_state_hash(state_root: &[u8]) -> StateHash {
    hash_bytes(state_root)
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

fn to_proto_header(header: &BlockHeader) -> proto::BlockHeader {
    proto::BlockHeader {
        parents: header.parents.iter().map(|h| h.to_vec()).collect(),
        sender: header.sender.clone(),
        sig_algorithm: header.sig_algorithm.clone(),
        sig: header.sig.clone(),
        shard_id: header.shard_id.clone(),
        seq_num: header.seq_num,
        version: header.version,
        body_hash: header.body_hash.to_vec(),
        block_hash: header.block_hash.to_vec(),
        dag_level: header.dag_level,
        justifications: header
            .justifications
            .iter()
            .map(|j| proto::Justification {
                validator: j.validator.clone(),
                latest_block_hash: j.latest_block_hash.to_vec(),
            })
            .collect(),
    }
}

fn to_proto_body(body: &BlockBody) -> proto::BlockBody {
    proto::BlockBody {
        deploys: body
            .deploys
            .iter()
            .map(|d| proto::ProcessedDeploy {
                deploy: Some(to_proto_deploy(&d.deploy)),
                cost: Some(proto::PCost { cost: d.cost.cost }),
                deploy_log: Vec::new(),
                payments_results: Vec::new(),
                is_failed: d.is_failed,
                system_deploy_error: d.system_deploy_error.clone(),
            })
            .collect(),
        system_deploys: Vec::new(),
        state: Some(proto::RChainState {
            pre_state_hash: body.state.pre_state_hash.to_vec(),
            post_state_hash: body.state.post_state_hash.to_vec(),
            bonds: body
                .state
                .bonds
                .iter()
                .map(|b| proto::Bond {
                    validator: b.validator.clone(),
                    stake: b.stake,
                })
                .collect(),
            block_number: body.state.block_number,
        }),
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
