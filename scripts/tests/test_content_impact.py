"""Regression tests for the single-source content-only publication gate."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from content_impact import Change, classify, content_only, git_changes, markdown, parse_name_status


class ClassifierTests(unittest.TestCase):
    def test_all_embedded_content_types_are_supported(self):
        for path in (
            "content/index.yaml",
            "content/program.yaml",
            "content/hb/L01.yaml",
            "content/neph/L14.yaml",
            "content/lessons/hb/L01.yaml",
            "content/lessons/img/hb/L01-portal-anatomy.webp",
        ):
            with self.subTest(path=path):
                self.assertEqual(classify(path), "content")

    def test_styles_and_metadata_can_skip_app_build(self):
        self.assertEqual(classify("crates/app/src/lecture/lecture.css"), "style")
        for path in (
            "changes/content-report.yaml",
            "docs/RELEASE.md",
            "README.md",
            "content/book-refs/hb/L01.tsv",
            "content/book-refs/L01.tsv",
            "content/approved.txt",
        ):
            with self.subTest(path=path):
                self.assertEqual(classify(path), "metadata")

    def test_unknown_and_code_are_never_content_only(self):
        for path in (
            "Cargo.toml",
            "crates/app/src/content.rs",
            "crates/core/src/pack.rs",
            "content/glossary.yaml",
            "content/lessons/img/hb/L01-image.exe",
            "scripts/content_impact.py",
            ".github/workflows/ci.yml",
            "content/hb/L01.json",
        ):
            with self.subTest(path=path):
                self.assertEqual(classify(path), "app")

    def test_mixed_or_empty_delta_never_skips_an_apk(self):
        self.assertFalse(content_only([]))
        self.assertTrue(content_only([Change("M", "content/index.yaml", "content")]))
        self.assertFalse(content_only([
            Change("M", "content/index.yaml", "content"),
            Change("M", "crates/app/src/main.rs", "app"),
        ]))

    def test_null_delimited_changes_with_whitespace(self):
        data = b"M\0content/lessons/hb/L01.yaml\0A\0docs/a file.md\0"
        changes = parse_name_status(data)
        self.assertEqual(len(changes), 2)
        self.assertEqual(changes[1].path, "docs/a file.md")
        self.assertTrue(content_only(changes))
        with self.assertRaises(ValueError):
            parse_name_status(b"M\0truncated")

    def test_readable_report_identifies_files_requiring_rebuild(self):
        report = markdown(
            [Change("M", "content/index.yaml", "content"), Change("M", "Cargo.toml", "app")],
            "HEAD^1", "HEAD",
        )
        self.assertIn("Full app build required", report)
        self.assertIn("`Cargo.toml`", report)

    def test_diff_detects_rename_crossing_safe_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(["git", "-C", directory, *args], check=True,
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            git("init", "-q")
            git("config", "user.name", "Test")
            git("config", "user.email", "test@example.com")
            (root / "content" / "hb").mkdir(parents=True)
            (root / "content" / "hb" / "L01.yaml").write_text("content")
            git("add", ".")
            git("commit", "-qm", "Add lecture")
            base = git("rev-parse", "HEAD").stdout.decode().strip()
            (root / "scripts").mkdir()
            (root / "content" / "hb" / "L01.yaml").rename(root / "scripts" / "L01.yaml")
            git("add", "-A")
            git("commit", "-qm", "Move lecture to scripts")
            import os
            current = Path.cwd()
            try:
                os.chdir(root)
                changes = git_changes(base, "HEAD")
            finally:
                os.chdir(current)
            self.assertEqual({c.status for c in changes}, {"A", "D"})
            self.assertFalse(content_only(changes))


if __name__ == "__main__":
    unittest.main()
