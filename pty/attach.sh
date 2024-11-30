#!/usr/bin/env bash

tty_state="$(stty -g)"
stty raw -echo
nc localhost 12345
stty "$tty_state"
