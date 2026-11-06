#!/bin/sh
set -eu
root=$(git rev-parse --show-toplevel)
cd "$root"
current=$(git config --get core.hooksPath || :)
case "$current" in
    .githooks|"$root/.githooks") ;;
    '')
        directory=$(git rev-parse --git-path hooks)
        for hook in "$directory"/*; do
            case "$hook" in *.sample) continue ;; esac
            if [ -f "$hook" ] && [ -x "$hook" ]; then
                printf '%s\n' "Existing hook preserved: $hook. Integrate it before changing core.hooksPath." >&2
                exit 1
            fi
        done
        ;;
    *) printf '%s\n' "Existing core.hooksPath preserved: $current. Integrate it before installing project hooks." >&2; exit 1 ;;
esac
for hook in pre-commit commit-msg pre-push; do
    [ -f ".githooks/$hook" ] || { printf '%s\n' "Missing .githooks/$hook" >&2; exit 1; }
done
[ -r .githooks/commit-policy ] || { printf '%s\n' 'Missing .githooks/commit-policy' >&2; exit 1; }
chmod +x .githooks/pre-commit .githooks/commit-msg .githooks/pre-push
git config --local core.hooksPath .githooks
printf '%s\n' 'Installed project pre-commit, commit-msg and pre-push hooks: .githooks'
