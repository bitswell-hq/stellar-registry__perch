"""fig-proof-scope: what the proofs cover (main paper; the verification paper has the full picture).
One Rust source, two paths: translated into Lean, where the theorems are checked (double border),
and compiled into the account's Wasm, which is tested (single); tests bridge them (dashed);
nothing checks one against the other. At most 300 u high."""
from figlib import SVG, fact, num, wrap, arrow_right, arrow_down

FID = 'fig-proof-scope'


def wide():
    s = SVG(FID, 760)
    s.textbox(2, 96, 170, [('Rust source', 't'), (f'rule-core `{fact("rule-core-pin-short")}`, the evaluator', 'x')], 'tested', size=14, minh=80)
    # proof path
    s.textbox(236, 12, 200, [('Charon and Aeneas', 't'), ('translate it to Lean', 'x')], 'trusted', size=14, minh=60)
    hl = s.textbox(500, 6, 258, [('Lean: the translation and a model', 't'),
                            (f'all {fact("exported-theorems")} rule-core theorems kernel-checked; the stated theorems are among them', 'x')], 'proved', size=14, minh=76)
    # shipping path
    s.textbox(236, 196, 200, [(f'rustc {fact("rustc-perch")}', 't'), ('compiles it to Wasm', 'x')], 'trusted', size=14, minh=60)
    s.textbox(500, 196, 258, [('The account’s Wasm', 't'), (f'Perch `{fact("perch-pin-short")}`: tested', 'x')], 'tested', size=14, minh=60)
    s.path(f'M{num(172)} {num(120)}H{num(200)}V{num(42)}H{num(232)}', 'fl', marker_end='url(#perch-ah)')
    s.path(f'M{num(172)} {num(150)}H{num(200)}V{num(226)}H{num(232)}', 'fl', marker_end='url(#perch-ah)')
    arrow_right(s, 436, 496, 42, 'fl', 'perch-ah')
    arrow_right(s, 436, 496, 226, 'fl', 'perch-ah')
    # tests bridge them; nothing checks one against the other
    top = 6 + hl
    s.path(f'M{num(629)} {num(top + 2)}V{num(192)}', 'fl-m dash')
    s.lines(640, top + 22, wrap('tests bridge them: golden fixtures, Lean-generated traces', 118, 14), 'm', 14, lh=17)
    s.lines(236, 120, wrap('Nothing checks the Wasm against the Lean translation.', 220, 14), 'd', 14, lh=17)
    s.lines(2, 280, wrap('Double border: proved. Single: tested. Hatched: trusted. Dashed: tests, not proofs.', 756, 14), 'm', 14, lh=17)
    s.h = 290
    return s


def narrow():
    s = SVG(FID, 340, narrow=True)
    y = 4
    y += s.textbox(2, y, 336, [('Rust source', 't'), (f'rule-core `{fact("rule-core-pin-short")}`, the evaluator', 'x')], 'tested', size=14) + 10
    s.text(2, y + 14, 'PROOF PATH', 't tag', 14)
    y += 22
    y += s.textbox(2, y, 336, [('Charon and Aeneas', 't'), ('translate it to Lean', 'x')], 'trusted', size=14)
    arrow_down(s, 40, y + 2, y + 16, 'fl', 'perch-ah')
    y += 20
    y += s.textbox(2, y, 336, [('Lean: the translation and a model', 't'), (f'all {fact("exported-theorems")} rule-core theorems kernel-checked; the stated theorems are among them', 'x')], 'proved', size=14) + 12
    s.text(2, y + 14, 'SHIPPING PATH', 't tag', 14)
    y += 22
    y += s.textbox(2, y, 336, [(f'rustc {fact("rustc-perch")}', 't'), ('compiles it to Wasm', 'x')], 'trusted', size=14)
    arrow_down(s, 40, y + 2, y + 16, 'fl', 'perch-ah')
    y += 20
    y += s.textbox(2, y, 336, [('The account’s Wasm', 't'), (f'Perch `{fact("perch-pin-short")}`: tested', 'x')], 'tested', size=14) + 8
    y = s.lines(2, y + 14, wrap('Tests bridge the paths (golden fixtures, Lean-generated traces); nothing checks the Wasm against the Lean translation. Double border: proved. Single: tested. Hatched: trusted.', 336, 14), 'm', 14, lh=17)
    s.h = y + 8
    return s


if __name__ == '__main__':
    print(wide().write(), narrow().write())
