"""Offline acceptance for the AGENTS.md review integration."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class WordCountTests(unittest.TestCase):
    def test_word_count_boundary_and_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "AGENTS.md"
            for count, status in ((0, 0), (1199, 0), (1200, 0), (1201, 1)):
                path.write_text("word " * count)
                result = subprocess.run([sys.executable, str(ROOT / "scripts/check-word-count.py"), str(path), "1200"], capture_output=True, text=True)
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertIn(f"{count} words", result.stdout)
                self.assertIn("limit 1200", result.stdout)

    def test_ascii_whitespace_and_markdown_are_literal(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "AGENTS.md"
            path.write_text("# Heading\n`code`\tword\rnext\vlast\fnon\u00a0breaking")
            result = subprocess.run([sys.executable, str(ROOT / "scripts/check-word-count.py"), str(path), "7"], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0)
            self.assertIn("7 words", result.stdout)


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


adapter = module("gemini_review", "gemini-review.py")
summary = module("agents_summary", "agents-review-summary.py")
change_base = module("change_base", "review-change-base.py")


class AdapterTests(unittest.TestCase):
    def response(self, text='{"reviews":[]}'):
        import json
        return json.dumps({"candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": text}]}}]}).encode()

    def execute(self, outcomes):
        from unittest.mock import MagicMock, patch
        import contextlib
        import io
        opener = MagicMock()
        response = MagicMock(status=200)
        response.__enter__.return_value = response
        response.read.return_value = self.response()
        opener.open.side_effect = [response if x is None else x for x in outcomes]
        log = io.StringIO()
        with patch.dict(adapter.os.environ, {"GEMINI_API_KEY": "private-key", "HANNES_MODEL": "configured-model"}), patch.object(adapter, "build_opener", return_value=opener), patch.object(adapter.time, "sleep") as sleep, contextlib.redirect_stderr(log):
            try:
                result = adapter.run(b"complete private prompt")
            except adapter.ProviderError as error:
                result = error
        self.assertNotIn("private", log.getvalue())
        return result, opener.open, sleep, log.getvalue()

    def http_error(self, code, retry_after=None):
        import io
        import json
        from urllib.error import HTTPError
        return HTTPError("https://example.invalid", code, "private message", {"Retry-After": retry_after} if retry_after else {}, io.BytesIO(json.dumps({"error": {"code": code, "message": "private message", "status": "PRIVATE"}}).encode()))

    def test_request_and_success(self):
        import json
        result, call, sleep, log = self.execute([None])
        self.assertEqual(result, '{"reviews":[]}')
        request = call.call_args.args[0]
        self.assertEqual(request.full_url, "https://generativelanguage.googleapis.com/v1beta/models/configured-model:generateContent")
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.get_header("X-goog-api-key"), "private-key")
        self.assertEqual(request.get_header("Content-type"), "application/json")
        self.assertEqual(json.loads(request.data), {"contents": [{"role": "user", "parts": [{"text": "complete private prompt"}]}], "generationConfig": {"responseMimeType": "application/json", "maxOutputTokens": 8192}})
        self.assertEqual(call.call_args.kwargs, {"timeout": 600})
        sleep.assert_not_called()
        self.assertEqual(log, "")

    def test_retryable_statuses_and_exhaustion(self):
        for code in adapter.RETRYABLE:
            result, call, sleep, log = self.execute([self.http_error(code), None])
            self.assertIsInstance(result, str)
            self.assertEqual(call.call_count, 2)
            sleep.assert_called_once_with(30)
            self.assertIn(f"HTTP {code}; retrying (attempt 2/5) in 30 seconds.", log)
        result, call, sleep, log = self.execute([self.http_error(503) for _ in range(5)])
        self.assertIn("exhausted 5 attempts", str(result))
        self.assertEqual(call.call_count, 5)
        self.assertEqual([x.args[0] for x in sleep.call_args_list], [30, 60, 120, 240])
        self.assertEqual(len(log.splitlines()), 4)

    def test_retry_after_is_bounded(self):
        from unittest.mock import patch
        from email.utils import formatdate
        self.assertEqual(adapter.retry_delay({"Retry-After": "90"}, 30), 90)
        self.assertEqual(adapter.retry_delay({"Retry-After": "999999"}, 30), 240)
        self.assertEqual(adapter.retry_delay({"Retry-After": "bad"}, 60), 60)
        with patch.object(adapter.time, "time", return_value=1000):
            self.assertEqual(adapter.retry_delay({"Retry-After": formatdate(1090, usegmt=True)}, 30), 90)
        _, _, sleep, _ = self.execute([self.http_error(429, "90"), None])
        sleep.assert_called_once_with(90)

    def test_connection_timeout_and_nonretryable_errors(self):
        from urllib.error import URLError
        for error in [TimeoutError(), URLError(TimeoutError()), self.http_error(400), self.http_error(401), self.http_error(403), URLError("private failure")]:
            result, call, sleep, _ = self.execute([error])
            self.assertIsInstance(result, adapter.ProviderError)
            self.assertEqual(call.call_count, 1)
            sleep.assert_not_called()
        result, call, sleep, _ = self.execute([URLError(ConnectionResetError()), None])
        self.assertIsInstance(result, str)
        self.assertEqual(call.call_count, 2)
        sleep.assert_called_once_with(30)

    def test_envelope_does_not_repair_model_output(self):
        import json
        for text in ('{"reviews":[]}', '```json\n{}\n```', 'not JSON'):
            self.assertEqual(adapter.unwrap(self.response(text)), text)
        envelope = json.loads(self.response())
        envelope["candidates"][0]["content"]["parts"] = [{"text": "first"}, {"text": "second"}]
        self.assertEqual(adapter.unwrap(json.dumps(envelope).encode()), "firstsecond")
        for raw in (b'{}', b'{"candidates":[]}', b'{"candidates":null}', b'{"error":{}}', b'{"candidates":[],"candidates":[]}', b'x' * 65537, b'private malformed output'):
            with self.assertRaises(adapter.ProviderError):
                adapter.unwrap(raw)
        for finish in ("MAX_TOKENS", "SAFETY", "RECITATION", None):
            envelope["candidates"][0]["finishReason"] = finish
            with self.assertRaises(adapter.ProviderError):
                adapter.unwrap(json.dumps(envelope).encode())
        for parts in ([], [{"text": ""}], [{"text": 1}], [{"thought": True, "text": "private"}], [{"functionCall": {}}]):
            envelope["candidates"][0].update(finishReason="STOP", content={"parts": parts})
            with self.assertRaises(adapter.ProviderError):
                adapter.unwrap(json.dumps(envelope).encode())

    def test_invalid_response_is_not_retried(self):
        from unittest.mock import MagicMock, patch
        response = MagicMock(status=200)
        response.__enter__.return_value = response
        response.read.return_value = b"malformed private response"
        with patch.dict(adapter.os.environ, {"GEMINI_API_KEY": "key", "HANNES_MODEL": "model"}), patch.object(adapter, "build_opener") as build, patch.object(adapter.time, "sleep") as sleep:
            build.return_value.open.return_value = response
            with self.assertRaises(adapter.ProviderError):
                adapter.run("prompt")
            build.return_value.open.assert_called_once()
            sleep.assert_not_called()


class SummaryTests(unittest.TestCase):
    def report(self, status):
        return {"schema_version": 4, "overall": "FAIL" if status == "NOT_RUN" else "PASS", "checks_only": False, "plan": {"at": "check", "status": "planned", "omitted": []},
                "results": [{"id": "agents-quality", "kind": "review", "status": status,
                             "reason": "Evidence inspected", "findings": [{"message": "Concrete issue", "path": "AGENTS.md", "line": 1}]}]}

    def test_all_outcomes_and_evidence(self):
        for status in ("PASS", "FAIL", "ERROR", "NOT_RUN"):
            text = summary.render(self.report(status))
            self.assertIn("SKIP" if status == "NOT_RUN" else status, text)
            if status != "NOT_RUN":
                self.assertIn("AGENTS.md:1: Concrete issue", text)
            if status == "FAIL":
                self.assertIn("non-blocking", text)
        report = self.report("PASS")
        report["results"] = []
        report["plan"]["omitted"] = [{"rule": {"id": "agents-quality"}, "reason": "trigger_not_matched"}]
        self.assertIn("SKIP", summary.render(report))
        report["plan"]["omitted"] = []
        self.assertIn("ERROR", summary.render(report))
        self.assertIn("ERROR", summary.render({}))
        blocked = self.report("NOT_RUN")
        blocked["overall"] = "ERROR"
        self.assertIn("ERROR", summary.render(blocked))
        report = self.report("PASS")
        report["checks_only"] = True
        self.assertIn("ERROR", summary.render(report))

    def test_summary_escapes_markup_and_omits_raw_output(self):
        report = self.report("FAIL")
        report["results"][0].update(reason="<img src=x>", stdout="private raw output", stderr="private error")
        text = summary.render(report)
        self.assertIn("&lt;img src=x&gt;", text)
        self.assertNotIn("private", text)


class BaselineTests(unittest.TestCase):
    def test_pr_merge_base_push_and_invalid_baselines(self):
        from unittest.mock import patch
        base, head = "a" * 40, "b" * 40
        with patch.object(change_base, "commit", side_effect=lambda value: value), patch.object(change_base, "git", return_value="c" * 40) as git:
            self.assertEqual(change_base.baseline("pull_request", {"pull_request": {"base": {"sha": base}, "head": {"sha": head}}}), "c" * 40)
            git.assert_called_once_with("merge-base", base, head)
            self.assertEqual(change_base.baseline("push", {"before": base}), base)
        for value in ("", "HEAD^", "0" * 40, "--all"):
            with self.assertRaises(ValueError):
                change_base.commit(value)
        with patch.object(change_base, "git", side_effect=[subprocess.CalledProcessError(1, "git"), "", base]) as git:
            self.assertEqual(change_base.commit(base), base)
            self.assertIn(unittest.mock.call("fetch", "--no-tags", "origin", base), git.call_args_list)
        with patch.object(change_base, "git", side_effect=subprocess.CalledProcessError(1, "git")):
            with self.assertRaises(subprocess.CalledProcessError):
                change_base.commit(base)

    def test_ci_secret_scope_and_review_execution(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text().split("  action-consumer:")[0]
        self.assertEqual(workflow.count("GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}"), 1)
        self.assertIn("fetch-depth: 0", workflow)
        self.assertIn("scripts/review-change-base.py", workflow)
        self.assertIn("check --reviews", workflow)
        self.assertNotIn("gemini-cli", workflow)
        self.assertIn("timeout-minutes: 90", workflow)
        self.assertNotIn("continue-on-error:", workflow)
