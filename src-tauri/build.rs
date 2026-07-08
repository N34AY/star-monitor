fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to find vendored protoc");
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(false)
        .include_file("mod.rs")
        .compile_protos(
            &["proto/starlink.proto"],
            &["proto"],
        )
        .expect("failed to compile Starlink protobuf files");

    tauri_build::build()
}
