#!/usr/bin/env bash
# Build out/perch-paper.pdf with a pinned Tectonic and TeX bundle, and fail on
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
# SKIP_CHECK=1 skips the source/generated consistency check for local drafting; CI never sets it.
if [ "${SKIP_CHECK:-0}" != 1 ]; then python3 ../build/paper.py --check; fi
# ANONYMOUS=1 builds the double-blind edition (author names, affiliation and
# the authors' repository links withheld) as out/perch-paper-anonymous.pdf.
main=perch-paper
[ "${ANONYMOUS:-0}" = 1 ] && main=perch-paper-anonymous
mkdir -p out
rm -f out/$main.*
if [ "$main" = perch-paper-anonymous ]; then
  rm -rf out/perch-paper.tex out/sections out/generated
  printf '\\def\\anonymous{}\\input{perch-paper}\n' > out/$main.tex
  cp -R perch-paper.tex sections generated out/
fi
src=perch-paper.tex
[ "$main" = perch-paper ] || src=out/$main.tex
"$TECTONIC" -X compile --bundle "$BUNDLE" --keep-logs --keep-intermediates --outdir out "$src"
status=0
if grep -nE "LaTeX Warning: (Reference|Citation) .* undefined|There were undefined references|Label\(s\) may have changed|multiply defined|Package perch Error" out/$main.log; then
  status=1
fi
# A glyph the fonts lack is dropped silently from the PDF; refuse that too.
if grep -n "Missing character: There is no" out/$main.log; then
  status=1
fi
if [ -f out/$main.blg ] && grep -nE "^Warning--|error message" out/$main.blg; then
  status=1
fi
if [ "$status" -ne 0 ]; then
  echo "build.sh: unresolved references or citations, or missing glyphs (see above)" >&2
  exit 1
fi
echo "build.sh: wrote $(pwd)/out/$main.pdf"
