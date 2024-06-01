#!/usr/bin/env bash

set -e

cargo build --bin broadcast_c

maelstrom test -w broadcast --bin "target/debug/broadcast_c" --node-count 5 --time-limit 20 --rate 10 --nemesis partition
