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
split words. The 1200-word limit applies to the complete current document.
Exactly 1200 passes; exceeding it fails with the actual count and limit.
AGENTS.md is not rewritten or shortened by this integration.

The [rubric](agents-review-rubric.md) focuses on material governance defects,
including contradictions, ambiguous precedence, impossible instructions and unsafe
requirements. Optional suggestions do not cause FAIL. Findings must identify
concrete source evidence and consequences; verbosity earns no credit.

The [Review](../.hannes/reviews/agents-quality/rule.toml) runs only at check.
Both AGENTS.md and the rubric are selected as whole-file context, without heading
filters. Hannes supplies their complete current text and original line numbers,
the review instructions and its existing response schema. The active configuration
keeps the orientation diff until the new governor is released and adopted.
The implemented `include_diff = false` setting will exclude it after that separate
configuration migration. Existing context and snapshot limits apply; oversized or missing
context produces ERROR rather than silently reviewing an excerpt.

## Provider and local execution

Install Python 3 and supply the API key explicitly:

```sh
export GEMINI_API_KEY="your-api-key"
export HANNES_CHANGE_BASE="<baseline-commit>"
hannes mach was --at check
hannes was ist phase --details
```

The active configuration remains compatible with published Hannes v0.11.0.
It omits `review.include_diff` and retains its 180-second CLI deadline. Adopting a
release containing these changes and enabling `include_diff = false` and
`timeout_seconds = 0` are separate future operations.
For a local branch comparison, use the merge-base with the target branch as
HANNES_CHANGE_BASE; HEAD is suitable for uncommitted edits. Only AGENTS.md changes
trigger the Review. Rubric changes alone omit it, without checking credentials.
Git hooks and release build points never select this Review. The check point
still requires its deterministic tools. Agents use Git-hook validation; the
commands above are for developers.

The named [gemini-flash provider](../.hannes/providers/gemini-flash.toml) invokes
[scripts/gemini-review.py](../scripts/gemini-review.py) through the generic CLI
transport, using Python's standard library. The configured model is
gemini-3.8-flash, passed as HANNES_MODEL. No Gemini CLI or HTTP dependency is needed.
Authentication uses the GEMINI_API_KEY environment variable in x-goog-api-key.

One POST to `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
contains the complete assembled prompt as one user text part. generationConfig
sets responseMimeType to application/json and maxOutputTokens to 8192. The adapter
selects the first candidate, requires finishReason STOP and concatenates its text
parts. It rejects missing, blocked, malformed, duplicate-key, empty, oversized or
incomplete envelopes. HTTP 200 alone proves no review outcome. Extracted text is
returned unchanged; Hannes validates IDs, verdicts, JSON shape and source evidence.
It never removes fences, repairs JSON or fabricates PASS.

There are at most five attempts. HTTP 429, 500, 502, 503 and 504 and transient
connection failures use delays of 30, 60, 120 and 240 seconds. Retry-After seconds
or HTTP dates may increase each delay, capped at 240 seconds. Authentication,
invalid requests and malformed model responses are not retried. Each request uses
a 600-second HTTP timeout; a timeout stops immediately without retry. The adapter
has no overall execution deadline. The active provider retains its existing
180-second enclosing CLI deadline, which can interrupt a request or its retry
budget. After release adoption, timeout_seconds = 0 can disable that enclosing
deadline. Other providers retain their configured deadlines.
No fallback model or provider is selected.

Normal success is quiet. Each scheduled retry prints one safe diagnostic to
stderr. The new Hannes implementation forwards only fixed Gemini diagnostic formats and bounded numeric
fields; arbitrary provider stderr stays private. Final provider errors remain in
reports and CLI output. API error envelopes are parsed when possible, but HTTP
status controls retries and API message content is never logged. Redirects are
rejected to prevent forwarding the API key. Responses use the existing 64 KiB
capture bound; usage statistics are not imported.

## CI results and baseline

The existing Quality checks job fetches full history. Its
[baseline helper](../scripts/review-change-base.py) uses the PR head/base merge-base
for pull requests, the event's before commit for pushes, and HEAD for manual release
verification. Missing event commits are fetched explicitly; invalid, zero, unavailable
or unsupported baselines fail visibly. A first push without a previous commit
therefore requires an explicit usable baseline integration rather than HEAD^.
The trigger compares that baseline against the worktree under the existing
HANNES_CHANGE_BASE semantics. AGENTS.md
changing schedules the Review; all other changes omit it.

CI allows 90 minutes for checks and the HTTP retry budget, and exposes the GEMINI_API_KEY
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
| ERROR | Review could not complete reliably | Fatal with v0.11.0; non-blocking with the new implementation |
| SKIP | Trigger did not match, or checks prevented execution | Check failures still fail the job |

Missing credentials, malformed responses, timeouts and provider failures remain
technical ERROR. Missing/startup reports render ERROR, never PASS. The report
artifact retains validated findings and execution metadata; prompts and raw
provider output are not included in the summary. Semantic judgments remain
probabilistic and grounded references do not establish their correctness.

Offline Python tests cover the size boundary, envelope handling, baselines and
summaries. A Rust acceptance fixture uses the real Rule, provider and adapter with
fixture-only new settings and mocked HTTP responses to verify full context, AGENTS.md-only triggering, omission,
global gating, credentials, PASS, non-blocking FAIL and malformed output.
