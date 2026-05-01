fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("Locate protoc");
    std::env::set_var("PROTOC", protoc);

    prost_build::Config::new()
        .compile_protos(&["proto/block.proto"], &["proto"])
        .expect("Failed to compile protobufs");
}
