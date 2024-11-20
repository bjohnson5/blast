#!/bin/bash

test ! -f "$1/lightningd-$network.pid" || \
    (kill "$(cat "$1/lightningd-$network.pid")"; \
    rm "$1/lightningd-$network.pid")

rm -rf "$HOME/.blast/clightning/sockets"
