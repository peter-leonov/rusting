#!/usr/bin/env bash

set -e

cargo build --bin broadcast_a
maelstrom test -w broadcast --bin "target/debug/broadcast_a" --node-count 1 --time-limit 20 --rate 10
