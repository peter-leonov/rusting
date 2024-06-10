#!/usr/bin/env bash

set -e

cargo build --bin broadcast_b

maelstrom test -w broadcast --bin "target/debug/broadcast_b" --node-count 5 --time-limit 20 --rate 10
