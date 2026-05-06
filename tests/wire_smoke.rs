use f1r3fly_block_structure::{BlockMessage, BlockSerialize, Bond, GenesisConfig};

#[test]
fn genesis_wire_format_smoke_test() {
    let genesis = GenesisConfig {
        shard_id: "root".to_string(),
        validators: vec![Bond {
            validator: vec![1u8; 32],
            stake: 100,
        }],
        timestamp: 0,
    }
    .build_genesis_block()
    .expect("genesis");

    let bytes = genesis.to_proto_bytes().expect("serialize");
    let expected_hex = include_str!("fixtures/genesis_wire.hex").trim();
    assert_eq!(hex::encode(&bytes), expected_hex);

    let decoded = BlockMessage::from_proto_bytes(&bytes).expect("deserialize");
    assert_eq!(decoded, genesis);
}
