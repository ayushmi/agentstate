fn main() {
    println!("cargo:rerun-if-changed=../../proto/agentstate.proto");
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["../../proto/agentstate.proto"], &["../../proto"]) // paths relative to crate dir
        .unwrap();
}
