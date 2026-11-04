#!/usr/bin/env bash
# The build number every Steps build carries (STEPS_BUILD, update.json's
# `build`): the number of commits on HEAD. Main only grows, so the preview,
# the releases and a build from the Mac all count on one scale and
# `(version, build)` orders them (#110). Each workflow's own run counter
# used to be used instead, so a preview always looked newer than a release.
# Needs the full history (actions/checkout with fetch-depth: 0).
set -euo pipefail
if [ "$(git rev-parse --is-shallow-repository)" = true ]; then
  echo "build-number.sh needs the full history (fetch-depth: 0)" >&2
  exit 1
fi
git rev-list --count HEAD
