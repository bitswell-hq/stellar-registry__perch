"""Bind the standard-axiom audit to the exact reusable policy sources."""
from pathlib import Path
import hashlib, json, subprocess
from datetime import datetime, timezone
root = Path(__file__).resolve().parents[3]
proof = Path(__file__).resolve().parent
names = ["plan_safe", "accepted_refill", "lifetime_budget"]
lines = (proof / "audit.log").read_text().strip().splitlines()
expected = [f"'{n}' depends on axioms: [propext, Classical.choice, Quot.sound]" for n in names]
if lines != expected:
    raise SystemExit("Unexpected or incomplete Lean axiom audit")
files = ["crates/perch-fee-budget-core/src/lib.rs",
    "crates/perch-fee-budget-core/proof/RefillProof.lean", "crates/perch-fee-budget-core/proof/generated/RefillCore.lean"]
image = "sha256:459d72ce3dca6dcc62e4b00d49be6869619f27935ec5b9076f8d06dc77822db2"
image_id = subprocess.check_output(["docker", "image", "inspect", "--format", "{{.Id}}", image], text=True).strip()
result = {"checkedAt": datetime.now(timezone.utc).isoformat(), "image": image, "imageId": image_id,
    "aeneas": "0d21a9ec7bbbb110110cacc14464a47fd71c8705", "charonRust": "nightly-2026-09-17", "lean": "4.36.0-rc2",
    "sourceHashes": {f: hashlib.sha256((root/f).read_bytes()).hexdigest() for f in files},
    "theorems": names, "standardLeanAxioms": ["propext", "Classical.choice", "Quot.sound"], "addedAxioms": [],
    "scope": "Extracted Rust plan and committed-plan trace. Storage/token/auth integration tested, not formally extracted. Compiler, extraction, Soroban host, SAC and key custody remain trusted."}
(proof / "evidence.json").write_text(json.dumps(result, indent=2)+"\n")
