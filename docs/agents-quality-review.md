# AGENTS.md quality review

The repository demonstrates a conditional document review using ordinary Hannes
Rules. A blocking size Check runs first; a non-blocking semantic Review then
checks whether root AGENTS.md provides usable guidance for coding agents.

## Size and evaluation

The [size Rule](../.hannes/checks/04-agents-size/rule.toml) runs at pre-commit,
pre-push and check. Its command passes the explicit limit **1200** to
[scripts/check-word-count.py](../scripts/check-word-count.py). A word is a maximal
run separated by ASCII whitespace: space, tab, LF, CR, vertical tab or form feed.
Markdown punctuation, headings and code count literally; non-ASCII spaces do not
split words. The original 1050-word document has about 14% room for growth.
Exactly 1200 passes; exceeding it fails with the actual count and limit.
AGENTS.md is not rewritten or shortened by this integration.

The [rubric](agents-review-rubric.md) focuses on material governance defects,
including contradictions, ambiguous precedence, impossible instructions and unsafe
requirements. Optional suggestions do not cause FAIL. Findings must identify
concrete source evidence and consequences; verbosity earns no credit.

The [Review](../.hannes/reviews/agents-quality/rule.toml) runs only at check.
Both AGENTS.md and the rubric are selected as whole-file context, without heading
filters. Hannes supplies their complete current text and original line numbers,
the review instructions and its existing response schema. The orientation diff
is supplementary. Existing context and snapshot limits apply; oversized or missing
context produces ERROR rather than silently reviewing an excerpt.

## Provider and local execution

Install Node.js 20+ and the pinned CLI explicitly:

```sh
npm install --global --ignore-scripts @google/gemini-cli@0.67.0
export GEMINI_API_KEY="your-api-key"
export HANNES_CHANGE_BASE="<baseline-commit>"
hannes mach was --at check
hannes was ist phase --details
```

Use a compatible published governor from the repository's .hannes-version pin.
For a local branch comparison, set HANNES_CHANGE_BASE to
`git merge-base origin/main HEAD`. To review uncommitted document edits, HEAD is
an appropriate baseline. The baseline must resolve to an available commit; no
baseline is guessed. An unchanged document and rubric produce trigger_not_matched,
without checking Gemini credentials or invoking the provider. Git hooks and release
build points never select this Review and need no Gemini installation or key.
The check point still runs its normal deterministic suite and requires its tools.
Agents follow the repository's Git-hook validation policy; these manual commands
are for developers.

The named [gemini-flash provider](../.hannes/providers/gemini-flash.toml) uses the
generic cli transport, gemini-2.5-flash and a 180-second timeout. Runtime requirements
declare python3, gemini and nonempty GEMINI_API_KEY. Credentials are environment
values only and are never stored in provider configuration.

[Gemini's headless reference](https://geminicli.com/docs/cli/headless/) documents
that JSON mode returns an envelope whose response string contains the answer.
The narrow [adapter](../scripts/gemini-review.py) forwards Hannes' stdin prompt to
`gemini --prompt ... --model ... --output-format json --policy ...`, unwraps only
that response string and suppresses raw stdout/stderr diagnostics. It rejects
malformed, duplicate-key, error, empty and oversized envelopes. It never removes
Markdown fences or interprets prose. Hannes independently validates JSON shape,
IDs, verdicts and source evidence under the unchanged response contract.

The adapter uses a temporary working directory and Gemini home, explicit settings
and a deny-all tool policy. Only PATH, the API key and isolated Gemini settings
are passed to the CLI. User/repository memory, extensions, MCP settings and hooks
are not loaded from the repository or normal user configuration. This is not an
OS sandbox. Hannes' existing process deadline and process-tree cleanup cover the
adapter and CLI together. Temporary output is read with the existing 64 KiB limit;
provider usage statistics are not imported.

The pinned package's actual --help output and bundled invocation/policy definitions
were inspected without model calls. Live authentication, model availability and
model adherence to the JSON contract have not been tested. See the official
[CLI reference](https://geminicli.com/docs/cli/cli-reference/) and
[policy reference](https://geminicli.com/docs/reference/policy-engine/).

## CI results and baseline

The existing Quality checks job fetches full history. Its
[baseline helper](../scripts/review-change-base.py) uses the PR head/base merge-base
for pull requests, the event's before commit for pushes, and HEAD for manual release
verification. Missing event commits are fetched explicitly; invalid, zero, unavailable
or unsupported baselines fail visibly. A first push without a previous commit
therefore requires an explicit usable baseline integration rather than HEAD^.
The trigger compares that baseline against the worktree under the existing
HANNES_CHANGE_BASE semantics. Either AGENTS.md or docs/agents-review-rubric.md
changing schedules the Review; all other changes omit it.

CI installs the pinned Gemini CLI before execution and exposes the GEMINI_API_KEY
repository secret only to the Hannes execution step. The reusable release CI call
passes this optional secret explicitly; native builds and Action-consumer jobs
never need it. Fork pull requests normally receive no secret. If their trigger
matches and checks pass, the review reports ERROR; it never fabricates PASS.
Do not supply secrets to untrusted fork code to avoid that error.

All applicable checks run in the same Hannes invocation before any review. A failed
check prevents every LLM review from running. No continue-on-error hides failures.
The always-run [summary helper](../scripts/agents-review-summary.py) reads only
bounded verdict fields from the fresh report, escapes model-authored markup and
publishes evidence where available:

| Summary | Meaning | Job behavior |
| --- | --- | --- |
| PASS | No substantive issues found | Success if all other rules succeeded |
| FAIL | Substantive issues found | Non-blocking because blocking = false |
| ERROR | Review could not complete reliably | Execution fails; investigate |
| SKIP | Trigger did not match, or checks prevented execution | Check failures still fail the job |

Missing credentials, malformed responses, timeouts and provider failures remain
technical ERROR. Missing/startup reports render ERROR, never PASS. The report
artifact retains validated findings and execution metadata; prompts and raw
provider output are not included in the summary. Semantic judgments remain
probabilistic and grounded references do not establish their correctness.

Offline Python tests cover the size boundary, envelope handling, baselines and
summaries. A Rust acceptance fixture uses the real Rule, provider and adapter with
a fake local Gemini executable to verify full context, both triggers, omission,
global gating, credentials, PASS, non-blocking FAIL and malformed output.
