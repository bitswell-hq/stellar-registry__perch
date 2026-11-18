#![no_std]

/// Lifetime accounting, in stroops. No renewal/reset entry point exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub cap: u64,
    pub spent: u64,
    pub low: u64,
    pub target: u64,
    pub expires: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refill<R> {
    pub recipient: R,
    pub amount: u64,
    pub total: u64,
}

/// This exact function is called by the contract and extracted to Lean.
/// There is no requested destination or requested amount.
pub fn plan<R>(registered: R, b: Budget, balance: u64, ledger: u32) -> Option<Refill<R>> {
    if b.spent > b.cap || b.low > b.target || balance >= b.low || ledger > b.expires {
        return None;
    }
    let amount = b.target - balance;
    if amount == 0 || amount > b.cap - b.spent {
        return None;
    }
    Some(Refill {
        recipient: registered,
        amount,
        total: b.spent + amount,
    })
}
