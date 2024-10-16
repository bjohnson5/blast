#!/bin/bash

bitcoin-cli stop
rm -rf ~/.bitcoin/regtest
rm ~/.bitcoin/bitcoin.conf

if [ -f electrs_id.txt ]; then
    PID=$(cat electrs_id.txt)
    kill -9 $PID
    rm electrs_id.txt
fi
rm -rf ~/.electrs
