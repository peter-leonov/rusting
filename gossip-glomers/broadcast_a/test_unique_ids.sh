#!/usr/bin/env bash

set -e

{
  echo '{"src":"p1", "dest": "n1", "body":{"type":"init", "msg_id": 1, "node_id": "n1", "node_ids": []}}'
  echo '{"src":"n1", "dest": "n2", "body":{"type":"generate", "msg_id": 1}}'
 } | cargo run --bin unique_ids
