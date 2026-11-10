// The sign dialog's Perch verdict and budgets, from the core, against the
// account's own verdicts on the committed gate cases.

import { beforeAll, describe, expect, it } from 'vitest';
import { ReviewCore, type Verified } from '../src/index.js';
import { perchBudgets, perchPreview, perchVerdict } from '../src/sign-dialog.js';
import { fixtures, parseReviewEnvelope } from './fixtures.js';

let core: ReviewCore;

beforeAll(async () => {
  core = await ReviewCore.load();
});

/** The account's verdict on a fixture: `true`, `false`, and its error code. */
function account(f: { account: string }): { allowed: boolean; code: number | null } {
  if (f.account === 'Accepted') return { allowed: true, code: null };
  const m = /^Refused\((?:Some\((\d+)\)|None)\)$/.exec(f.account);
  if (!m) throw new Error(f.account);
  return { allowed: false, code: m[1] ? Number(m[1]) : null };
}

describe('perchPreview', () => {
  it("gives the account's own verdict on every gate case", () => {
    const seen = new Set<string>();
    for (const f of fixtures.review) {
      const p = parseReviewEnvelope(f.input);
      const verified = p.verified.map((x) => (x === 0 ? 'no' : x === 1 ? 'yes' : 'assumed') as Verified);
      const { verdicts } = perchPreview(core, {
        authEntries: [p.entry],
        ledgerEntries: p.entries,
        ledger: p.ledger,
        verified: verified.length ? [verified] : undefined,
      });
      expect(verdicts.length, f.name).toBe(1);
      const v = verdicts[0]!;
      const want = account(f);
      expect(v.kind).toBe('perch');
      expect(v.address.startsWith('C')).toBe(true);
      if (v.reasonCode === 'controller') {
        // The controller decides a completion; the account's verdict is its.
        expect(v.allowed).toBeNull();
      } else {
        expect(v.allowed, f.name).toBe(want.allowed);
        if (!want.allowed && v.code != null) expect(v.code, f.name).toBe(want.code);
      }
      expect(v.reason.length).toBeGreaterThan(0);
      seen.add(v.reasonCode);
    }
    expect([...seen].sort()).toEqual(expect.arrayContaining(['accepted', 'refused']));
  });

  it('names the ledger and the assumed signatures in an accepted verdict', () => {
    const f = fixtures.review.find((x) => x.account === 'Accepted')!;
    const p = parseReviewEnvelope(f.input);
    const review = core.review({ entry: p.entry, ledgerEntries: p.entries, ledger: p.ledger });
    const v = perchVerdict('C', review);
    expect(v.allowed).toBe(true);
    expect(v.reason).toContain(`at ledger ${p.ledger}`);
    if (review.status !== 'unreadable' && review.assumed.length) {
      expect(v.reason).toContain('assuming');
    }
  });

  it('marks a refusal over the cap', () => {
    const f = fixtures.review.find(
      (x) => (x.output.refusal as { detail?: string } | null)?.detail === 'Cap.LimitExceeded',
    );
    expect(f).toBeDefined();
    const p = parseReviewEnvelope(f!.input);
    const review = core.review({ entry: p.entry, ledgerEntries: p.entries, ledger: p.ledger });
    expect(perchVerdict('C', review).reasonCode).toBe('refused-over-cap');
  });

  it('skips entries of other accounts', () => {
    const f = fixtures.review[0]!;
    const p = parseReviewEnvelope(f.input);
    const other = 'CA3D5KRYM6CB7OWQ6TWYRR3Z4T7GNZLKERYNZGGA5SOAOPIFY6YQGAXE';
    const out = perchPreview(core, { authEntries: [p.entry], ledgerEntries: p.entries, ledger: p.ledger, account: other });
    expect(out.verdicts).toEqual([]);
  });
});

describe('perchBudgets', () => {
  it('counts every context charging a rule together, and gives what is left exactly', () => {
    let checked = 0;
    for (const f of fixtures.review) {
      const p = parseReviewEnvelope(f.input);
      const review = core.review({ entry: p.entry, ledgerEntries: p.entries, ledger: p.ledger });
      if (review.status === 'unreadable') continue;
      const budgets = perchBudgets('C', review, {});
      expect(budgets.length).toBe(review.budgets.length);
      for (const [b, r] of budgets.map((b, i) => [b, review.budgets[i]!] as const)) {
        checked++;
        expect(b.limit).toBe(r.limit.toString());
        expect(b.amount).toBe((r.requested ?? 0n).toString());
        expect(b.remaining).toBe(r.before.remaining.toString());
        expect(b.token.code).toBe('units');
        if (review.status === 'accepted') {
          expect(BigInt(b.remainingAfter!)).toBe(r.before.remaining - (r.requested ?? 0n));
        }
      }
    }
    expect(checked).toBeGreaterThan(0);
  });
});
