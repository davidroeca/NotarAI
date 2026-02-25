#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# 1. Run Astro build (outputs to dist/ — preserved for astro preview)
npx astro build

# 2. Create GitHub Pages output directory (clean any leftover from a previous partial run)
rm -rf gh-pages
mkdir -p gh-pages/docs

# 3. Copy Astro output into docs/ subdirectory (copy, not move — keeps dist/ intact)
cp -r dist/. gh-pages/docs/

# 4. Copy non-docs root assets (redirect page, schema)
cp -rL public-root/. gh-pages/
