#!/bin/sh
exec python3 scripts/run-check.py cargo +stable clippy --all-targets --locked -- -D warnings
