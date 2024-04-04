use std::env;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut current_dir = env::current_dir()?;
    current_dir.push("../blast_proto/blast_proto.proto");
    let current_dir_string = current_dir.to_string_lossy().into_owned();
    tonic_build::compile_protos(current_dir_string)?;
    Ok(())
}