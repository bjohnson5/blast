#!/bin/bash
echo "server=1" > ~/.bitcoin/bitcoin.conf
echo "regtest=1" >> ~/.bitcoin/bitcoin.conf
echo "rpcuser=user" >> ~/.bitcoin/bitcoin.conf
echo "rpcpassword=pass" >> ~/.bitcoin/bitcoin.conf
echo "zmqpubrawblock=tcp://127.0.0.1:28332" >> ~/.bitcoin/bitcoin.conf
echo "zmqpubrawtx=tcp://127.0.0.1:28333" >> ~/.bitcoin/bitcoin.conf
echo "blockfilterindex=1" >> ~/.bitcoin/bitcoin.conf
echo "peerblockfilters=1" >> ~/.bitcoin/bitcoin.conf
echo "fallbackfee=0.00001" >> ~/.bitcoin/bitcoin.conf
echo "[regtest]" >> ~/.bitcoin/bitcoin.conf
echo "rpcport=18443" >> ~/.bitcoin/bitcoin.conf

bitcoind -daemon
sleep 5
bitcoin-cli createwallet test
bitcoin-cli -generate 101
