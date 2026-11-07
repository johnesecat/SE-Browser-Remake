#!/usr/bin/env bash
# Run every check the site must pass, failing on the first failure.
#
# GitHub Actions and a developer run the same commands here, so a check that
# only exists in CI cannot drift from the one used locally.
#
# Usage: ci-test.sh [--skip-browser]
#   --skip-browser  skip the Playwright check (needs Chromium and a built dist)
set -euo pipefail

skip_browser=0
case "${1:-}" in
  "") ;;
  --skip-browser) skip_browser=1 ;;
  *) echo "usage: ci-test.sh [--skip-browser]" >&2; exit 2 ;;
esac

cd "$(dirname "$0")/.."

# Chromium needs libraries this sandbox does not ship; unpacked ones live here.
if [ -d .tools/debs/root/usr/lib/x86_64-linux-gnu ]; then
  export LD_LIBRARY_PATH="$PWD/.tools/debs/root/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
python=${PYTHON:-}
if [ -z "$python" ]; then
  if [ -x .tools/venv/bin/python ]; then python=.tools/venv/bin/python; else python=python3; fi
fi

run() {
  echo "== $*"
  "$@"
}

run node tools/test-loader.js
run node tools/test-budget.js
run node --test tools/test-package.js

if [ "$skip_browser" -eq 1 ]; then
  echo "== browser check skipped"
else
  run "$python" tools/test-web.py
fi

echo "All checks passed."
