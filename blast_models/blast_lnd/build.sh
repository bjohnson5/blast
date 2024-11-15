#!/bin/bash
set -e

# Copy admin.macaroon to blast data dir
cp sample_admin.macaroon "$HOME/.blast/admin.macaroon"

# Build protobuf
protoc --go_out=. --go-grpc_out=. --proto_path=../../blast_proto/ blast_proto.proto

# Build the model
go build
