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
    def test_envelope_is_unwrapped_without_repairing_model_output(self):
        import json
        for response in ('{"reviews":[]}', '```json\n{}\n```', 'not JSON'):
            self.assertEqual(adapter.unwrap(json.dumps({"response": response, "stats": {}}).encode()), response)
        for raw in (b'{}', b'{"response":null}', b'{"response":""}', b'{"response":"{}","error":{}}',
                    b'{"response":"{}","response":"{}"}', b'x' * 65537, b'private malformed output'):
            with self.assertRaises(ValueError):
                adapter.unwrap(raw)


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
        self.assertIn("@google/gemini-cli@0.67.0", workflow)
        self.assertNotIn("continue-on-error:", workflow)
