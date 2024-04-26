#!/bin/bash

cp bitcoin.conf ~/.bitcoin/
bitcoind -daemon
sleep 5
bitcoin-cli createwallet test
bitcoin-cli -generate 200