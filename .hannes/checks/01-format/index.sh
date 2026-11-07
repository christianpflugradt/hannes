#!/bin/sh
exec python3 scripts/run-check.py cargo +stable fmt --all -- --check
