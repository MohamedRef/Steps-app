#!/usr/bin/env python3
"""Tag an item `high-yield` exactly when First Aid itself emphasises its fact.

Mohamed, 2026-11-14 (issue #667): follow the book. First Aid 2026 has one
typographic emphasis in its body text, the red bold of its mnemonics, plus
its own emphasis words ("most common", "classic", "pathognomonic", ...).
An item is `high-yield` when its `src.quote`, found on its `src.page`:

1. holds a red bold mnemonic, and a word that carries the red bold also
   appears in the item's own text (the item tests the mnemonic's fact), or
2. says one of EMPHASIS, and the item's own text says one too.

Nothing else: not the Section II/III titles or the Rapid Review preface,
which call everything high-yield. Items from another source (`src.book`)
keep their tags. The rule is in docs/CONTENT.md.

The book stays outside git, so this runs where the PDF is (PyMuPDF):

    python3 scripts/high_yield_tags.py --pdf "/mnt/project-files/First Aid ....pdf" [--write] [course ...]

Without --write it prints what would change. It edits only the `tags:`
lines of `content/<course>/LNN.yaml`.
"""

from __future__ import annotations

import argparse
import collections
import glob
import os
import re
import sys
import unicodedata

# Printed page = PDF page - 22 (docs/CONTENT.md).
PAGE_OFFSET = 22
# The red of First Aid's bold mnemonics (ElectraLTStd-Bold).
MNEMONIC_RED = 0xB82837
TAG = "high-yield"

EMPHASIS = re.compile(
    r"(?i)(?<![\w])(most common(ly)?|most frequent(ly)?|#1|number one|classic(al(ly)?)?|"
    r"pathognomonic|hallmark|characteristic(ally)?|most important|leading cause|"
    r"(drug|treatment|agent) of choice|most specific|most sensitive|gold standard)(?![\w])"
)
# Short words a mnemonic can bold that say nothing about the item.
STOP = {
    "not", "are", "out", "the", "and", "with", "for", "you", "can", "all", "has", "its",
    "his", "her", "one", "two", "from", "that", "this", "have", "get", "got", "may", "use",
}
# Item fields that are not the item's own words.
SKIP_KEYS = {"src", "id", "status", "topic", "tags", "kind"}


def nfkc(s: str) -> str:
    return unicodedata.normalize("NFKC", s)


def norm(s: str) -> str:
    return "".join(c.lower() for c in nfkc(s) if c.isalnum())


class Book:
    """Each printed page as lines of (text, is red bold) spans."""

    def __init__(self, pdf: str):
        import pymupdf  # only needed here

        self.doc = pymupdf.open(pdf)
        self.cache: dict[int, list[list[tuple[str, bool]]]] = {}

    def lines(self, page: int) -> list[list[tuple[str, bool]]]:
        if page not in self.cache:
            i = page + PAGE_OFFSET - 1
            out = []
            if 0 <= i < self.doc.page_count:
                for b in self.doc[i].get_text("dict")["blocks"]:
                    for line in b.get("lines", []):
                        out.append([
                            (nfkc(s["text"]), "Bold" in s["font"] and s["color"] == MNEMONIC_RED)
                            for s in line["spans"]
                        ])
            self.cache[page] = out
        return self.cache[page]

    def red_words(self, page: int, quote: str) -> set[str] | None:
        """Words carrying red bold inside the quote, or None if the quote isn't found."""
        lines = self.lines(page)
        text = "".join(c.lower() for line in lines for t, _ in line for c in t if c.isalnum())
        q = norm(quote)
        spans = []
        at = text.find(q) if q else -1
        if at >= 0:
            spans.append((at, at + len(q)))
        else:  # columns read in another order: find the quote's pieces
            for part in re.split(r"[.;:,()\n“”\"]|—|→|–", quote):
                n = norm(part)
                j = text.find(n) if len(n) >= 8 else -1
                if j >= 0:
                    spans.append((j, j + len(n)))
        if not spans:
            return None
        words, k = set(), 0
        for line in lines:
            s = "".join(t for t, _ in line)
            red = [r for t, r in line for _ in t]
            pos = []
            for ch in s:
                pos.append(k if ch.isalnum() else None)
                k += ch.isalnum()
            for m in re.finditer(r"[A-Za-z0-9]+", s):
                start = pos[m.start()]
                if any(red[m.start():m.end()]) and any(a <= start < b for a, b in spans):
                    words.add(m.group(0))
        return words


def own_text(item) -> str:
    out = []

    def walk(v, key=None):
        if key in SKIP_KEYS:
            return
        if isinstance(v, str):
            out.append(v)
        elif isinstance(v, list):
            for x in v:
                walk(x)
        elif isinstance(v, dict):
            for k, x in v.items():
                walk(x, k)

    walk(item)
    return " ".join(out)


def why_high_yield(book: Book, item) -> str | None:
    """The book's emphasis on this item's fact, or None."""
    src = item.get("src") or {}
    quote, page = src.get("quote") or "", src.get("page")
    text = own_text(item)
    reasons = []
    words = book.red_words(page, quote) if isinstance(page, int) else None
    hits = sorted(
        w for w in (words or ())
        if len(w) >= 3 and w.lower() not in STOP and re.search(rf"(?i)\b{re.escape(w)}\b", text)
    )
    if hits:
        reasons.append("mnemonic " + ", ".join(hits))
    q, t = EMPHASIS.search(quote), EMPHASIS.search(text)
    if q and t:
        reasons.append(f"“{q.group(0)}”")
    return "; ".join(reasons) or None


TAGS_LINE = re.compile(r"^    tags: \[(.*)\]\s*$")


def retag(path: str, decide) -> tuple[int, int]:
    """Rewrite the tags lines of one lecture file. Returns (added, removed)."""
    lines = open(path, encoding="utf-8").read().split("\n")
    out, added, removed = [], 0, 0
    i = 0
    # Split into item blocks at "  - id:" lines.
    while i < len(lines):
        m = re.match(r"^  - id: (\S+)\s*$", lines[i])
        if not m:
            out.append(lines[i])
            i += 1
            continue
        j = i + 1
        while j < len(lines) and not re.match(r"^  (- |\S)", lines[j]) and not re.match(r"^\S", lines[j]):
            j += 1
        block = lines[i:j]
        want = decide(m.group(1))
        if want is not None:
            tags_at = next((n for n, l in enumerate(block) if TAGS_LINE.match(l)), None)
            tags = [] if tags_at is None else [
                t.strip() for t in TAGS_LINE.match(block[tags_at]).group(1).split(",") if t.strip()
            ]
            has = TAG in tags
            if want and not has:
                tags = [TAG] + tags
                added += 1
            elif not want and has:
                tags.remove(TAG)
                removed += 1
            if want != has:
                new = f"    tags: [{', '.join(tags)}]" if tags else None
                if tags_at is not None:
                    if new is None:
                        del block[tags_at]
                    else:
                        block[tags_at] = new
                else:
                    status = max(n for n, l in enumerate(block) if l.startswith("    status:"))
                    block.insert(status + 1, new)
        out.extend(block)
        i = j
    open(path, "w", encoding="utf-8").write("\n".join(out))
    return added, removed


def main() -> int:
    import yaml

    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--pdf", required=True)
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--verbose", action="store_true", help="print every item's reason")
    ap.add_argument("courses", nargs="*")
    a = ap.parse_args()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    book = Book(a.pdf)
    files = sorted(glob.glob(os.path.join(root, "content", "*", "L*.yaml")))
    files = [f for f in files if os.path.basename(os.path.dirname(f)) not in ("lessons", "glossary", "qbank")]
    if a.courses:
        files = [f for f in files if os.path.basename(os.path.dirname(f)) in a.courses]
    totals = collections.Counter()
    for f in files:
        course = os.path.basename(os.path.dirname(f))
        doc = yaml.safe_load(open(f, encoding="utf-8"))
        verdict = {}
        for item in doc.get("items", []):
            if (item.get("src") or {}).get("book"):
                continue  # another source: its own emphasis, left as written
            why = why_high_yield(book, item)
            verdict[item["id"]] = why is not None
            if a.verbose and why:
                print(f"{item['id']}: {why}")
        totals[course, "items"] += len(doc.get("items", []))
        totals[course, "tagged"] += sum(verdict.values())
        if a.write:
            add, rem = retag(f, verdict.get)
        else:
            old = {i["id"]: TAG in (i.get("tags") or []) for i in doc.get("items", [])}
            add = sum(1 for k, v in verdict.items() if v and not old[k])
            rem = sum(1 for k, v in verdict.items() if not v and old[k])
        totals[course, "added"] += add
        totals[course, "removed"] += rem
    for course in sorted({c for c, _ in totals}):
        n, t = totals[course, "items"], totals[course, "tagged"]
        print(f"{course}: {t} of {n} items high-yield ({100 * t / max(n, 1):.0f}%), "
              f"+{totals[course, 'added']} -{totals[course, 'removed']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
