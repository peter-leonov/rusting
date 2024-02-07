#!/usr/bin/env bash

set -e

cargo build --bin broadcast_d

maelstrom test -w broadcast --bin "target/debug/broadcast_d" --node-count 25 --time-limit 20 --rate 100 --latency 100
