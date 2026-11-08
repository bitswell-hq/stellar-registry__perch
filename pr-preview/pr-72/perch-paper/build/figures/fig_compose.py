"""fig-compose: one decision point against a composition of policies (explorable E4; static form).

Four requests through two arrangements of the same five checks, computed with
perchmodel.py: (a) one decision point holding the checks and their state, shaped like a
Perch account; (b) one policy per check, each with its own state, run in order until one
refuses, shaped like a policy-composed OpenZeppelin account. A model: no verdict here
names a contract or an error code. <g data-request> groups carry the verdicts for the
model test.
"""
import html
from figlib import SVG, FIGDIR, num, width, wrap
import perchmodel as M

FID = 'fig-compose'
NAMES = {'pin': 'recipient pin', 'percall': 'per-call bound', 'window': 'window cap', 'threshold': 'two signers',
         'live': 'live read', 'decision': 'decision point'}
TITLE_A = '(a) Shaped like a Perch account'
TITLE_B = '(b) Shaped like a policy-composed OpenZeppelin account'
SUB_A = 'one decision point holds the checks and the window record'
SUB_B = 'one policy per check, each with its own state, run in order'


SETUP = ('Each request is signed by the app and a co-signer, and the oracle\u2019s price is above the floor, '
         'unless it says otherwise.')


def arith(c):
    return f' ({c["remaining"]} + {c["amount"]} = {c["total"]} > {c["limit"]})' if c else ''


def a_text(a):
    if a['ok']:
        return f'accepted; written: the window record, now {a["window"]} of 10'
    return f'refused at the decision point: {NAMES[a["refused"]]}{arith(a.get("charge"))}; nothing written'


def b_items(b):
    items = []
    for k, name in enumerate(b['ran']):
        if k:
            items.append(('arrow',))
        refused = (not b['ok']) and k == len(b['ran']) - 1
        items.append(('chip', NAMES[name], 'deny' if refused else 'tested'))
    return items


def b_text(b):
    if b['ok']:
        if b['written']:
            return f'accepted; written: the window cap’s record, now {b["moved"]} of 10'
        over = ', over 10' if b['moved'] > 10 else ''
        return f'accepted; no window cap attached, so nothing written; moved in the last 100 ledgers: {b["moved"]}{over}'
    if b['rolled_back']:
        return f'refused by the {NAMES[b["refused"]]} after the window cap wrote: rolled back with the call'
    return f'refused by the {NAMES[b["refused"]]}{arith(b.get("charge"))}; nothing written'


def chip_w(it, size):
    if it[0] == 'arrow':
        return 12
    return width(it[1], size) + 12


def flow(s, x, y, w, items, size, draw=True):
    cx, cy = x, y
    for n, it in enumerate(items):
        iw = chip_w(it, size)
        need = iw + (chip_w(items[n + 1], size) + 3 if it[0] == 'arrow' and n + 1 < len(items) else 0)
        if cx > x and cx + need > x + w:
            cx, cy = x, cy + 30
        if draw:
            if it[0] == 'arrow':
                s.path(f'M{num(cx)} {num(cy + 12)}H{num(cx + 11)}', 'fl-m', marker_end='url(#perch-ah-m)')
            else:
                s.box(cx, cy, iw, 24, it[2], 4)
                s.text(cx + 6, cy + 17, it[1], 'd' if it[2] == 'deny' else 'x', size)
                if it[2] == 'deny':
                    s.path(f'M{num(cx + iw)} {num(cy - 3)}V{num(cy + 27)}', 'fl-d')
        cx += iw + 3
    return cy - y + 24


def cell(s, x, y, w, items, text, ok, size, draw=True):
    h = 8
    if items:
        h += flow(s, x + 8, y + h, w - 16, items, size, draw) + 8
    rows = wrap(text, w - 40, size)
    if draw:
        if ok:
            s.check(x + 9, y + h + 3, 'gl')
        else:
            s.cross(x + 15, y + h + 9, 5)
        s.lines(x + 30, y + h + 14, rows, 'x' if ok else 'd', size, lh=18)
    return h + len(rows) * 18 + 8


def draw(narrow):
    script = M.e4_script()
    W = 340 if narrow else 760
    size = 14
    s = SVG(FID, W, narrow=narrow)
    s.text(2, 16, 'MODEL', 'm tag', 14)
    s.text(60, 16, 'the same five checks, two arrangements; no contract named', 'm', 14) if not narrow else None
    y = s.lines(2, 38, wrap(SETUP, W - 4, 14), 'x', 14, lh=17) + 4
    if narrow:
        cols = {'a': (2, 336), 'b': (2, 336)}
    else:
        LW = 150
        cols = {'a': (LW + 8, 250), 'b': (LW + 266, W - LW - 268)}
    # the arrangements
    order0 = script[0][2]
    if not narrow:
        for key, title, sub in (('a', TITLE_A, SUB_A), ('b', TITLE_B, SUB_B)):
            x, w = cols[key]
            rows = wrap(title, w, 14, True) + wrap(sub, w, 14)
            for k, ln in enumerate(rows):
                s.text(x, y + 14 + k * 17, ln, 't' if k < len(wrap(title, w, 14, True)) else 'm', 14)
        y += 14 + max(len(wrap(TITLE_A, cols['a'][1], 14, True) + wrap(SUB_A, cols['a'][1], 14)),
                      len(wrap(TITLE_B, cols['b'][1], 14, True) + wrap(SUB_B, cols['b'][1], 14))) * 17 + 4
        y = structure(s, cols, y, order0, size, narrow)
        s.text(2, y + 24, 'REQUESTS', 't tag', 14)
        y += 34
        for n, (label, req, order, a, b) in enumerate(script, 1):
            lab = wrap(label, LW - 30, 14, True)
            ha = cell(s, cols['a'][0], 0, cols['a'][1], [('chip', NAMES['decision'], 'tested' if a['ok'] else 'deny')], a_text(a), a['ok'], size, False)
            hb = cell(s, cols['b'][0], 0, cols['b'][1], b_items(b), b_text(b), b['ok'], size, False)
            h = max(ha, hb, len(lab) * 17 + 14)
            s.open(data_request=str(n), data_a='accepted' if a['ok'] else f'refused:{a["refused"]}',
                   data_b='accepted' if b['ok'] else f'refused:{b["refused"]}' + (':rolled-back' if b['rolled_back'] else ''))
            s.numdot(13, y + 17, n, r=11, size=14)
            s.lines(30, y + 22, lab, 't', 14, lh=17)
            for key, items, text, ok in (('a', [('chip', NAMES['decision'], 'tested' if a['ok'] else 'deny')], a_text(a), a['ok']),
                                         ('b', b_items(b), b_text(b), b['ok'])):
                x, w = cols[key]
                s.box(x, y, w, h, 'tested', 6)
                cell(s, x, y, w, items, text, ok, size)
            s.close()
            y += h + 6
    else:
        for key, title, sub in (('a', TITLE_A, SUB_A), ('b', TITLE_B, SUB_B)):
            y = s.lines(2, y + 14, wrap(title, 336, 14, True), 't', 14, lh=17)
            y = s.lines(2, y + 17, wrap(sub, 336, 14), 'm', 14, lh=17) + 8
            y = structure(s, {key: (2, 336)}, y, order0, size, narrow, only=key) + 10
        s.text(2, y + 14, 'REQUESTS', 't tag', 14)
        y += 26
        for n, (label, req, order, a, b) in enumerate(script, 1):
            s.open(data_request=str(n), data_a='accepted' if a['ok'] else f'refused:{a["refused"]}',
                   data_b='accepted' if b['ok'] else f'refused:{b["refused"]}' + (':rolled-back' if b['rolled_back'] else ''))
            s.numdot(13, y + 12, n, r=11, size=14)
            y = s.lines(32, y + 17, wrap(label, 300, 14, True), 't', 14, lh=17) + 10
            for key, tag, items, text, ok in (('a', '(A)', [('chip', NAMES['decision'], 'tested' if a['ok'] else 'deny')], a_text(a), a['ok']),
                                              ('b', '(B)', b_items(b), b_text(b), b['ok'])):
                h = cell(s, 2, 0, 336, items, text, ok, size, False) + 18
                s.box(2, y, 336, h, 'tested', 6)
                s.text(10, y + 17, tag, 'm tag', 14)
                cell(s, 2, y + 18, 336, items, text, ok, size)
                y += h + 6
            s.close()
            y += 8
    # the guarantees
    y += 18
    g = [('a', 'Window at most 10: holds while the rule keeps its cap.'),
         ('b', 'Window at most 10: holds while the window-cap policy is attached.')]
    for key, text in g:
        x, w = cols[key]
        rows = wrap(('(a) ' if narrow and key == 'a' else '(b) ' if narrow else '') + text, w, 14)
        if narrow:
            y = s.lines(2, y, rows, 'x', 14, lh=17) + 20
    if not narrow:
        ya = s.lines(cols['a'][0], y, wrap(g[0][1], cols['a'][1], 14), 'x', 14, lh=17)
        yb = s.lines(cols['b'][0], y, wrap(g[1][1], cols['b'][1], 14), 'x', 14, lh=17)
        s.text(2, y, 'GUARANTEE', 't tag', 14)
        y = max(ya, yb) + 20
    s.h = y - 8
    return s


def structure(s, cols, y, order, size, narrow, only=None):
    """The two arrangements: one box holding four checks and an empty live-read slot; a chain of five policies."""
    h_out = 0
    if only in (None, 'a'):
        x, w = cols['a']
        checks = ['recipient pin', 'two signers', 'window cap + its record']
        rows = []
        for c in checks:
            rows += wrap('• ' + c, w - 24, 14)
        slot = wrap('per-call bound and live read: a Perch rule has neither', w - 40, 14)
        h = 30 + len(rows) * 17 + 10 + len(slot) * 17 + 14
        s.box(x, y, w, h, 'key', 6)
        s.text(x + 12, y + 22, NAMES['decision'], 't', 14)
        yy = s.lines(x + 12, y + 42, rows, 'x', 14, lh=17) + 8
        s.box(x + 10, yy, w - 20, len(slot) * 17 + 10, 'none', 4)
        s.lines(x + 20, yy + 18, slot, 'm', 14, lh=17)
        h_out = max(h_out, h)
    if only in (None, 'b'):
        x, w = cols['b']
        items = []
        for k, name in enumerate(order):
            if k:
                items.append(('arrow',))
            items.append(('chip', NAMES[name] + (' + record' if name == 'window' else ''), 'tested'))
        h = flow(s, x, y + 4, w, items, 14) + 8
        note = wrap('Each policy can be attached, detached or moved; the first to refuse stops the call.', w, 14)
        s.lines(x, y + h + 14, note, 'm', 14, lh=17)
        h_out = max(h_out, h + 14 + len(note) * 17)
    return y + h_out + 6


def text_alt():
    out = [f'<p>A model of the same five checks in two arrangements. {html.escape(SETUP)} {html.escape(TITLE_A)}: {html.escape(SUB_A)}; '
           'it has no slot for a live read or a per-call bound: a Perch rule reads nothing outside the account and '
           'cannot bound one call\u2019s amount. '
           f'{html.escape(TITLE_B)}: {html.escape(SUB_B)}, starting with the window cap.</p><ol>']
    for label, req, order, a, b in M.e4_script():
        out.append(f'<li>{html.escape(label)}. (a) {html.escape(a_text(a))}. (b) policies run: '
                   f'{html.escape(", ".join(NAMES[n] for n in b["ran"]))}; {html.escape(b_text(b))}.</li>')
    out.append(f'</ol><p>Window at most 10: in (a) while the rule keeps its cap; in (b) while the window-cap policy is '
               f'attached.</p>')
    return ''.join(out)


if __name__ == '__main__':
    print(draw(False).write(), draw(True).write())
    (FIGDIR / f'{FID}.text.html').write_text(text_alt() + '\n', encoding='utf-8')
