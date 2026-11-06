#!/usr/bin/env python3
"""Regression tests for documentation failures that must block CI."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("check_docs", Path(__file__).with_name("check-docs.py"))
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class DocumentationTests(unittest.TestCase):
    def test_fenced_headings_do_not_create_anchors_and_duplicate_headings_get_suffixes(self):
        self.assertEqual(checker.anchors("# Setup\n```text\n# Hidden\n```\n# Setup\n"), {"setup", "setup-1"})

    def test_missing_files_and_anchors_fail_instead_of_silently_passing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            readme = root / "README.md"
            for link in ["missing.md", "#missing"]:
                readme.write_text(f"# Setup\n[broken]({link})\n")
                with self.assertRaises(AssertionError):
                    checker.check_links(root, [readme])
            readme.write_text("# Setup\n[valid](#setup)\n")
            checker.check_links(root, [readme])

    def test_deleted_tracked_markdown_is_not_read_but_links_to_it_still_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "README.md").write_text("# Setup\n")
            (root / "SPECIFICATION.md").write_text("")
            with patch.object(checker, "ROOT", root), patch.object(checker.subprocess, "check_output", return_value=b"README.md\0deleted.md\0"):
                checker.main()
                (root / "README.md").write_text("[broken](deleted.md)\n")
                with self.assertRaises(AssertionError):
                    checker.main()

    def test_new_requirements_need_existing_acceptance_references(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "acceptance.rs"
            source.write_text("#[test]\nfn implemented() {}\n")
            contract = root / "SPECIFICATION.md"
            contract.write_text("## CFG-001: Settings\n")
            with self.assertRaises(AssertionError):
                checker.check_requirements(root)
            contract.write_text("## CFG-001: Settings\n| CFG-001 | `acceptance.rs::missing` |\n")
            with self.assertRaises(AssertionError):
                checker.check_requirements(root)
            contract.write_text("## CFG-001: Settings\n| CFG-001 | `acceptance.rs::implemented` |\n")
            checker.check_requirements(root)


if __name__ == "__main__":
    unittest.main()
