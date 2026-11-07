"""Preserve offline Git-hook execution, then launch the configured check command."""
import os
import sys

if len(sys.argv) < 2:
    print("Usage: python3 scripts/run-check.py COMMAND [ARG ...]", file=sys.stderr)
    sys.exit(2)
if os.environ.get("HANNES_GIT_HOOK") in ("pre-commit", "commit-msg", "pre-push"):
    os.environ.update(CARGO_NET_OFFLINE="true", npm_config_offline="true", RUSTUP_AUTO_INSTALL="0")
try:
    os.execvpe(sys.argv[1], sys.argv[1:], os.environ)
except OSError as error:
    print(f"Cannot start check command {sys.argv[1]!r}: {error}", file=sys.stderr)
    sys.exit(2)
