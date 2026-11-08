"""fig-contexts: one authorization, several contexts, using the game grant."""
from figlib import SVG, num, wrap

FID = 'fig-contexts'

PANELS = [
    ('(a) A lost guess',
     ['`game.guess(0, guesser)`', ('`xlm.transfer(account → game,\n1 XLM)`', 1)],
     [('tested', '[0] `game.guess` → the game grant, rule `guess`: allowed'),
      ('hi', '[1] `xlm.transfer` → the game grant, rule `spend` (2 XLM per 100 ledgers): allowed, charges 1 XLM')]),
    ('(b) A winning guess',
     ['`game.guess(n, guesser)`'],
     [('tested', '[0] `game.guess` → the game grant, rule `guess`: allowed'),
      ('note', 'The game then pays out its balance (3 XLM) under its own authority. Nothing is charged to the game grant.')]),
    ('(c) A direct transfer',
     [('`xlm.transfer(account → anyone,\n≤ 2 XLM)`', 0)],
     [('tested', '[0] `xlm.transfer` → the game grant, rule `spend`: allowed at the pinned build'),
      ('draft', 'A draft that checks exact entry roots would refuse it.')]),
]


def tree(s, x, y, w, calls, size):
    """Signed-tree box; a child call gets an L-shaped connector drawn as a path."""
    lh = round(size * 1.33)
    rows = []
    for c in calls:
        content, depth = (c, 0) if isinstance(c, str) else c
        indent = 18 * depth
        for i, ln in enumerate(wrap(content, w - 24 - indent - 8, size)):
            rows.append((ln, indent + (8 if i else 0), depth and i == 0))
    h = 16 + lh + len(rows) * lh + 4
    s.box(x, y, w, h, 'key')
    s.text(x + 12, y + 16 + round(0.78 * size), 'Signed tree', 't', size)
    ty = y + 16 + round(0.78 * size) + lh
    for ln, indent, elbow in rows:
        if elbow:
            ex = x + 16
            s.path(f'M{num(ex)} {num(ty - lh + 4)}V{num(ty - 5)}H{num(ex + 10)}', 'fl-m')
        s.text(x + 12 + indent, ty, ln, 'x', size)
        ty += lh
    return h


def ctx_box(s, x, y, w, kind, content, size, minh=0):
    if kind == 'note':
        return s.textbox(x, y, w, [(content, 'm', size)], 'none', size=size, minh=minh)
    if kind == 'draft':
        pw = s.pill(x, y + 4, 'DRAFT', 'm', 14)
        lines = wrap(content, w - pw - 10, size)
        s.lines(x + pw + 10, y + 22, lines, 'm', size)
        return 30 + (len(lines) - 1) * round(size * 1.33)
    h = s.textbox(x, y, w, [(content, 'x', size)], kind, size=size, minh=minh)
    s.check(x - 22, y + 14, 'gl-a' if kind == 'hi' else 'gl')
    return h


def wide():
    s = SVG(FID, 760)
    y = 0
    for title, calls, ctxs in PANELS:
        s.text(2, y + 17, title, 't', 16)
        top = y + 28
        th = tree(s, 2, top, 318, calls, 15)
        cy = top
        for i, (kind, content) in enumerate(ctxs):
            h = ctx_box(s, 356, cy, 402, kind, content, 15)
            if kind not in ('note', 'draft'):
                s.path(f'M320 {num(top + 22)}H326V{num(cy + 20)}H330', 'fl-m', marker_end='url(#perch-ah-m)')
            cy += h + 8
        y = max(top + th, cy - 8) + 26
    s.h = y - 24
    return s


def narrow():
    s = SVG(FID, 340, narrow=True)
    y = 0
    for title, calls, ctxs in PANELS:
        s.text(2, y + 17, title, 't', 15)
        top = y + 28
        th = tree(s, 2, top, 336, calls, 14)
        cy = top + th + 10
        for kind, content in ctxs:
            h = ctx_box(s, 34, cy, 304, kind, content, 14)
            if kind not in ('note', 'draft'):
                s.path(f'M8 {num(top + th)}V{num(cy + 20)}H10', 'fl-m')
            cy += h + 8
        y = cy + 18
    s.h = y - 22
    return s


if __name__ == '__main__':
    print(wide().write(), narrow().write())
