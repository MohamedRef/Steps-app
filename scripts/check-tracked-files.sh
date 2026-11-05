#!/usr/bin/env bash
# Fails if git tracks build output: a target*/ directory, compiled artifacts,
# or any file over 5 MB (the biggest real source file, a map contact sheet,
# is ~3.5 MB), and the textbook PDF or its extracted text (copyright). Build into target/ or a directory outside the checkout.
set -euo pipefail
cd "$(dirname "$0")/.."
LIMIT=$((5 * 1024 * 1024))
bad=0

files="$(git ls-files)"
built="$(grep -E '(^|/)target[^/]*/|\.(rlib|rmeta|o|a|so|dylib|dll|exe|pdb|apk|aab|dex|class|wasm)$' <<<"$files" || true)"
if [ -n "$built" ]; then
  echo "Build output is tracked ($(wc -l <<<"$built") files), e.g.:"
  head -5 <<<"$built"
  bad=1
fi

big="$(git ls-files -s | awk '{print $2}' | git cat-file --batch-check='%(objectsize) %(objectname)' \
  | paste -d' ' - <(git ls-files) | awk -v limit="$LIMIT" '$1 > limit {size = $1; $1 = $2 = ""; sub(/^ +/, ""); print size, $0}')"
if [ -n "$big" ]; then
  echo "Files over 5 MB are tracked:"
  head -10 <<<"$big"
  bad=1
fi

book="$(grep -Ei '\.pdf$|(^|/)book/|(^|/)courses/[^/]+/text/' <<<"$files" || true)"
if [ -n "$book" ]; then
  echo "The textbook or its extracted text is tracked (copyright: keep it in /mnt/project-files/steps/book/):"
  head -5 <<<"$book"
  bad=1
fi

# Mohamed, 2026-11-05: the repo never names the tools it was written with
# (no assistant names, generated-by lines or co-author trailers). The eponym
# "Bernard" (Horner syndrome) is medicine, not a tool. The pattern is split
# so this file doesn't match itself.
tool="cl""aude|anthr""opic"
named="$(git grep -nIiE "$tool" -- . | grep -viE "(${tool%%|*})[ -]bernard" || true)"
if [ -n "$named" ]; then
  echo "Files name an assistant or carry a generated-by line (Mohamed: never on GitHub):"
  head -10 <<<"$named"
  bad=1
fi

[ "$bad" -eq 0 ] || { echo "Fix the files listed above (git rm --cached and .gitignore for tracked output)."; exit 1; }
echo "Tracked files: no build output, no book text, nothing over 5 MB, no assistant names"
