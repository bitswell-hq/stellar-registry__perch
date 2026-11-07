#!/usr/bin/env bash
# Build perch-paper.pdf with a pinned Tectonic and TeX bundle, and fail on
# undefined or changed references, undefined citations and BibTeX warnings.
# CI installs the binary it names (TECTONIC_VERSION); locally, put that
# version on PATH or set TECTONIC to its path.
set -euo pipefail
cd "$(dirname "$0")"
TECTONIC_VERSION=0.17.0
BUNDLE=https://relay.fullyjustified.net/default_bundle_v33.tar
TECTONIC=${TECTONIC:-tectonic}
version=$("$TECTONIC" --version)
if [ "$version" != "Tectonic $TECTONIC_VERSION" ]; then
  echo "build.sh: need Tectonic $TECTONIC_VERSION, found: $version" >&2
  exit 1
fi
python3 ../build/paper.py --check
rm -rf out && mkdir out
"$TECTONIC" -X compile --bundle "$BUNDLE" --keep-logs --keep-intermediates --outdir out perch-paper.tex
status=0
if grep -nE "LaTeX Warning: (Reference|Citation) .* undefined|There were undefined references|Label\(s\) may have changed|multiply defined|Package perch Error" out/perch-paper.log; then
  status=1
fi
if [ -f out/perch-paper.blg ] && grep -nE "^Warning--|error message" out/perch-paper.blg; then
  status=1
fi
if [ "$status" -ne 0 ]; then
  echo "build.sh: unresolved references or citations (see above)" >&2
  exit 1
fi
cp out/perch-paper.pdf perch-paper.pdf
echo "build.sh: wrote $(pwd)/perch-paper.pdf"
