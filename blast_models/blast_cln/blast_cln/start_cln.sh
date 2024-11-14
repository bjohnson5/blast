#!/bin/bash

socket=$1
socketrpc=$2
LIGHTNING_DIR=$3
BITCOIN_DIR="$HOME/.bitcoin"
CLND="$HOME/.blast/lightning/lightningd/lightningd"
mkdir -p "$LIGHTNING_DIR"
cat <<- EOF > "$LIGHTNING_DIR/config"
network=regtest
log-level=debug
log-file=$LIGHTNING_DIR/log
addr=localhost:$socket
allow-deprecated-apis=false
developer
dev-fast-gossip
dev-bitcoind-poll=5
experimental-dual-fund
experimental-splicing
experimental-offers
funder-policy=match
funder-policy-mod=100
funder-min-their-funding=10000
funder-per-channel-max=100000
funder-fuzz-percent=0
funder-lease-requests-only=false
lease-fee-base-sat=2sat
lease-fee-basis=50
invoices-onchain-fallback
EOF

test -f "$LIGHTNING_DIR/lightningd-regtest.pid" || \
    "$CLND" "--network=regtest" "--lightning-dir=$LIGHTNING_DIR" "--bitcoin-datadir=$BITCOIN_DIR" "--database-upgrade=true" "--grpc-port=$socketrpc"&
