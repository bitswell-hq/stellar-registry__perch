"""Write ../nido.html: the standalone sign-dialog page with Nido's theme (theme.css) linked and the page theme
and dialog style selects hidden, since Nido's light-only theme overrides both. The component and the page are
otherwise unchanged. Run from this directory: python3 build.py"""
from pathlib import Path
here = Path(__file__).resolve().parent
page = (here.parent / 'standalone.html').read_text(encoding='utf-8')
link = '<link rel="stylesheet" href="nido/theme.css">\n<style>.controls label[hidden] { display: none; }</style>\n'
assert page.count('</head>') == 1
page = page.replace('</head>', link + '</head>', 1)
for select in ('<select id="theme">', '<select id="brand">'):
    label = '<label>' + page.split(select, 1)[0].rsplit('<label>', 1)[-1] + select
    assert page.count(select) == 1 and page.count(label) == 1
    page = page.replace(label, '<label hidden>' + label[len('<label>'):], 1)
(here.parent / 'nido.html').write_text(page, encoding='utf-8')
