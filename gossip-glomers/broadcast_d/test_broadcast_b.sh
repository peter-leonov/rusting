#!/usr/bin/env bash

set -e

{
  echo '{"src":"p1", "dest": "n1", "body":{"type":"init", "msg_id": 1, "node_id": "n1", "node_ids": ["n2","n3","n4","n5","n6","n7","n8","n9", "n10"]}}'
  echo '{"src":"p1", "dest": "n1", "body":{"type": "broadcast", "msg_id": 2, "message": 1000}}'
  echo '{"src":"p1", "dest": "n1", "body":{"type": "read", "msg_id": 3}}'
  echo '{"src":"n2", "dest": "n1", "body":{"type": "gossip", "msg_id": 3, "messages": [2000, 3000], "nodes": ["n2","n3","n4"]}}'
 } | cargo run --bin broadcast_b
