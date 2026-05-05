use std::path::PathBuf;

fn main() {
    let _out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("Failed to locate vendored protoc");
    std::env::set_var("PROTOC", protoc);

    // All proto files live in ../proto (sibling of core/)
    let proto_root = PathBuf::from("..");

    // Services we're compiling
    let services = vec![
        "proto/common.proto",
        "proto/crypto.proto",
        "proto/battery.proto",
        "proto/auth.proto",
        "proto/lifecycle.proto",
    ];

    // Compile all protos together
    let mut proto_paths = Vec::new();
    for service in &services {
        proto_paths.push(proto_root.join(service));
    }

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&proto_paths, std::slice::from_ref(&proto_root))
        .expect("Failed to compile proto stubs");

    // Rerun if any proto file changes
    println!("cargo:rerun-if-changed=../proto");
    for service in services {
        println!("cargo:rerun-if-changed=../{}", service);
    }
}
