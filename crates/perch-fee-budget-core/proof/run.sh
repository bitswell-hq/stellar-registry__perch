#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
# Cached offline tools only: no credential, Claude CLI or paid model API.
proof_image="sha256:459d72ce3dca6dcc62e4b00d49be6869619f27935ec5b9076f8d06dc77822db2"
docker image inspect "$proof_image" >/dev/null # refuse a missing cache; never auto-pull
docker run --rm --network none -v "$PWD/crates/perch-fee-budget-core/proof:/workspace/proof" -v "$PWD/crates/perch-fee-budget-core:/workspace/policy-core:ro" "$proof_image" bash -euo pipefail -c '
  cd /workspace/proof
  mkdir -p generated
  charon rustc --sysroot default --preset aeneas --dest-file generated/refill_core.llbc -- --crate-type lib --crate-name refill_core /workspace/policy-core/src/lib.rs
  aeneas -backend lean -dest generated generated/refill_core.llbc
  mkdir /tmp/refill-proof
  cp /opt/lean-template/lean-toolchain /opt/lean-template/lakefile.lean /opt/lean-template/lake-manifest.json /tmp/refill-proof/
  ln -s /opt/lean-template/.lake /tmp/refill-proof/.lake
  cp generated/RefillCore.lean RefillProof.lean /tmp/refill-proof/
  cd /tmp/refill-proof
  lake env lean -o RefillCore.olean RefillCore.lean
  lake env sh -c "LEAN_PATH=.:\$LEAN_PATH lean RefillProof.lean" | tee /workspace/proof/audit.log
  if grep -E "sorryAx|error:|declaration uses" /workspace/proof/audit.log; then exit 1; fi
'
python3 crates/perch-fee-budget-core/proof/evidence.py
