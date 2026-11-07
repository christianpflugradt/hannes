#!/bin/sh
exec python3 scripts/run-check.py cargo +1.85.0 check --all-targets --locked
