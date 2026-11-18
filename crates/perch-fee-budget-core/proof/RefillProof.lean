import RefillCore
open Aeneas Aeneas.Std Result WP
open refill_core

-- This theorem applies to the extracted function called by the contract,
-- for every recipient type and every representable input, not sampled values.
theorem plan_safe {R : Type} (f : R) (b : Budget) (balance : U64) (ledger : U32) :
    plan f b balance ledger ⦃ out => ∀ p, out = some p →
      p.recipient = f ∧
      0 < p.amount.val ∧
      p.total.val = b.spent.val + p.amount.val ∧
      p.total.val ≤ b.cap.val ∧
      balance.val < b.low.val ∧
      balance.val + p.amount.val = b.target.val ∧
      ledger.val ≤ b.expires.val ⦄ := by
  unfold plan
  split
  · simp
  · split
    · simp
    · split
      · simp
      · split
        · simp
        · step
          split
          · simp
          · step
            split
            · simp
            · step
              simp only [Option.some.injEq, forall_eq']
              scalar_tac

#print axioms plan_safe

theorem accepted_refill {R : Type} {f : R} {b : Budget} {balance : U64} {ledger : U32}
    {p : Refill R} (execution : plan f b balance ledger = ok (some p)) :
    p.recipient = f ∧ 0 < p.amount.val ∧
    p.total.val = b.spent.val + p.amount.val ∧ p.total.val ≤ b.cap.val := by
  have h := plan_safe f b balance ledger
  simp only [execution, WP.spec_ok] at h
  have good := h p rfl
  exact ⟨good.1, good.2.1, good.2.2.1, good.2.2.2.1⟩

-- Consecutive committed plans reuse the previous total; balance may change
-- arbitrarily between refills. Failed calls do not advance this trace.
inductive Run {R : Type} (f : R) (initial : Budget) : U64 → List (Refill R) → Prop
  | start : Run f initial initial.spent []
  | refill {spent history} (before : Run f initial spent history)
      (balance : U64) (ledger : U32) (p : Refill R)
      (executed : plan f { initial with spent := spent } balance ledger = ok (some p)) :
      Run f initial p.total (history ++ [p])

theorem lifetime_budget {R : Type} {f : R} {initial : Budget} {spent : U64}
    {history : List (Refill R)} (valid : initial.spent.val ≤ initial.cap.val)
    (run : Run f initial spent history) :
    spent.val ≤ initial.cap.val ∧
    spent.val = initial.spent.val + (history.map (fun p => p.amount.val)).sum ∧
    ∀ p ∈ history, p.recipient = f := by
  induction run with
  | start => simp [valid]
  | refill before balance ledger p executed ih =>
    have good := accepted_refill executed
    refine ⟨good.2.2.2, ?_, ?_⟩
    · simp only [List.map_append, List.map_cons, List.map_nil, List.sum_append,
        List.sum_cons, List.sum_nil, Nat.add_zero]
      rw [good.2.2.1, ih.2.1]
      omega
    · intro q member
      simp only [List.mem_append, List.mem_singleton] at member
      rcases member with old | same
      · exact ih.2.2 q old
      · subst q; exact good.1

#print axioms accepted_refill
#print axioms lifetime_budget
