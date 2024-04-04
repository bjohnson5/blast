#!/bin/bash

# Clean the models
cd blast_models/blast_lnd
./clean.sh
cd -

cd blast_models/blast_cln
./clean.sh
cd -

cd blast_models/blast_ldk
./clean.sh
cd -

# Clean the CLI
cd blast_cli
cargo clean
rm Cargo.lock
cd -

# Clean the core library
cd blast_core
cargo clean
rm Cargo.lock
cd -

# Clean the model interface
cd blast_model_interface
cargo clean
rm Cargo.lock
cd -

# Clean the Web UI
cd blast_web
cargo clean
rm Cargo.lock
cd -



