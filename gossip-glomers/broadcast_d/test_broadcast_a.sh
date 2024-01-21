#!/usr/bin/env bash

set -e

{
  echo '{"src":"p1", "dest": "n1", "body":{"type":"init", "msg_id": 1, "node_id": "n1", "node_ids": []}}'
  echo '{"src":"p1", "dest": "n1", "body":{"type": "broadcast", "msg_id": 2, "message": 1000}}'
  echo '{"src":"n1", "dest": "n1", "body":{"type": "read", "msg_id": 3}}'
 } | cargo run --bin broadcast_a
