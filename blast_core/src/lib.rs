use std::env;

use std::process::{Command, Child};

pub fn create_blast_lnd_nodes(num_nodes: u64) -> Option<Child> {
    let mut current_dir = match env::current_dir() {
        Ok(d) => d,
        Err(_) => {
            println!("Failed to get the current directory");
            return None;
        }
    };

    current_dir.push("../blast_models/blast_lnd/blast_lnd");
    let current_dir_string = current_dir.to_string_lossy().into_owned();

    println!("{}", current_dir_string);

    let child = Command::new(current_dir_string)
        .arg(&num_nodes.to_string())
        .spawn()
        .expect("Failed to execute process");

    Some(child)
}
