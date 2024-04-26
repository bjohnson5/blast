#!/bin/bash

bitcoin-cli stop
rm -rf ~/.bitcoin/regtest
rm -rf blast_models/blast_lnd/blast_data
