#!/usr/bin/env python3
"""Explain whether a Git change can ship as a Steps content-only preview.

The same conservative classifier is used by the preview publisher and CI's
human-readable report. Unknown paths always require a full application build.
"""

from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
import re
import subprocess
import sys

# Only changes represented by the current content pack may skip an APK.
LECTURE = re.compile(r"^content/[a-z]+/L[0-9]+\.yaml$")
LESSON = re.compile(r"^content/lessons/[a-z]+/L[0-9]+\.yaml$")
LESSON_IMAGE = re.compile(r"^content/lessons/img/[a-z]+/[^/]+\.(?:webp|png|jpg|jpeg)$")
STYLESHEET = re.compile(r"^crates/app/.+\.css$")
BOOK_REF = re.compile(r"^content/book-refs/(?:[a-z]+/)?L[0-9]+\.tsv$")

LABELS = {
    "content": "Packed lessons / lectures / images",
    "style": "Packed stylesheets",
    "metadata": "Notes / source checks (no app code)",
    "app": "App build required",
}


@dataclass(frozen=True)
class Change:
    status: str
    path: str
    category: str


def classify(path: str) -> str:
    """Be conservative: do not ship an unknown dependency as content-only."""
    if path in {"content/index.yaml", "content/program.yaml"}:
        return "content"
    if LECTURE.fullmatch(path) or LESSON.fullmatch(path) or LESSON_IMAGE.fullmatch(path):
        return "content"
    if STYLESHEET.fullmatch(path):
        return "style"
    if (
        path in {"content/approved.txt", "content/hb/.gitkeep"}
        or BOOK_REF.fullmatch(path)
        or path.startswith(("changes/", "docs/"))
        or ("/" not in path and path.endswith(".md"))
    ):
        return "metadata"
    # content/glossary.yaml is embedded separately in the app and cannot be
    # published as a downloadable content-only update.
    return "app"


def parse_name_status(data: bytes) -> list[Change]:
    """Read NUL-delimited git diff --name-status -z --no-renames."""
    if not data:
        return []
    fields = data.split(b"\0")
    if fields[-1] != b"" or len(fields) % 2 != 1:
        raise ValueError("unexpected git diff --name-status -z output")
    changes: list[Change] = []
    for i in range(0, len(fields) - 1, 2):
        status = fields[i].decode("ascii")
        path = fields[i + 1].decode("utf-8", errors="surrogateescape")
        if not status or not path or len(status) != 1:
            raise ValueError("invalid status or path in git diff")
        changes.append(Change(status, path, classify(path)))
    return changes


def git_changes(base: str, head: str) -> list[Change]:
    # --no-renames shows both the old and new paths. A move out of an
    # allowed directory can never accidentally qualify as content-only.
    output = subprocess.run(
        ["git", "diff", "--name-status", "-z", "--no-renames", base, head],
        check=True,
        capture_output=True,
    )
    return parse_name_status(output.stdout)


def content_only(changes: list[Change]) -> bool:
    return bool(changes) and all(c.category != "app" for c in changes)


def markdown(changes: list[Change], base: str, head: str) -> str:
    permitted = content_only(changes)
    if not changes:
        outcome = "No file changes; keep the normal build path."
    elif permitted:
        outcome = "Content-only preview is eligible (no new APK needed)."
    else:
        outcome = "Full app build required (the existing APK cannot be carried)."
    counts = Counter(c.category for c in changes)
    lines = [
        "### Steps update impact",
        "",
        f"**Result:** {outcome}",
        "",
        f"Compared `{base}` to `{head}`. {len(changes)} changed file(s).",
        "",
        "| Change type | Files |",
        "| --- | ---: |",
    ]
    for category in LABELS:
        lines.append(f"| {LABELS[category]} | {counts[category]} |")
    lines.append("")
    for category, label in LABELS.items():
        matches = [c for c in changes if c.category == category]
        if not matches:
            continue
        lines.append(f"**{label}**")
        lines.append("")
        for c in matches[:20]:
            safe = c.path.replace("`", "\\`").replace("|", "\\|")
            lines.append(f"- `{safe}` ({c.status})")
        if len(matches) > 20:
            lines.append(f"- ...and {len(matches) - 20} more file(s)")
        lines.append("")
    lines.append(
        "Classification is conservative. Publication still requires valid content, "
        "the existing signing keys, compatible version and verified update feed."
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, help="Existing build or PR base commit")
    parser.add_argument("--head", default="HEAD", help="Target commit (default: HEAD)")
    parser.add_argument(
        "--check-content-only", action="store_true",
        help="Exit 0 only when every changed path may skip a new APK; otherwise exit 1",
    )
    args = parser.parse_args(argv)
    try:
        changes = git_changes(args.base, args.head)
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"Unable to classify update: {error}", file=sys.stderr)
        return 2
    if args.check_content_only:
        print("Content-only preview eligible" if content_only(changes) else "Full app build required")
        return 0 if content_only(changes) else 1
    print(markdown(changes, args.base, args.head))
    return 0


if __name__ == "__main__":
    sys.exit(main())
