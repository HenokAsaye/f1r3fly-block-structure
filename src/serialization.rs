
use prost::Message;
use thiserror::Error;

use crate::proto::block as proto;
use crate::types::*;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Protobuf error: {0}")]
    Protobuf(String),
    #[error("JSON error: {0}")]
    Json(String),
}

pub trait BlockSerialize: Sized {
    fn to_proto_bytes(&self) -> Result<Vec<u8>, SerializationError>;
    fn from_proto_bytes(bytes: &[u8]) -> Result<Self, SerializationError>;
    fn to_json(&self) -> Result<String, SerializationError>;
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
        justifications: block
            .justifications
            .iter()
            .map(to_proto_justification)
            .collect(),
        sender: block.sender.clone(),
        seq_num: block.seq_num,
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
        seq_num: proto.seq_num,
        sig: proto.sig,
        sig_algorithm: proto.sig_algorithm,
        shard_id: proto.shard_id,
        extra_bytes: proto.extra_bytes,
    })
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
            .map(to_proto_justification)
            .collect(),
    }
}

fn from_proto_header(proto: proto::BlockHeader) -> BlockHeader {
    BlockHeader {
        parents: proto
            .parents
            .into_iter()
            .map(|bytes| bytes_to_hash(&bytes))
            .collect(),
        sender: proto.sender,
        sig_algorithm: proto.sig_algorithm,
        sig: proto.sig,
        shard_id: proto.shard_id,
        seq_num: proto.seq_num,
        version: proto.version,
        body_hash: bytes_to_hash(&proto.body_hash),
        block_hash: bytes_to_hash(&proto.block_hash),
        dag_level: proto.dag_level,
        justifications: proto
            .justifications
            .into_iter()
            .map(from_proto_justification)
            .collect(),
    }
}

fn to_proto_body(body: &BlockBody) -> proto::BlockBody {
    proto::BlockBody {
        deploys: body.deploys.iter().map(to_proto_processed_deploy).collect(),
        system_deploys: body.system_deploys.iter().map(to_proto_processed_system_deploy).collect(),
        state: Some(to_proto_state(&body.state)),
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
        state: from_proto_state(proto.state.unwrap_or_default()),
    }
}

fn to_proto_processed_deploy(deploy: &ProcessedDeploy) -> proto::ProcessedDeploy {
    proto::ProcessedDeploy {
        deploy: Some(to_proto_deploy(&deploy.deploy)),
        cost: Some(to_proto_pcost(&deploy.cost)),
        deploy_log: deploy.deploy_log.iter().map(to_proto_event).collect(),
        payments_results: deploy.payments_results.iter().map(to_proto_event).collect(),
        is_failed: deploy.is_failed,
        system_deploy_error: deploy.system_deploy_error.clone(),
    }
}

fn from_proto_processed_deploy(proto: proto::ProcessedDeploy) -> ProcessedDeploy {
    ProcessedDeploy {
        deploy: from_proto_deploy(proto.deploy.unwrap_or_default()),
        cost: from_proto_pcost(proto.cost.unwrap_or_default()),
        deploy_log: proto.deploy_log.into_iter().map(from_proto_event).collect(),
        payments_results: proto.payments_results.into_iter().map(from_proto_event).collect(),
        is_failed: proto.is_failed,
        system_deploy_error: proto.system_deploy_error,
    }
}

fn to_proto_processed_system_deploy(deploy: &ProcessedSystemDeploy) -> proto::ProcessedSystemDeploy {
    let system_deploy = match deploy {
        ProcessedSystemDeploy::CloseBlockDeploy(_) => {
            Some(proto::processed_system_deploy::SystemDeploy::CloseBlockDeploy(
                proto::CloseBlockDeploy {},
            ))
        }
        ProcessedSystemDeploy::SlashSystemDeploy(slash) => Some(
            proto::processed_system_deploy::SystemDeploy::SlashSystemDeploy(
                proto::SlashSystemDeploy {
                    invalid_block_hash: slash.invalid_block_hash.clone(),
                    issuer_public_key: slash.issuer_public_key.clone(),
                },
            ),
        ),
    };
    proto::ProcessedSystemDeploy { system_deploy }
}

fn from_proto_processed_system_deploy(proto: proto::ProcessedSystemDeploy) -> ProcessedSystemDeploy {
    match proto.system_deploy {
        Some(proto::processed_system_deploy::SystemDeploy::CloseBlockDeploy(_)) => {
            ProcessedSystemDeploy::CloseBlockDeploy(CloseBlockDeploy {})
        }
        Some(proto::processed_system_deploy::SystemDeploy::SlashSystemDeploy(slash)) => {
            ProcessedSystemDeploy::SlashSystemDeploy(SlashSystemDeploy {
                invalid_block_hash: slash.invalid_block_hash,
                issuer_public_key: slash.issuer_public_key,
            })
        }
        None => ProcessedSystemDeploy::CloseBlockDeploy(CloseBlockDeploy {}),
    }
}

fn to_proto_pcost(cost: &PCost) -> proto::PCost {
    proto::PCost { cost: cost.cost }
}

fn from_proto_pcost(proto: proto::PCost) -> PCost {
    PCost { cost: proto.cost }
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

fn to_proto_event(event: &Event) -> proto::Event {
    let event_instance = match event {
        Event::Produce(produce) => Some(proto::event::EventInstance::Produce(to_proto_produce(produce))),
        Event::Consume(consume) => Some(proto::event::EventInstance::Consume(to_proto_consume(consume))),
    };
    proto::Event { event_instance }
}

fn from_proto_event(proto: proto::Event) -> Event {
    match proto.event_instance {
        Some(proto::event::EventInstance::Produce(produce)) => Event::Produce(from_proto_produce(produce)),
        Some(proto::event::EventInstance::Consume(consume)) => Event::Consume(from_proto_consume(consume)),
        None => Event::Produce(ProduceEvent {
            channel_hash: Vec::new(),
            data: Vec::new(),
            persistent: false,
        }),
    }
}

fn to_proto_produce(event: &ProduceEvent) -> proto::ProduceEvent {
    proto::ProduceEvent {
        channel_hash: event.channel_hash.clone(),
        data: event.data.clone(),
        persistent: event.persistent,
    }
}

fn from_proto_produce(proto: proto::ProduceEvent) -> ProduceEvent {
    ProduceEvent {
        channel_hash: proto.channel_hash,
        data: proto.data,
        persistent: proto.persistent,
    }
}

fn to_proto_consume(event: &ConsumeEvent) -> proto::ConsumeEvent {
    proto::ConsumeEvent {
        channel_hashes: event.channel_hashes.clone(),
        data: event.data.clone(),
        persistent: event.persistent,
    }
}

fn from_proto_consume(proto: proto::ConsumeEvent) -> ConsumeEvent {
    ConsumeEvent {
        channel_hashes: proto.channel_hashes,
        data: proto.data,
        persistent: proto.persistent,
    }
}

fn to_proto_state(state: &RChainState) -> proto::RChainState {
    proto::RChainState {
        pre_state_hash: state.pre_state_hash.to_vec(),
        post_state_hash: state.post_state_hash.to_vec(),
        bonds: state.bonds.iter().map(to_proto_bond).collect(),
        block_number: state.block_number,
    }
}

fn from_proto_state(proto: proto::RChainState) -> RChainState {
    RChainState {
        pre_state_hash: bytes_to_hash(&proto.pre_state_hash),
        post_state_hash: bytes_to_hash(&proto.post_state_hash),
        bonds: proto.bonds.into_iter().map(from_proto_bond).collect(),
        block_number: proto.block_number,
    }
}

fn to_proto_bond(bond: &Bond) -> proto::Bond {
    proto::Bond {
        validator: bond.validator.clone(),
        stake: bond.stake,
    }
}

fn from_proto_bond(proto: proto::Bond) -> Bond {
    Bond {
        validator: proto.validator,
        stake: proto.stake,
    }
}

fn bytes_to_hash(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let copy_len = bytes.len().min(32);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

