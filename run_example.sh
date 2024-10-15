#!/bin/bash

export PATH="blast_core/bin:$PATH"
cd blast_example
cargo run -- $1
cd -