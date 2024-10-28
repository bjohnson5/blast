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

bitcoind -daemon -reindex
sleep 5
bitcoin-cli loadwallet test

# https://bitcoin.stackexchange.com/questions/101153/setting-the-fee-rate-on-regtest-or-quickly-generating-full-blocks
cont=true
smartfee=$(bitcoin-cli estimatesmartfee 6)
if [[ "$smartfee" == *"\"feerate\":"* ]]; then
    cont=false
fi
while $cont
do
    counterb=0
    range=$(( $RANDOM % 11 + 20 ))
    while [ $counterb -lt $range ]
    do
        power=$(( $RANDOM % 29 ))
        randfee=`echo "scale=8; 0.00001 * (1.1892 ^ $power)" | bc`
        newaddress=$(bitcoin-cli getnewaddress)
        rawtx=$(bitcoin-cli createrawtransaction "[]" "[{\"$newaddress\":0.005}]")
        fundedtx=$(bitcoin-cli fundrawtransaction "$rawtx" "{\"feeRate\": \"0$randfee\"}" | jq -r ".hex")
        signedtx=$(bitcoin-cli signrawtransactionwithwallet "$fundedtx" | jq -r ".hex")
        senttx=$(bitcoin-cli sendrawtransaction "$signedtx")
        ((++counterb))
        echo "Created $counterb transactions this block"
    done
    bitcoin-cli generatetoaddress 1 "mp76nrashrCCYLy3a8cAc5HufEas11yHbh"
    smartfee=$(bitcoin-cli estimatesmartfee 6)
    if [[ "$smartfee" == *"\"feerate\":"* ]]; then
        cont=false
    fi
done
bitcoin-cli generatetoaddress 6 "mp76nrashrCCYLy3a8cAc5HufEas11yHbh"
