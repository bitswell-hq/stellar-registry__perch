"""Write ../nido.html: the standalone sign-dialog page with Nido's theme (theme.css) linked. The page and the
component are otherwise unchanged. Run from this directory: python3 build.py"""
from pathlib import Path
here = Path(__file__).resolve().parent
page = (here.parent / 'standalone.html').read_text(encoding='utf-8')
link = '<link rel="stylesheet" href="nido/theme.css">\n'
assert page.count('</head>') == 1
(here.parent / 'nido.html').write_text(page.replace('</head>', link + '</head>', 1), encoding='utf-8')
