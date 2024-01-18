#!/usr/bin/env bash

set -e

cargo build
maelstrom test -w echo --bin "target/debug/echo" --node-count 1 --time-limit 10
