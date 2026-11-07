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
        self.numbers = {ed: self._number(ed) for ed in ('html', 'latex')}

    def in_edition(self, item, ed):
        return ed in item.get('editions', ['html', 'latex'])

    def _number(self, ed):
        nums, fig, tab, main, app = {}, 0, 0, 0, 0
        for s in self.sections:
            if not self.in_edition(s, ed):
                continue
            if s.get('appendix'):
                nums['sec:' + s['id']] = 'ABCDEFGH'[app]
                app += 1
            elif not s.get('unnumbered'):
                main += 1
                nums['sec:' + s['id']] = f'{main:02d}' if ed == 'html' else str(main)
            for f in s.get('figures', []):
                if self.in_edition(self.figures[f], ed):
                    fig += 1
                    nums['fig:' + f] = str(fig)
            for t in s.get('tables', []):
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
        return fact_value(p, arg, ed)
    if kind == 'ref':
        if arg not in p.numbers[ed] and not (arg.startswith('sec:') and arg[4:] in p.sec_ids):
            fail(f'unknown or excluded reference {arg!r} in {ctx} ({ed})')
            return '??'
        if ed == 'latex':
            pre = {'sec': r'\S', 'fig': r'Figure~', 'tab': r'Table~'}[arg[:3]]
            return pre + r'\ref{' + arg + '}'
        n = p.numbers['html'].get(arg, '??')
        label = {'sec': '§', 'fig': 'Figure ', 'tab': 'Table '}[arg[:3]] + n
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
            out.append(r'\code{' + svg2tikz.tex_escape(inner) + '}' if ed == 'latex'
                       else '<code>' + html.escape(inner, quote=False) + '</code>')
        elif part.startswith('**') and part.endswith('**'):
            inner = render(p, part[2:-2], ed, ctx)
            out.append(r'\textbf{' + inner + '}' if ed == 'latex' else f'<strong>{inner}</strong>')
        elif part.startswith('*') and part.endswith('*') and len(part) > 2:
            inner = render(p, part[1:-1], ed, ctx)
            out.append(r'\emph{' + inner + '}' if ed == 'latex' else f'<em>{inner}</em>')
        elif part.startswith('[') and part.endswith(')') and '](' in part:
            text, url = part[1:-1].split('](', 1)
            inner = render(p, text, ed, ctx)
            out.append(r'\href{' + url.replace('%', r'\%').replace('#', r'\#') + '}{' + inner + '}' if ed == 'latex'
                       else f'<a href="{html.escape(url)}">{inner}</a>')
        elif part.startswith('{{') and part.endswith('}}'):
            out.append(_directive(p, part[2:-2], ed, ctx))
        else:
            out.append(svg2tikz.tex_escape(part) if ed == 'latex' else html.escape(part, quote=False))
    return ''.join(out)


def fact_value(p, key, ed):
    f = p.facts[key]
    v = f['value'] if isinstance(f, dict) else f
    code = isinstance(f, dict) and f.get('code')
    if ed == 'latex':
        v = svg2tikz.tex_escape(v)
        return r'\code{' + v + '}' if code else v
    v = html.escape(v, quote=False)
    return f'<code>{v}</code>' if code else v


# ---------------------------------------------------------------- HTML edition

def html_figure(p, fid):
    f = p.figures[fid]
    n = p.numbers['html']['fig:' + fid]
    title = render(p, f['title'], 'html', fid)
    cap = render(p, f.get('caption', ''), 'html', fid)
    cls = 'fig' + (' interactive' if f.get('interactive') else '')
    attrs = f' data-interactive="{f["interactive"]}"' if f.get('interactive') else ''
    if f.get('kind') == 'table':
        body = html_table_body(p, p.tables[f['table']], f['table'])
        return (f'<figure class="{cls} fig-table" id="{fid}"{attrs}>{body}'
                f'<figcaption><span class="figure-title" id="{fid}-title">Figure {n}. {title}</span> {cap}</figcaption></figure>')
    wide = read(SRC / 'figures' / f'{fid}.svg').strip()
    narrow_p = SRC / 'figures' / f'{fid}.narrow.svg'
    narrow = read(narrow_p).strip() if narrow_p.exists() else ''
    controls = ''
    cp = SRC / 'figures' / f'{fid}.controls.html'
    if cp.exists():
        controls = read(cp).strip()
    text_p = SRC / 'figures' / f'{fid}.text.html'
    alt = (f'<details class="fig-text" id="{fid}-text"><summary>Figure {n} as text</summary>{read(text_p).strip()}</details>'
           if text_p.exists() else '')
    return (f'<figure class="{cls}" id="{fid}"{attrs}>{controls}{wide}{narrow}'
            f'<figcaption><span class="figure-title" id="{fid}-title">Figure {n}. {title}</span> {cap}</figcaption>{alt}</figure>')


def html_table_body(p, t, tid):
    head = ''.join(f'<th>{render(p, c, "html", tid)}</th>' for c in t['columns'])
    rows = ''.join('<tr>' + ''.join(f'<td>{render(p, c, "html", tid)}</td>' for c in r) + '</tr>' for r in t['rows'])
    cls = f' class="{t["html_class"]}"' if t.get('html_class') else ''
    return f'<div class="table-wrap"><table{cls}><thead><tr>{head}</tr></thead><tbody>{rows}</tbody></table></div>'


def html_table(p, tid):
    t = p.tables[tid]
    n = p.numbers['html']['tab:' + tid]
    body = html_table_body(p, t, tid)
    cap = (f'<caption>Table {n}. {render(p, t["title"], "html", tid)} '
           f'{render(p, t.get("caption", ""), "html", tid)}</caption>')
    return body.replace('</tbody></table>', f'</tbody>{cap}</table>', 1).replace('<table', f'<table id="{tid}"', 1)


def html_statements(p):
    items = ''.join(
        f'<li id="stmt-{st["id"]}"><strong>{render(p, st["title"], "html", st["id"])}</strong>'
        f'<p>{render(p, st["text"], "html", st["id"])}</p>'
        f'<div class="artifact">{render(p, st["evidence"], "html", st["id"])}</div></li>'
        for st in p.statements)
    return f'<ol class="proof-list">{items}</ol>'


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


DIRECTIVE = re.compile(r'\{\{([a-z]+):?([^}]*)\}\}')


def expand_html(p, text, ctx, blocks, seen):
    def sub(m):
        kind, arg = m.group(1), m.group(2)
        if kind == 'fig':
            seen['fig'].append(arg)
            return html_figure(p, arg)
        if kind == 'table':
            seen['tab'].append(arg)
            return html_table(p, arg)
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
    head = read(SRC / 'html' / 'head.html').replace('{{style}}', read(SRC / 'html' / 'style.css').strip())
    parts = [expand_html(p, head, 'head', blocks, seen).strip(), ''.join(nav), '']
    parts.append('<main id="main">' + read(SRC / 'html' / 'sprite.svg').strip()
                 + expand_html(p, read(SRC / 'html' / 'header.html'), 'header', blocks, seen).strip())
    parts.append(expand_html(p, read(SRC / 'html' / 'abstract.html'), 'abstract', blocks, seen).strip())
    for s in p.sections:
        if not p.in_edition(s, 'html'):
            continue
        n = p.numbers['html'].get('sec:' + s['id'])
        body = expand_html(p, read(SRC / 'html' / 'sections' / f'{s["id"]}.html'), s['id'], blocks, seen).strip()
        head = f'<h2><span class="n">{n}</span>{html.escape(s["title_html"])}</h2>' if n else f'<h2>{html.escape(s["title_html"])}</h2>'
        parts.append(f'<section id="{s["id"]}">{head}{body}</section>')
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
    return '\n'.join(parts)


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

def latex_figure(p, fid):
    f = p.figures[fid]
    env = "figure*" if f.get("latex_span") == "full" else "figure"
    cap = r'\textbf{' + render(p, f['title'], 'latex', fid) + '} ' + render(p, f.get('caption', ''), 'latex', fid)
    if f.get('kind') == 'table':
        body = latex_tabular(p, p.tables[f['table']], f['table'])
    else:
        tikz, w, h = svg2tikz.convert(read(SRC / 'figures' / f'{fid}.svg'))
        maxh = f.get('latex_max_height', '0.8\\textheight')
        body = (r'\begin{adjustbox}{max width=\linewidth,max totalheight=' + maxh + '}%\n'
                + tikz + '%\n\\end{adjustbox}')
    return (f'% Generated from source/outline.json and source/figures/{fid}.svg; edit those.\n'
            f'\\begin{{{env}}}[{f.get("latex_place", "tbp")}]\n\\centering\n{body}\n'
            f'\\caption{{{cap}}}\\label{{fig:{fid}}}\n\\end{{{env}}}\n')


def latex_tabular(p, t, tid):
    spec = t.get('latex_colspec') or ('@{}' + 'l' * len(t['columns']) + '@{}')
    size = t.get('latex_size', 'footnotesize')
    head = ' & '.join(r'\textbf{' + render(p, c, 'latex', tid) + '}' for c in t['columns']) + r' \\'
    rows = '\n'.join(' & '.join(render(p, c, 'latex', tid) for c in r) + r' \\' for r in t['rows'])
    return (f'\\{size}\\begin{{tabular}}{{{spec}}}\n\\toprule\n{head}\n\\midrule\n{rows}\n\\bottomrule\n\\end{{tabular}}')


def latex_table(p, tid):
    t = p.tables[tid]
    env = "table*" if t.get("latex_span") == "full" else "table"
    cap = r'\textbf{' + render(p, t['title'], 'latex', tid) + '} ' + render(p, t.get('caption', ''), 'latex', tid)
    return (f'% Generated from source/tables/{tid}.json; edit that.\n'
            f'\\begin{{{env}}}[{t.get("latex_place", "tbp")}]\n\\centering\n'
            f'\\caption{{{cap}}}\\label{{tab:{tid}}}\n{latex_tabular(p, t, tid)}\n\\end{{{env}}}\n')


def latex_statements(p):
    out = ['% Generated from source/statements.json; edit that.']
    for st in p.statements:
        body = st.get('latex') or render(p, st['text'], 'latex', st['id'])
        out.append(f'\\begin{{statement}}[{render(p, st["title"], "latex", st["id"])}]\\label{{stmt:{st["id"]}}}\n'
                   f'{body}\n\\statementevidence{{{render(p, st["evidence"], "latex", st["id"])}}}\n'
                   '\\end{statement}')
    return '\n'.join(out) + '\n'


def latex_facts(p):
    out = ['% Generated from source/facts.json; edit that.']
    for k in sorted(p.facts):
        out.append(f'\\expandafter\\def\\csname perchfact@{k}\\endcsname{{{fact_value(p, k, "latex")}}}')
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
             'statements.tex': latex_statements(p), 'refs.bib': latex_bib(p)}
    for f in p.figures:
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
    want_f = [f for s in p.sections if p.in_edition(s, 'latex') for f in s.get('figures', []) if p.in_edition(p.figures[f], 'latex')]
    want_t = [t for s in p.sections if p.in_edition(s, 'latex') for t in s.get('tables', []) if p.in_edition(p.tables[t], 'latex')]
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
    page = build_html(p, blocks) if not ERRORS else ''
    latex = build_latex(p)
    if ERRORS:
        print('\n'.join('error: ' + e for e in ERRORS), file=sys.stderr)
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
