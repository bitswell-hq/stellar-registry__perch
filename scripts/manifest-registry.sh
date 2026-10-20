#!/usr/bin/env bash
# Whether a deployment manifest's registry is the one this source derives on
# its network. The account and the factory derive their registry from the
# committed crates/*/wasm/registry.path (a root deployer and salt, then named
# deploys) under the ledger's network id; perch-derive-id runs that same
# compiled-in derivation.
#
# Prints one line and exits 0 when the manifest is usable:
#   current <registry>            the manifest records the derived registry
#   pending <recorded> <derived>  it records another, and
#                                 deployments/pending-registry.json declares
#                                 exactly that migration for its network: the
#                                 source has moved to a layout no deployment
#                                 has created yet
# Exits 1, saying why, on an undeclared mismatch, on a declaration the
# manifest has already caught up with (delete it), and when the crates'
# registry.path files differ.
#
# Usage: scripts/manifest-registry.sh [--manifest <path>]
#   --manifest  default deployments/${STELLAR_NETWORK:-testnet}.json
# Env: PERCH_DERIVE_ID  a perch-derive-id to run (default: build this
#      checkout's)
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/deployments/${STELLAR_NETWORK:-testnet}.json"
while [ $# -gt 0 ]; do
    case "$1" in
        --manifest) manifest="$2"; shift 2 ;;
        -h|--help) awk 'NR>1 && /^#/{sub(/^# ?/,""); print; next} NR>1{exit}' "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 1 ;;
    esac
done
[ -f "$manifest" ] || { echo "error: no manifest at $manifest" >&2; exit 1; }
command -v jq >/dev/null || { echo "error: jq not found" >&2; exit 1; }

shopt -s nullglob
paths=("$repo_root"/crates/*/wasm/registry.path)
[ "${#paths[@]}" -gt 0 ] || { echo "error: no crates/*/wasm/registry.path" >&2; exit 1; }
for p in "${paths[@]}"; do
    cmp -s "$p" "${paths[0]}" || {
        echo "error: ${p#"$repo_root"/} differs from ${paths[0]#"$repo_root"/}" >&2
        exit 1
    }
done

derive_registry() { # passphrase
    if [ -n "${PERCH_DERIVE_ID:-}" ]; then
        "$PERCH_DERIVE_ID" registry "$1"
    else
        (cd "$repo_root" && cargo run -q -p perch-derive-id -- registry "$1")
    fi
}
network="$(jq -r '.network' "$manifest")"
recorded="$(jq -r '.registry.id' "$manifest")"
derived="$(derive_registry "$(jq -r '.network_passphrase' "$manifest")")"
pending="$repo_root/deployments/pending-registry.json"
declared() { [ -f "$pending" ] || return 0; jq -r --arg n "$network" --arg k "$1" '.[$n][$k] // empty' "$pending"; }

if [ "$recorded" = "$derived" ]; then
    [ -z "$(declared derived)" ] || {
        echo "error: $(basename "$manifest") records the derived registry $derived: delete the $network entry from deployments/pending-registry.json" >&2
        exit 1
    }
    echo "current $recorded"
elif [ "$(declared recorded)" = "$recorded" ] && [ "$(declared derived)" = "$derived" ]; then
    echo "pending $recorded $derived"
else
    echo "error: $(basename "$manifest") records registry $recorded, this source derives $derived on $network, and deployments/pending-registry.json declares no such migration" >&2
    exit 1
fi
