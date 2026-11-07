# Perch paper

The paper has two editions built from one source:

- `index.html`, the interactive edition for a general audience. It embeds an
  executable Wasm experiment, its source download, the pinned build manifest
  and proof boundaries, and works offline from disk.
- `latex/perch-paper.pdf`, the academic edition (LNCS). CI builds it with a
  pinned Tectonic and publishes it beside the HTML, which links it; it is not
  committed.

## Sources

`source/outline.json` orders the sections, figures and tables of both editions,
so their numbering agrees. Everything an edition states as a fact comes from
shared data:

| Source | Holds | HTML | LaTeX |
| --- | --- | --- | --- |
| `source/facts/*.json` | numbers, hashes, versions, dates | `{{fact:key}}` | `\fact{key}` |
| `source/tables/<id>.json` | tables | `{{table:id}}` | `\perchtable{id}` |
| `source/figures/<id>.svg` | figures (wide; `.narrow.svg` for phones) | `{{fig:id}}` | `\perchfigure{id}` (TikZ from the same SVG) |
| `source/statements.json` | formal statements | `{{statements}}` | `generated/statements.tex` |
| `source/refs.bib` | references | `{{cite:key}}`, list generated | `\cite{key}` |

Prose is per edition: `source/html/sections/<id>.html` (general audience) and
`latex/sections/<id>.tex` (academic). Shared strings use a small inline markup:
`` `code` ``, `**strong**`, `*em*`, `[text](url)` and the `{{…}}` directives
above (`build/paper.py` documents them).

Interactive figures enhance the static SVG with `source/html/figures.js`
(`data-step`, `data-scenario`, `data-detail`, `data-print="no"`); without
JavaScript, and in print and the PDF, every step shows.

## Building

```sh
python3 docs/perch-paper/build/paper.py          # write index.html and latex/generated/
python3 docs/perch-paper/build/paper.py --check  # CI: fail if either is stale
docs/perch-paper/latex/build.sh                  # PDF; needs Tectonic 0.17.0 on PATH or in $TECTONIC
```

`build.sh` pins the TeX bundle, fails on any undefined or changed reference
or citation, and writes the PDF to `latex/out/`. `build/paper.py` uses only the
Python standard library.

`index.html` keeps four blocks that the embedded source bundle's
`projects/perch-browser/build-page.mjs` refreshes in place (`MEASUREMENTS`,
`MANIFEST_TABLE`, `GOLDEN`, `EMBEDDED_ASSETS`); `paper.py` copies them from the
current file, so the two tools can run in either order. Do not edit any other
part of `index.html` by hand; edit `source/` and rebuild.

Links to sibling pages use relative paths (for example
`../cedar-walkthrough/index.html#perch`) so they work at the site root and in a
nested pull-request preview.
