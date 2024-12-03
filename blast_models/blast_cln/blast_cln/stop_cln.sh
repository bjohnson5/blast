#!/bin/bash

test ! -f "$1/lightningd-$network.pid" || \
    (kill -9 "$(cat "$1/lightningd-$network.pid")"; \
    rm "$1/lightningd-$network.pid")
