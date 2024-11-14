use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut current_dir = env::current_dir()?;
    current_dir.push("../../../blast_proto/blast_proto.proto");
    let current_dir_string = current_dir.to_string_lossy().into_owned();
    tonic_build::compile_protos(current_dir_string)?;

    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    let home = env::var("HOME").expect("HOME environment variable not set");
	let proto_path = PathBuf::from(home.clone()).join(".blast/lightning/cln-grpc/proto/node.proto");
    let folder_path = PathBuf::from(home.clone()).join(".blast/lightning/cln-grpc/proto");
    tonic_build::configure().compile_protos_with_config(config, &[proto_path], &[folder_path])?;

    Ok(())
}
