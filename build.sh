#!/bin/bash

# Build the models
cd blast_models/blast_lnd
./build.sh
cd -

cd blast_models/blast_cln
./build.sh
cd -

cd blast_models/blast_ldk
./build.sh
cd -

# Build the CLI
cd blast_cli
cargo build
cd -

# Build the Web UI
cd blast_web
cargo build
cd -

