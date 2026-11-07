#!/usr/bin/env python3
"""Build both editions of the Perch paper from docs/perch-paper/source/.

    python3 docs/perch-paper/build/paper.py          # write index.html and latex/generated/
    python3 docs/perch-paper/build/paper.py --check  # fail if either is stale or inconsistent

One outline (source/outline.json) orders the sections, figures and tables of
both editions. Facts, tables, formal statements, references and figure SVG are
shared data, rendered into the HTML edition and into LaTeX includes. Prose is
per edition: source/html/sections/<id>.html and latex/sections/<id>.tex.

index.html keeps four blocks that projects/perch-browser/build-page.mjs (in the
embedded source bundle) refreshes in place: MEASUREMENTS, MANIFEST_TABLE, GOLDEN
and EMBEDDED_ASSETS. This build copies them from the current index.html, so the
two tools can run in either order. Standard library only.
"""
import html
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import svg2tikz  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / 'source'
LATEX = ROOT / 'latex'
GEN = LATEX / 'generated'
INDEX = ROOT / 'index.html'
WALKTHROUGH = ROOT.parent / 'oz-walkthrough' / 'index.html'
CARRIED = ('MEASUREMENTS', 'MANIFEST_TABLE', 'GOLDEN', 'EMBEDDED_ASSETS')
ERRORS = []


def fail(msg):
    ERRORS.append(msg)


def read(p):
    return Path(p).read_text(encoding='utf-8')


# ---------------------------------------------------------------- shared data

class Paper:
    def __init__(self):
        self.outline = json.loads(read(SRC / 'outline.json'))
        self.facts = {}
        for fp in sorted((SRC / 'facts').glob('*.json')):
            for k, v in json.loads(read(fp)).items():
                if k in self.facts:
                    fail(f'fact {k!r} defined twice (second in facts/{fp.name})')
                self.facts[k] = v
        self.tables = {p.stem: json.loads(read(p)) for p in sorted((SRC / 'tables').glob('*.json'))}
        self.statements = json.loads(read(SRC / 'statements.json'))
        self.bib = parse_bib(read(SRC / 'refs.bib'))
        self.sections = self.outline['sections']
        self.figures = self.outline['figures']
        self.sec_ids = [s['id'] for s in self.sections]
        for f in self.figures:
            if f not in [x for s in self.sections for x in s.get('figures', [])]:
                fail(f'figure {f} is in no section')
        for s in self.sections:
            if s.get('editions') and set(s['editions']) != {'html', 'latex'} and not (s.get('unnumbered') or s.get('appendix')):
                fail(f'section {s["id"]} is in one edition only, so it must be unnumbered or an appendix')
        self.numbers = {ed: self._number(ed) for ed in ('html', 'latex')}

    def in_edition(self, item, ed):
        return ed in item.get('editions', ['html', 'latex'])

    def placed(self, ed, kind):
        # Exhibit ids per section for one edition, after latex_section overrides.
        items = self.figures if kind == 'figures' else self.tables
        out = {s['id']: [] for s in self.sections}
        for s in self.sections:
            for x in s.get(kind, []):
                target = items[x].get('latex_section', s['id']) if ed == 'latex' else s['id']
                out[target].append(x)
        return out

    def _number(self, ed):
        nums, fig, tab, main, app = {}, 0, 0, 0, 0
        figs, tabs = self.placed(ed, 'figures'), self.placed(ed, 'tables')
        for s in self.sections:
            if not self.in_edition(s, ed):
                continue
            if s.get('unnumbered'):
                pass
            elif s.get('appendix'):
                nums['sec:' + s['id']] = 'ABCDEFGH'[app]
                app += 1
            else:
                main += 1
                nums['sec:' + s['id']] = f'{main:02d}' if ed == 'html' else str(main)
            for f in figs[s['id']]:
                if self.in_edition(self.figures[f], ed):
                    fig += 1
                    nums['fig:' + f] = str(fig)
            for t in tabs[s['id']]:
                if self.in_edition(self.tables[t], ed):
                    tab += 1
                    nums['tab:' + t] = str(tab)
        return nums


def parse_bib(text):
    """Parse the restricted BibTeX this repo writes: @type{key, field = {value}, ...}."""
    entries = {}
    text = re.sub(r'^%.*$', '', text, flags=re.M)
    for m in re.finditer(r'@(\w+)\s*\{\s*([^,\s]+)\s*,(.*?)\n\}', text, re.S):
        fields = {}
        for fm in re.finditer(r'(\w+)\s*=\s*\{((?:[^{}]|\{[^{}]*\})*)\}', m.group(3), re.S):
            fields[fm.group(1).lower()] = ' '.join(fm.group(2).split())
        fields['_type'] = m.group(1).lower()
        entries[m.group(2)] = fields
    return entries


# ---------------------------------------------------------------- inline markup
# Shared strings (captions, cells, statements, facts) use a small markup:
# `code`, **strong**, *em*, [text](url), and {{fact:k}} {{ref:fig:id}}
# {{ref:tab:id}} {{ref:sec:id}} {{cite:key}} {{badge:kind:LABEL}}.

TOKEN = re.compile(r'(`[^`]+`|\*\*.+?\*\*|\*[^*\s][^*]*?\*|\[[^\]]+\]\([^)\s]+\)|\{\{[^}]+\}\})')


def _directive(p, body, ed, ctx):
    kind, _, arg = body.partition(':')
    if kind == 'fact':
        if arg not in p.facts:
            fail(f'unknown fact {arg!r} in {ctx}')
            return '??'
        return r'\fact{' + arg + '}' if ed == 'latex' else fact_value(p, arg, ed)
    if kind == 'ref':
        if arg not in p.numbers[ed] and not (arg.startswith('sec:') and arg[4:] in p.sec_ids):
            fail(f'unknown or excluded reference {arg!r} in {ctx} ({ed})')
            return '??'
        appendix = arg.startswith('sec:') and any(x['id'] == arg[4:] and x.get('appendix') for x in p.sections)
        if ed == 'latex':
            if arg.startswith('sec:') and arg not in p.numbers['latex']:
                fail(f'reference {arg!r} in {ctx} names a section that has no number in the LaTeX edition')
            pre = 'Appendix~' if appendix else {'sec': r'\S', 'fig': r'Figure~', 'tab': r'Table~'}[arg[:3]]
            return pre + r'\ref{' + arg + '}'
        n = p.numbers['html'].get(arg)
        if n is None and arg.startswith('sec:'):
            sec = next(x for x in p.sections if x['id'] == arg[4:])
            return f'<a href="#{arg[4:]}">{html.escape(sec["title_html"])}</a>'
        label = ('Appendix ' if appendix else {'sec': '§', 'fig': 'Figure ', 'tab': 'Table '}[arg[:3]]) + (n or '??')
        target = arg[4:]
        return f'<a href="#{target}">{label}</a>'
    if kind == 'cite':
        keys = arg.split(',')
        for k in keys:
            if k not in p.bib:
                fail(f'unknown citation {k!r} in {ctx}')
        if ed == 'latex':
            return r'\cite{' + ','.join(keys) + '}'
        return '[' + ', '.join(f'<a class="cite" href="#ref-{k}">{p.cite_number(k)}</a>' for k in keys) + ']'
    if kind == 'obj':
        if ed == 'latex':
            return r'\statusobj{' + svg2tikz.tex_escape(arg) + '}'
        return f'<span class="obj">{html.escape(arg, quote=False)}</span>'
    if kind == 'badge':
        b, _, label = arg.partition(':')
        if ed == 'latex':
            return r'\badge{' + svg2tikz.tex_escape(label) + '}'
        return f'<span class="badge {html.escape(b)}">{html.escape(label)}</span>'
    fail(f'unknown directive {{{{{body}}}}} in {ctx}')
    return '??'


def render(p, s, ed, ctx=''):
    out = []
    for part in TOKEN.split(s):
        if not part:
            continue
        if part.startswith('`') and part.endswith('`') and len(part) > 1:
            inner = part[1:-1]
            if ed == 'latex':
                bits = re.split(r'(\{\{fact:[^}]+\}\})', inner)
                body = ''.join(r'\fact{' + b[7:-2] + '}' if b.startswith('{{fact:') and b[7:-2] in p.facts
                               else tex_code(b) for b in bits)
                out.append(r'\code{' + body + '}')
            else:
                inner = re.sub(r'\{\{fact:([^}]+)\}\}', lambda m: plain_fact(p, m.group(1), ctx), inner)
                out.append('<code>' + html.escape(inner, quote=False) + '</code>')
        elif part.startswith('**') and part.endswith('**'):
            inner = render(p, part[2:-2], ed, ctx)
            out.append(r'\textbf{' + inner + '}' if ed == 'latex' else f'<strong>{inner}</strong>')
        elif part.startswith('*') and part.endswith('*') and len(part) > 2:
            inner = render(p, part[1:-1], ed, ctx)
            out.append(r'\emph{' + inner + '}' if ed == 'latex' else f'<em>{inner}</em>')
        elif part.startswith('[') and part.endswith(')') and '](' in part:
            text, url = part[1:-1].split('](', 1)
            inner = render(p, text, ed, ctx)
            if ed == 'latex':
                cmd = r'\repolink' if SELF_REPOS.match(url) else r'\href'
                out.append(cmd + '{' + url.replace('%', r'\%').replace('#', r'\#') + '}{' + inner + '}')
            else:
                out.append(f'<a href="{html.escape(url)}">{inner}</a>')
        elif part.startswith('{{') and part.endswith('}}'):
            out.append(_directive(p, part[2:-2], ed, ctx))
        else:
            out.append(svg2tikz.tex_escape(part) if ed == 'latex' else html.escape(part, quote=False))
    return ''.join(out)


def plain_fact(p, key, ctx):
    if key not in p.facts:
        fail(f'unknown fact {key!r} in {ctx}')
        return '??'
    f = p.facts[key]
    return f['value'] if isinstance(f, dict) else f


SELF_REPOS = re.compile(r'https://github\.com/(bitswell|bitswell-hq|theahaco|nidohq)/')


def tex_code(s):
    # Inline code in LaTeX: escaped, with break points after path and hash separators in
    # long spans, since \texttt does not hyphenate and table columns are narrow.
    out = svg2tikz.tex_escape(s)
    if len(s) > 16:
        out = re.sub(r'(/|\\_|\.|:|-|,)', r'\1\\allowbreak{}', out)
        out = re.sub(r'([0-9a-f]{8})(?=[0-9a-f]{8})', r'\1\\allowbreak{}', out)
    return out


def fact_value(p, key, ed, anon=False):
    f = p.facts[key]
    if not isinstance(f, dict):
        f = {'value': f}
    v = f['anon'] if anon and 'anon' in f else f['value']
    if f.get('markup'):
        return render(p, v, ed, key)
    if ed == 'latex':
        return r'\code{' + tex_code(v) + '}' if f.get('code') else svg2tikz.tex_escape(v)
    v = html.escape(v, quote=False)
    return f'<code>{v}</code>' if f.get('code') else v


# ---------------------------------------------------------------- HTML edition

def html_figure(p, fid):
    f = p.figures[fid]
    n = p.numbers['html']['fig:' + fid]
    title = render(p, f['title'], 'html', fid)
    cap = render(p, f.get('caption', ''), 'html', fid)
    cls = 'fig' + (' interactive' if f.get('interactive') else '')
    attrs = f' data-interactive="{f["interactive"]}"' if f.get('interactive') else ''
    if f.get('scenarios'):
        attrs += f' data-scenarios="{html.escape(f["scenarios"])}"'
    if f.get('kind') == 'table':
        body = html_table_body(p, p.tables[f['table']], f['table'])
        return (f'<figure class="{cls} fig-table" id="{fid}"{attrs}>{body}'
                f'<figcaption><span class="figure-title" id="{fid}-title">Figure {n}. {title}</span> {cap}</figcaption></figure>')
    if f.get('kind') == 'code':
        panels = ''.join(
            f'<div class="code-panel"><p class="panel-title">{render(p, pa["title"], "html", fid)}</p>'
            f'<pre>{html.escape(read(SRC / "figures" / f"{fid}.{pa["file"]}.txt").rstrip())}</pre></div>'
            for pa in f['panels'])
        return (f'<figure class="{cls} fig-code" id="{fid}"{attrs}><div class="code-panels">{panels}</div>'
                f'<figcaption><span class="figure-title" id="{fid}-title">Figure {n}. {title}</span> {cap}</figcaption></figure>')
    wide = read(SRC / 'figures' / f'{fid}.svg').strip()
    narrow_p = SRC / 'figures' / f'{fid}.narrow.svg'
    narrow = read(narrow_p).strip() if narrow_p.exists() else ''
    controls = ''
    cp = SRC / 'figures' / f'{fid}.controls.html'
    if cp.exists():
        controls = read(cp).strip()
    text_p = SRC / 'figures' / f'{fid}.text.html'
    steps = load_steps(fid)
    alt_body = html_steps(p, fid, steps) if steps else (read(text_p).strip() if text_p.exists() else '')
    alt = (f'<details class="fig-text" id="{fid}-text"><summary>Figure {n} as text</summary>{alt_body}</details>'
           if alt_body else '')
    return (f'<figure class="{cls}" id="{fid}"{attrs}>{controls}{wide}{narrow}'
            f'<figcaption><span class="figure-title" id="{fid}-title">Figure {n}. {title}</span> {cap}</figcaption>{alt}</figure>')


def slug(text):
    return re.sub(r'[^a-z0-9]+', '-', re.sub(r'`', '', text).lower()).strip('-')


GLOSS_LINK = re.compile(r'<dfn><a href="#glossary">(.*?)</a></dfn>')


def link_glossary(p, page):
    # Point each <dfn> link at its glossary row: exact term, a plural, or one name of a "A, B" row.
    t = p.tables.get('tab-glossary')
    if not t:
        return page
    names = {}
    for r in t['rows']:
        for name in [r[0]] + [x.strip() for x in re.split(r',| and ', r[0])]:
            names.setdefault(slug(name), slug(r[0]))

    def sub(m):
        key = slug(re.sub(r'<[^>]+>', '', m.group(1)))
        for k in (key, key[:-1] if key.endswith('s') else key, key[:-2] if key.endswith('es') else key):
            if k in names:
                return f'<dfn><a href="#{t["row_ids"]}-{names[k]}">{m.group(1)}</a></dfn>'
        fail(f'glossary link {m.group(1)!r} matches no glossary row')
        return m.group(0)
    return GLOSS_LINK.sub(sub, page)


def load_steps(fid):
    sp = SRC / 'figures' / f'{fid}.steps.json'
    return json.loads(read(sp)) if sp.exists() else None


def html_steps(p, fid, st):
    def item(x):
        scen = f' data-scenario="{x["scenario"]}"' if x.get('scenario') else ''
        exits = ''.join(f'<br>→ {render(p, e, "html", fid)}' for e in x.get('exits', []))
        return f'<li data-step="{x["n"]}"{scen}>{render(p, x["text"], "html", fid)}{exits}</li>'
    intro = f'<p>{render(p, st["intro"], "html", fid)}</p>' if st.get('intro') else ''
    outro = f'<p>{render(p, st["outro"], "html", fid)}</p>' if st.get('outro') else ''
    return intro + '<ol>' + ''.join(item(x) for x in st['steps']) + '</ol>' + outro


def latex_steps(p, fid, st):
    # An algorithm figure: one row per step (number, action, exits with codes, assurance).
    rows = []
    for x in st['steps']:
        if x.get('latex') is False:
            continue
        exits = '; '.join(render(p, e, 'latex', fid) for e in x.get('exits', [])) or '--'
        rows.append(f'{x["n"]} & {render(p, x["text"], "latex", fid)} & {exits} & {render(p, x.get("assurance", ""), "latex", fid)} \\\\')
    intro = render(p, st['intro'], 'latex', fid) + '\\par\\smallskip\n' if st.get('intro') else ''
    outro = '\n\\par\\smallskip ' + render(p, st['outro'], 'latex', fid) if st.get('outro') else ''
    return (intro + '\\footnotesize\\begin{tabularx}{\\linewidth}{@{}r>{\\raggedright\\arraybackslash}X'
            '>{\\raggedright\\arraybackslash}p{0.3\\linewidth}l@{}}\n\\toprule\n'
            '\\textbf{No.} & \\textbf{Step} & \\textbf{Exit and code} & \\textbf{Assurance} \\\\\n\\midrule\n'
            + '\n'.join(rows) + '\n\\bottomrule\n\\end{tabularx}' + outro)


def html_table_body(p, t, tid):
    head = ''.join(f'<th>{render(p, c, "html", tid)}</th>' for c in t['columns'])
    def tr(r):
        rid = f' id="{t["row_ids"]}-{slug(r[0])}"' if t.get('row_ids') else ''
        return f'<tr{rid}>' + ''.join(f'<td>{render(p, c, "html", tid)}</td>' for c in r) + '</tr>'
    rows = ''.join(tr(r) for r in t['rows'])
    cls = f' class="{t["html_class"]}"' if t.get('html_class') else ''
    return f'<div class="table-wrap"><table{cls}><thead><tr>{head}</tr></thead><tbody>{rows}</tbody></table></div>'


def html_table(p, tid, numbered=True):
    t = p.tables[tid]
    body = html_table_body(p, t, tid)
    lead = f'Table {p.numbers["html"]["tab:" + tid]}. ' if numbered else ''
    cap = (f'<caption>{lead}{render(p, t["title"], "html", tid)} '
           f'{render(p, t.get("caption", ""), "html", tid)}</caption>')
    return body.replace('</tbody></table>', f'</tbody>{cap}</table>', 1).replace('<table', f'<table id="{tid}"', 1)


def html_statements(p):
    def item(st):
        data = ''.join(f' data-{k}="{html.escape(" ".join(st[k]))}"' for k in ('objects', 'assumes') if st.get(k))
        label = f'{st["label"]} · ' if st.get('label') else ''
        return (f'<li id="stmt-{st["id"]}"{data}><strong>{label}{render(p, st["title"], "html", st["id"])}</strong>'
                f'<p>{render(p, st["text"], "html", st["id"])}</p>'
                f'<div class="artifact">{render(p, st["evidence"], "html", st["id"])}</div></li>')
    return '<ol class="proof-list">' + ''.join(item(st) for st in p.statements) + '</ol>'


def html_references(p):
    items = []
    for k in p.cite_order:
        e = p.bib[k]
        items.append(f'<li id="ref-{k}">{bib_html(e)}</li>')
    return '<ol class="refs">' + ''.join(items) + '</ol>'


TEX_ACCENTS = {r"\\'U": 'Ú', r"\\'u": 'ú', r'\\"o': 'ö', r'\\"u': 'ü', r"\\'e": 'é', r'\\#': '#', r'\\_': '_', r'\\&': '&'}


def bib_text(s):
    for k, v in TEX_ACCENTS.items():
        s = re.sub(k, v, s)
    return re.sub(r'[{}]', '', s).replace('--', '–')


def bib_html(e):
    def t(s):
        return html.escape(bib_text(s), quote=False)
    parts = []
    if e.get('author'):
        names = [n.strip() for n in e['author'].split(' and ')]
        parts.append(t(names[0] + ' et al.' if len(names) > 3 else ', '.join(names[:-1]) + (' and ' if len(names) > 1 else '') + names[-1])
                     + ('' if len(names) > 3 else '.'))
    title = t(e.get('title', ''))
    url = e.get('url')
    parts.append(f'<a href="{html.escape(url)}">{title}.</a>' if url else title + '.')
    venue = e.get('booktitle') or e.get('journal') or e.get('howpublished') or e.get('publisher')
    vol = (f' {t(e["volume"])}' + (f'({t(e["number"])})' if e.get('number') else '')) if e.get('volume') else ''
    tail = ', '.join(x for x in [(f'<em>{t(venue)}</em>{vol}' if e.get('journal') else (t(venue) if venue else '')),
                                 t(e['year']) if e.get('year') else ''] if x)
    if tail:
        parts.append(tail + '.')
    if e.get('note'):
        parts.append(t(e['note']) + ('' if e['note'].endswith('.') else '.'))
    return ' '.join(parts)


DIRECTIVE = re.compile(r'\{\{([a-z-]+):?([^}]*)\}\}')


def expand_html(p, text, ctx, blocks, seen):
    def sub(m):
        kind, arg = m.group(1), m.group(2)
        if kind == 'fig':
            seen['fig'].append(arg)
            return html_figure(p, arg)
        if kind == 'table':
            tid, _, flag = arg.partition('|')
            if flag == 'unnumbered':
                return html_table(p, tid, numbered=False)
            seen['tab'].append(tid)
            return html_table(p, tid)
        if kind == 'if-walkthrough':
            return expand_html(p, read(SRC / 'html' / arg), arg, blocks, seen) if WALKTHROUGH.exists() else ''
        if kind == 'statements':
            return html_statements(p)
        if kind == 'block':
            if arg not in blocks:
                fail(f'missing carried block {arg}')
            return blocks.get(arg, '')
        if kind == 'include':
            return expand_html(p, read(SRC / 'html' / arg), arg, blocks, seen)
        if kind == 'references':
            return html_references(p)
        return render(p, m.group(0), 'html', ctx)
    return DIRECTIVE.sub(sub, text)


def build_html(p, blocks):
    o = p.outline
    seen = {'fig': [], 'tab': []}
    # Citation numbers follow first use across the HTML sections.
    order = []
    for s in p.sections:
        if p.in_edition(s, 'html'):
            for k in re.findall(r'\{\{cite:([^}]+)\}\}', read(SRC / 'html' / 'sections' / f'{s["id"]}.html')):
                for kk in k.split(','):
                    if kk not in order:
                        order.append(kk)
    order += [k for k in p.bib if k not in order and p.bib[k].get('_list', 'html') != 'latex' and 'html' in p.bib[k].get('editions', 'html latex')]
    p.cite_order = order
    p.cite_number = lambda k: order.index(k) + 1 if k in order else '?'

    nav = ['<nav aria-label="Paper sections"><a class="brand" href="#top">PERCH</a><div class="nav-links">']
    for s in p.sections:
        if p.in_edition(s, 'html') and s.get('nav'):
            n = p.numbers['html'].get('sec:' + s['id'])
            label = f'{n} · {s["nav"]}' if n and n.isdigit() else s['nav']
            nav.append(f'<a href="#{s["id"]}">{html.escape(label)}</a>')
    nav.append(f'</div><small>{o["nav_note"]}</small></nav>')
    css = '\n'.join(read(SRC / 'html' / n).strip() for n in ('style.css', 'page.css', 'figures.css')
                    if (SRC / 'html' / n).exists())
    head = read(SRC / 'html' / 'head.html').replace('{{style}}', css)
    parts = [expand_html(p, head, 'head', blocks, seen).strip(), ''.join(nav), '']
    wt = ' data-walkthrough="../oz-walkthrough/index.html"' if WALKTHROUGH.exists() else ''
    parts.append(f'<main id="main"{wt}>' + read(SRC / 'html' / 'sprite.svg').strip()
                 + expand_html(p, read(SRC / 'html' / 'header.html'), 'header', blocks, seen).strip())
    parts.append(expand_html(p, read(SRC / 'html' / 'abstract.html'), 'abstract', blocks, seen).strip())
    for s in p.sections:
        if not p.in_edition(s, 'html'):
            continue
        n = p.numbers['html'].get('sec:' + s['id'])
        body = expand_html(p, read(SRC / 'html' / 'sections' / f'{s["id"]}.html'), s['id'], blocks, seen).strip()
        head = f'<h2><span class="n">{n}</span>{html.escape(s["title_html"])}</h2>' if n else f'<h2>{html.escape(s["title_html"])}</h2>'
        alias = ''.join(f'<span class="alias" id="{a}"></span>' for a in s.get('aliases', []))
        parts.append(f'<section id="{s["id"]}">{alias}{head}{body}</section>')
    parts.append(expand_html(p, read(SRC / 'html' / 'footer.html'), 'footer', blocks, seen).strip())
    parts.append('</main></div>')
    parts.append(blocks['EMBEDDED_ASSETS'])
    parts.append('<script>' + read(SRC / 'html' / 'figures.js').strip() + '</script>')
    parts.append('</body></html>\n')
    want_f = [f for s in p.sections if p.in_edition(s, 'html') for f in s.get('figures', []) if p.in_edition(p.figures[f], 'html')]
    want_t = [t for s in p.sections if p.in_edition(s, 'html') for t in s.get('tables', []) if p.in_edition(p.tables[t], 'html')]
    if seen['fig'] != want_f:
        fail(f'HTML figure order {seen["fig"]} != outline {want_f}')
    if seen['tab'] != want_t:
        fail(f'HTML table order {seen["tab"]} != outline {want_t}')
    page = link_glossary(p, '\n'.join(parts))
    # build-page.mjs refreshes these blocks in place, so each must survive a rebuild.
    for name in CARRIED:
        if page.count(f'<!-- {name}_START -->') != 1:
            fail(f'the {name} block must appear exactly once in the HTML edition (place {{{{block:{name}}}}})')
    return page


MEASURED = {'Install transfer grant': 'cpu-install-grant', 'Spend six demo XLM': 'cpu-spend-6',
            'Complete guardian recovery': 'cpu-complete', 'Spend with fresh grant': 'cpu-spend-grant2',
            'Losing game guess': 'cpu-guess-1', 'Winning game guess': 'cpu-win'}


def check_measurements(p, block):
    for label, value in re.findall(r'<tr><td>([^<]+)</td><td>([\d,]+)</td>', block):
        key = MEASURED.get(label)
        if key and key in p.facts and p.facts[key]['value'] != value:
            fail(f'MEASUREMENTS row {label!r} is {value} but fact {key} is {p.facts[key]["value"]}')


def carried_blocks(text):
    blocks = {}
    for name in CARRIED:
        m = re.search(f'<!-- {name}_START -->.*?<!-- {name}_END -->', text, re.S)
        if not m:
            fail(f'index.html lacks the {name} block')
        else:
            blocks[name] = m.group(0)
    return blocks


# ---------------------------------------------------------------- LaTeX edition

LINT = []


def lint_svg(fid, f):
    for suffix in ('svg', 'narrow.svg'):
        path = SRC / 'figures' / f'{fid}.{suffix}'
        if not path.exists():
            continue
        t = read(path)
        if ' style="' in t or ' opacity="' in t:
            LINT.append(f'{path.name}: no style= or opacity attributes')
        for size in re.findall(r'font-size="([\d.]+)"', t):
            if not 14 <= float(size) <= 17:
                LINT.append(f'{path.name}: font-size {size} outside 14-17')
                break
        vb = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', t)
        want = 340 if suffix == 'narrow.svg' else 760
        if vb and float(vb.group(1)) != want:
            LINT.append(f'{path.name}: width {vb.group(1)}, want {want}')
        if suffix == 'svg' and vb and float(vb.group(2)) > 900 and f.get('kind') != 'algorithm':
            LINT.append(f'{path.name}: height {vb.group(2)} > 900 (redesign, or make it an algorithm figure)')


def latex_figure(p, fid):
    f = p.figures[fid]
    env = "figure*" if f.get("latex_span") == "full" else "figure"
    cap = r'\textbf{' + render(p, f['title'], 'latex', fid) + '} ' + render(p, f.get('caption', ''), 'latex', fid)
    if f.get('kind') == 'table':
        body = latex_tabular(p, p.tables[f['table']], f['table'])
    elif f.get('kind') == 'algorithm':
        st = load_steps(fid)
        if not st:
            fail(f'{fid}: kind algorithm needs source/figures/{fid}.steps.json')
            return ''
        body = latex_steps(p, fid, st)
    elif f.get('kind') == 'code':
        parts = []
        for pa in f['panels']:
            w = pa.get('latex_width', 1)
            text = read(SRC / 'figures' / f'{fid}.{pa["file"]}.txt').rstrip()
            parts.append(f'\\begin{{minipage}}[t]{{{w}\\linewidth}}\\footnotesize\\textbf{{{render(p, pa["title"], "latex", fid)}}}\n'
                         f'\\begin{{Verbatim}}[fontsize=\\scriptsize,frame=single,framesep=1.5mm]\n{text}\n\\end{{Verbatim}}\n\\end{{minipage}}')
        body = ''
        for i, part in enumerate(parts):
            body += part
            if i + 1 < len(parts):
                a, b = f['panels'][i].get('latex_width', 1), f['panels'][i + 1].get('latex_width', 1)
                body += '\\hfill\n' if a < 1 and b < 1 and a + b <= 1 else '\\par\\medskip\n'
    else:
        tikz, w, h = svg2tikz.convert(read(SRC / 'figures' / f'{fid}.svg'))
        maxh = f.get('latex_max_height', '0.8\\textheight')
        body = (r'\begin{adjustbox}{max width=\linewidth,max totalheight=' + maxh + '}%\n'
                + tikz + '%\n\\end{adjustbox}')
    return (f'% Generated from source/outline.json and source/figures/{fid}.svg; edit those.\n'
            f'\\begin{{{env}}}[{f.get("latex_place", "tbp")}]\n\\centering\n{body}\n'
            f'\\caption{{{cap}}}\\label{{fig:{fid}}}\n\\end{{{env}}}\n')


def latex_tabular(p, t, tid):
    # Default: every column wraps (ragged right) across the text width. A table
    # may give `latex_colspec` in tabularx terms, using L for a wrapping column.
    spec = t.get('latex_colspec') or ('@{}' + 'L' * len(t['columns']) + '@{}')
    size = t.get('latex_size', 'scriptsize')
    head = ' & '.join(r'\textbf{' + render(p, c, 'latex', tid) + '}' for c in t['columns']) + r' \\'
    rows = '\n'.join(' & '.join(render(p, c, 'latex', tid) for c in r) + r' \\' for r in t['rows'])
    return (f'\\{size}\\begin{{tabularx}}{{\\linewidth}}{{{spec}}}\n\\toprule\n{head}\n\\midrule\n{rows}\n'
            f'\\bottomrule\n\\end{{tabularx}}')


def latex_table(p, tid):
    t = p.tables[tid]
    cap = r'\textbf{' + render(p, t['title'], 'latex', tid) + '} ' + render(p, t.get('caption', ''), 'latex', tid)
    if t.get('latex_long'):
        # A table that may break across pages (xltabular): not a float.
        spec = t.get('latex_colspec') or ('@{}' + 'L' * len(t['columns']) + '@{}')
        head = ' & '.join(r'\textbf{' + render(p, c, 'latex', tid) + '}' for c in t['columns']) + r' \\'
        rows = '\n'.join(' & '.join(render(p, c, 'latex', tid) for c in r) + r' \\' for r in t['rows'])
        return (f'% Generated from source/tables/{tid}.json; edit that.\n'
                f'{{\\{t.get("latex_size", "scriptsize")}\n\\begin{{xltabular}}{{\\linewidth}}{{{spec}}}\n'
                f'\\caption{{{cap}}}\\label{{tab:{tid}}}\\\\\n\\toprule\n{head}\n\\midrule\n\\endfirsthead\n'
                f'\\toprule\n{head}\n\\midrule\n\\endhead\n\\bottomrule\n\\endlastfoot\n{rows}\n\\end{{xltabular}}}}\n')
    env = "table*" if t.get("latex_span") == "full" else "table"
    return (f'% Generated from source/tables/{tid}.json; edit that.\n'
            f'\\begin{{{env}}}[{t.get("latex_place", "tbp")}]\n\\centering\n'
            f'\\caption{{{cap}}}\\label{{tab:{tid}}}\n{latex_tabular(p, t, tid)}\n\\end{{{env}}}\n')


def latex_statements(p, body):
    # Statements with "latex_body": true go in the body (generated/statements-body.tex);
    # the rest in the formal appendix (generated/statements-appendix.tex).
    out = ['% Generated from source/statements.json; edit that.']
    for st in p.statements:
        if bool(st.get('latex_body')) != body:
            continue
        text = st.get('latex') or render(p, st['text'], 'latex', st['id'])
        hyp = ''
        if st.get('hypotheses'):
            hyp = '\\statementhypotheses{' + render(p, st['hypotheses'], 'latex', st['id']) + '}\n'
        out.append(f'\\begin{{perchstatement}}{{{st.get("label", "")}}}{{{render(p, st["title"], "latex", st["id"])}}}\\label{{stmt:{st["id"]}}}\n'
                   f'{text}\n{hyp}\\statementevidence{{{render(p, st["evidence"], "latex", st["id"])}}}\n'
                   '\\end{perchstatement}')
    return '\n'.join(out) + '\n'


def latex_facts(p):
    out = ['% Generated from source/facts.json; edit that.']
    for k in sorted(p.facts):
        v = fact_value(p, k, 'latex')
        f = p.facts[k]
        if isinstance(f, dict) and 'anon' in f:
            v = '\\ifdefined\\anonymous ' + fact_value(p, k, 'latex', anon=True) + '\\else ' + v + '\\fi'
        out.append(f'\\expandafter\\def\\csname perchfact@{k}\\endcsname{{{v}}}')
    return '\n'.join(out) + '\n'


def latex_body(p):
    out = ['% Generated from source/outline.json; edit that.']
    appendix = False
    for s in p.sections:
        if not p.in_edition(s, 'latex'):
            continue
        if s.get('appendix') and not appendix:
            out.append(r'\appendix')
            appendix = True
        title = svg2tikz.tex_escape(s.get('title_latex', s['title_html']))
        cmd = r'\section*' if s.get('unnumbered') else r'\section'
        out.append(f'{cmd}{{{title}}}\\label{{sec:{s["id"]}}}')
        out.append(f'\\input{{sections/{s["id"]}}}')
    return '\n'.join(out) + '\n'


def latex_bib(p):
    out = ['% Generated from source/refs.bib; edit that.']
    src = read(SRC / 'refs.bib')
    out.append(re.sub(r'^\s*editions\s*=\s*\{[^}]*\},?\n', '', src, flags=re.M))
    return '\n'.join(out)


def build_latex(p):
    files = {'body.tex': latex_body(p), 'facts.tex': latex_facts(p),
             'colors.tex': '% Generated from build/svg2tikz.py (the page\'s light theme).\n' + svg2tikz.color_definitions() + '\n',
             'statements-body.tex': latex_statements(p, True),
             'statements-appendix.tex': latex_statements(p, False), 'refs.bib': latex_bib(p)}
    for f in p.figures:
        if p.figures[f].get('kind') not in ('table', 'code'):
            lint_svg(f, p.figures[f])
        if p.in_edition(p.figures[f], 'latex'):
            files[f'figures/{f}.tex'] = latex_figure(p, f)
    for t in p.tables:
        if p.in_edition(p.tables[t], 'latex') and not any(fg.get('table') == t for fg in p.figures.values()):
            files[f'tables/{t}.tex'] = latex_table(p, t)
    # Section sources must place exactly the outline's figures and tables, in order.
    seen_f, seen_t, used_facts = [], [], set()
    for s in p.sections:
        if not p.in_edition(s, 'latex'):
            continue
        sp = LATEX / 'sections' / f'{s["id"]}.tex'
        if not sp.exists():
            fail(f'missing {sp.relative_to(ROOT)}')
            continue
        tex = read(sp)
        seen_f += re.findall(r'\\perchfigure\{([^}]+)\}', tex)
        seen_t += re.findall(r'\\perchtable\{([^}]+)\}', tex)
        used_facts |= set(re.findall(r'\\fact\{([^}]+)\}', tex))
    for extra in ('abstract.tex',):
        if (LATEX / 'sections' / extra).exists():
            used_facts |= set(re.findall(r'\\fact\{([^}]+)\}', read(LATEX / 'sections' / extra)))
    figs, tabs = p.placed('latex', 'figures'), p.placed('latex', 'tables')
    want_f = [f for s in p.sections if p.in_edition(s, 'latex') for f in figs[s['id']] if p.in_edition(p.figures[f], 'latex')]
    want_t = [t for s in p.sections if p.in_edition(s, 'latex') for t in tabs[s['id']] if p.in_edition(p.tables[t], 'latex')]
    if seen_f != want_f:
        fail(f'LaTeX figure order {seen_f} != outline {want_f}')
    if seen_t != want_t:
        fail(f'LaTeX table order {seen_t} != outline {want_t}')
    for k in sorted(used_facts - set(p.facts)):
        fail(f'LaTeX uses unknown fact {k!r}')
    return files


# ---------------------------------------------------------------- main

def main(argv):
    check = '--check' in argv
    p = Paper()
    current = read(INDEX)
    blocks = carried_blocks(current)
    if 'MEASUREMENTS' in blocks:
        check_measurements(p, blocks['MEASUREMENTS'])
    page = build_html(p, blocks) if not ERRORS else ''
    latex = build_latex(p)
    if LINT:
        print('\n'.join(('error: ' if check else 'warning: ') + 'figure lint: ' + e for e in LINT), file=sys.stderr)
        if check:
            ERRORS.extend(LINT)
    if ERRORS:
        print('\n'.join('error: ' + e for e in ERRORS if e not in LINT), file=sys.stderr)
        return 1
    stale = []
    if page != current:
        stale.append(str(INDEX.relative_to(ROOT.parent.parent)))
    for rel, text in latex.items():
        path = GEN / rel
        if not path.exists() or read(path) != text:
            stale.append(str(path.relative_to(ROOT.parent.parent)))
    known = {GEN / r for r in latex}
    orphans = [q for q in GEN.rglob('*') if q.is_file() and q not in known] if GEN.exists() else []
    if check:
        if stale or orphans:
            print('stale generated files (run python3 docs/perch-paper/build/paper.py):', file=sys.stderr)
            for s in stale + [str(o.relative_to(ROOT.parent.parent)) for o in orphans]:
                print('  ' + s, file=sys.stderr)
            return 1
        print('paper sources and generated files agree')
        return 0
    INDEX.write_text(page, encoding='utf-8')
    for rel, text in latex.items():
        (GEN / rel).parent.mkdir(parents=True, exist_ok=True)
        (GEN / rel).write_text(text, encoding='utf-8')
    for o in orphans:
        o.unlink()
    print(f'wrote {INDEX.relative_to(ROOT.parent.parent)} ({len(page.encode()):,} bytes) and {len(latex)} LaTeX includes')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
