//! Serialization support for protobuf and JSON.

use prost::Message;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::proto::f1r3fly_block as proto;
use crate::types::*;

/// Errors returned during serialization or deserialization.
#[derive(Debug, Error)]
pub enum SerializationError {
    /// Protobuf encoding or decoding failed.
    #[error("Protobuf error: {0}")]
    Protobuf(String),
    /// JSON encoding or decoding failed.
    #[error("JSON error: {0}")]
    Json(String),
}

/// Serialization trait for block types.
pub trait BlockSerialize: Sized {
    /// Serialize to protobuf bytes.
    fn to_proto_bytes(&self) -> Result<Vec<u8>, SerializationError>;
    /// Deserialize from protobuf bytes.
    fn from_proto_bytes(bytes: &[u8]) -> Result<Self, SerializationError>;
    /// Serialize to JSON string.
    fn to_json(&self) -> Result<String, SerializationError>;
    /// Deserialize from JSON string.
    fn from_json(json: &str) -> Result<Self, SerializationError>;
}

impl BlockSerialize for BlockMessage {
    fn to_proto_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let proto = to_proto_block_message(self);
        let mut buf = Vec::new();
        proto.encode(&mut buf)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        Ok(buf)
    }

    fn from_proto_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        let proto = proto::BlockMessage::decode(bytes)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        from_proto_block_message(proto)
    }

    fn to_json(&self) -> Result<String, SerializationError> {
        serde_json::to_string(self).map_err(|e| SerializationError::Json(e.to_string()))
    }

    fn from_json(json: &str) -> Result<Self, SerializationError> {
        serde_json::from_str(json).map_err(|e| SerializationError::Json(e.to_string()))
    }
}

impl BlockSerialize for BlockHeader {
    fn to_proto_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let proto = to_proto_header(self);
        let mut buf = Vec::new();
        proto.encode(&mut buf)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        Ok(buf)
    }

    fn from_proto_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        let proto = proto::BlockHeader::decode(bytes)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        Ok(from_proto_header(proto))
    }

    fn to_json(&self) -> Result<String, SerializationError> {
        serde_json::to_string(self).map_err(|e| SerializationError::Json(e.to_string()))
    }

    fn from_json(json: &str) -> Result<Self, SerializationError> {
        serde_json::from_str(json).map_err(|e| SerializationError::Json(e.to_string()))
    }
}

impl BlockSerialize for DeployData {
    fn to_proto_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        let proto = to_proto_deploy(self);
        let mut buf = Vec::new();
        proto.encode(&mut buf)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        Ok(buf)
    }

    fn from_proto_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        let proto = proto::DeployData::decode(bytes)
            .map_err(|e| SerializationError::Protobuf(e.to_string()))?;
        Ok(from_proto_deploy(proto))
    }

    fn to_json(&self) -> Result<String, SerializationError> {
        serde_json::to_string(self).map_err(|e| SerializationError::Json(e.to_string()))
    }

    fn from_json(json: &str) -> Result<Self, SerializationError> {
        serde_json::from_str(json).map_err(|e| SerializationError::Json(e.to_string()))
    }
}

fn to_proto_block_message(block: &BlockMessage) -> proto::BlockMessage {
    proto::BlockMessage {
        block_hash: block.block_hash.to_vec(),
        header: Some(to_proto_header(&block.header)),
        body: Some(to_proto_body(&block.body)),
        justifications: block.justifications.iter().map(to_proto_justification).collect(),
        sender: block.sender.clone(),
        sig: block.sig.clone(),
        sig_algorithm: block.sig_algorithm.clone(),
        shard_id: block.shard_id.clone(),
        extra_bytes: block.extra_bytes.clone(),
    }
}

fn from_proto_block_message(proto: proto::BlockMessage) -> Result<BlockMessage, SerializationError> {
    let header = proto
        .header
        .ok_or_else(|| SerializationError::Protobuf("Missing header".to_string()))?;
    let body = proto
        .body
        .ok_or_else(|| SerializationError::Protobuf("Missing body".to_string()))?;

    Ok(BlockMessage {
        block_hash: bytes_to_hash(&proto.block_hash),
        header: from_proto_header(header),
        body: from_proto_body(body),
        justifications: proto.justifications.into_iter().map(from_proto_justification).collect(),
        sender: proto.sender,
        sig: proto.sig,
        sig_algorithm: proto.sig_algorithm,
        shard_id: proto.shard_id,
        extra_bytes: proto.extra_bytes,
    })
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

fn from_proto_header(proto: proto::BlockHeader) -> BlockHeader {
    BlockHeader {
        parents_hash_list: proto
            .parents_hash_list
            .into_iter()
            .map(|bytes| bytes_to_hash(&bytes))
            .collect(),
        post_state_hash: bytes_to_hash(&proto.post_state_hash),
        bonds_map_hash: bytes_to_hash(&proto.bonds_map_hash),
        state_dag_hash: bytes_to_hash(&proto.state_dag_hash),
        deploy_count: proto.deploy_count,
        timestamp: proto.timestamp,
        version: proto.version,
        seq_num: proto.seq_num,
        shard_id: proto.shard_id,
    }
}

fn to_proto_body(body: &BlockBody) -> proto::BlockBody {
    proto::BlockBody {
        deploys: body.deploys.iter().map(to_proto_processed_deploy).collect(),
        system_deploys: body.system_deploys.iter().map(to_proto_processed_system_deploy).collect(),
        state_dag: body.state_dag.iter().map(to_proto_bonded_validator_info).collect(),
    }
}

fn from_proto_body(proto: proto::BlockBody) -> BlockBody {
    BlockBody {
        deploys: proto.deploys.into_iter().map(from_proto_processed_deploy).collect(),
        system_deploys: proto
            .system_deploys
            .into_iter()
            .map(from_proto_processed_system_deploy)
            .collect(),
        state_dag: proto.state_dag.into_iter().map(from_proto_bonded_validator_info).collect(),
    }
}

fn to_proto_processed_deploy(deploy: &ProcessedDeploy) -> proto::ProcessedDeploy {
    proto::ProcessedDeploy {
        deploy: Some(to_proto_deploy(&deploy.deploy)),
        cost: deploy.cost,
        deploy_log: deploy.deploy_log.iter().map(to_proto_event).collect(),
        payments_results: deploy.payments_results.iter().map(to_proto_event).collect(),
        is_failed: deploy.is_failed,
    }
}

fn from_proto_processed_deploy(proto: proto::ProcessedDeploy) -> ProcessedDeploy {
    ProcessedDeploy {
        deploy: from_proto_deploy(proto.deploy.unwrap_or_default()),
        cost: proto.cost,
        deploy_log: proto.deploy_log.into_iter().map(from_proto_event).collect(),
        payments_results: proto.payments_results.into_iter().map(from_proto_event).collect(),
        is_failed: proto.is_failed,
    }
}

fn to_proto_processed_system_deploy(deploy: &ProcessedSystemDeploy) -> proto::ProcessedSystemDeploy {
    proto::ProcessedSystemDeploy {
        deploy: Some(to_proto_deploy(&deploy.deploy)),
        cost: deploy.cost,
        deploy_log: deploy.deploy_log.iter().map(to_proto_event).collect(),
        is_failed: deploy.is_failed,
    }
}

fn from_proto_processed_system_deploy(proto: proto::ProcessedSystemDeploy) -> ProcessedSystemDeploy {
    ProcessedSystemDeploy {
        deploy: from_proto_deploy(proto.deploy.unwrap_or_default()),
        cost: proto.cost,
        deploy_log: proto.deploy_log.into_iter().map(from_proto_event).collect(),
        is_failed: proto.is_failed,
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

fn from_proto_deploy(proto: proto::DeployData) -> DeployData {
    DeployData {
        deployer: proto.deployer,
        term: proto.term,
        timestamp: proto.timestamp,
        sig: proto.sig,
        sig_algorithm: proto.sig_algorithm,
        phlo_price: proto.phlo_price,
        phlo_limit: proto.phlo_limit,
        valid_after_block_number: proto.valid_after_block_number,
        shard_id: proto.shard_id,
    }
}

fn to_proto_justification(just: &Justification) -> proto::Justification {
    proto::Justification {
        validator: just.validator.clone(),
        latest_block_hash: just.latest_block_hash.to_vec(),
    }
}

fn from_proto_justification(proto: proto::Justification) -> Justification {
    Justification {
        validator: proto.validator,
        latest_block_hash: bytes_to_hash(&proto.latest_block_hash),
    }
}

fn to_proto_bonded_validator_info(info: &BondedValidatorInfo) -> proto::BondedValidatorInfo {
    proto::BondedValidatorInfo {
        validator: info.validator.clone(),
        stake: info.stake,
    }
}

fn from_proto_bonded_validator_info(proto: proto::BondedValidatorInfo) -> BondedValidatorInfo {
    BondedValidatorInfo {
        validator: proto.validator,
        stake: proto.stake,
    }
}

fn to_proto_event(event: &Event) -> proto::Event {
    proto::Event {
        name: event.name.clone(),
        payload: event.payload.clone(),
    }
}

fn from_proto_event(proto: proto::Event) -> Event {
    Event {
        name: proto.name,
        payload: proto.payload,
    }
}

fn bytes_to_hash(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let copy_len = bytes.len().min(32);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

#[derive(Serialize, Deserialize)]
struct _SerdeMarker;
