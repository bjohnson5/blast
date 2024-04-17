#!/bin/bash

protoc --go_out=. --go-grpc_out=. --proto_path=../../blast_proto/ blast_proto.proto
go build
