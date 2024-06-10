#!/usr/bin/env bash

set -e

cargo build --bin unique_ids
maelstrom test -w unique-ids --bin "target/debug/unique_ids" --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition
