#!/usr/bin/env bash
# Check the `mainnet` GitHub environment's settings before a mainnet release
# job uses them, read-only. Nothing here has a default: an unset variable
# fails, so a mainnet job can never fall back to a testnet value.
#
#   REGISTRY       the registry contract (C…) mainnet publishes into
#   REGISTRY_PATH  that registry's path under the root, e.g.
#                  perch/constructorless
#   ROOT           the Stellar Registry root contract (C…) the path starts at
#   AUTHOR         the perch-author smart account (C…) bound to every name
#
# The network must be mainnet (STELLAR_NETWORK_PASSPHRASE). REGISTRY must be
# DERIVED_REGISTRY, the registry this checkout's account and factory derive on
# mainnet from crates/*/wasm/registry.path (`perch-derive-id registry`, run by
# a keyless job, since nothing that holds these settings compiles), and
# REGISTRY_PATH must be that file's `path`. On-chain, the three addresses must
# be distinct contracts deployed there, and REGISTRY_PATH must resolve from
# ROOT, one name at a time through each registry's fetch_contract_id, to
# exactly REGISTRY. On success it prints `registry=`, `registry_path=`,
# `root=`, and `author=` lines for $GITHUB_OUTPUT.
#
# Usage: scripts/mainnet-settings.sh
# Env:   REGISTRY, REGISTRY_PATH, ROOT, AUTHOR, DERIVED_REGISTRY,
#        STELLAR_RPC_URL, STELLAR_NETWORK_PASSPHRASE, PERCH_READER
#        (simulation identity)
set -euo pipefail

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
[ $# -eq 0 ] || { awk 'NR>1 && /^#/{sub(/^# ?/,""); print; next} NR>1{exit}' "$0"; exit 0; }

[ "${STELLAR_NETWORK_PASSPHRASE:-}" = "Public Global Stellar Network ; September 2015" ] \
    || die "STELLAR_NETWORK_PASSPHRASE is not mainnet's"
[ -n "${STELLAR_RPC_URL:-}" ] || die "STELLAR_RPC_URL is not set"
for v in REGISTRY REGISTRY_PATH ROOT AUTHOR; do
    [ -n "${!v:-}" ] || die "$v is empty: set the mainnet environment's variable (docs/deploy/README.md, \"Mainnet\")"
done
for v in REGISTRY ROOT AUTHOR; do
    [[ "${!v}" =~ ^C[A-Z2-7]{55}$ ]] || die "$v '${!v}' is not a contract address"
done
[[ "$REGISTRY_PATH" =~ ^[A-Za-z0-9_-]+(/[A-Za-z0-9_-]+)*$ ]] || die "REGISTRY_PATH '$REGISTRY_PATH' is not a registry path"
[ "$REGISTRY" != "$ROOT" ] && [ "$REGISTRY" != "$AUTHOR" ] && [ "$ROOT" != "$AUTHOR" ] \
    || die "REGISTRY, ROOT, and AUTHOR must be three different contracts"

# The registry the account and the factory resolve their infra from on
# mainnet: publishing anywhere else would deploy infra they never call.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "${DERIVED_REGISTRY:-}" =~ ^C[A-Z2-7]{55}$ ]] || die "DERIVED_REGISTRY '${DERIVED_REGISTRY:-}' is not a contract address"
[ "$REGISTRY" = "$DERIVED_REGISTRY" ] \
    || die "REGISTRY $REGISTRY, but crates/*/wasm/registry.path derives $DERIVED_REGISTRY on mainnet"
path_file="$repo_root/crates/perch-smart-account/wasm/registry.path"
committed="$(awk '$1 == "path" {print $2}' "$path_file")"
[ "$REGISTRY_PATH" = "$committed" ] \
    || die "REGISTRY_PATH $REGISTRY_PATH, but ${path_file#"$repo_root"/} commits path '$committed'"

net=(--rpc-url "$STELLAR_RPC_URL" --network-passphrase "$STELLAR_NETWORK_PASSPHRASE")
reader="${PERCH_READER:-perch-fetch-reader}"
stellar keys address "$reader" >/dev/null 2>&1 || stellar keys generate "$reader" >/dev/null 2>&1

for v in ROOT REGISTRY AUTHOR; do
    stellar contract info hash "${net[@]}" --id "${!v}" >/dev/null 2>&1 || die "$v ${!v} is not a contract on mainnet"
done

at="$ROOT"
IFS=/ read -r -a names <<<"$REGISTRY_PATH"
for name in "${names[@]}"; do
    next="$(stellar contract invoke "${net[@]}" --source-account "$reader" --send=no --id "$at" -- \
        fetch_contract_id --contract_name "$name" 2>/dev/null | tr -d '"')" \
        || die "'$name' does not resolve under $at (REGISTRY_PATH $REGISTRY_PATH)"
    at="$next"
done
[ "$at" = "$REGISTRY" ] || die "REGISTRY_PATH $REGISTRY_PATH resolves to $at, not REGISTRY $REGISTRY"

printf 'registry=%s\nregistry_path=%s\nroot=%s\nauthor=%s\n' "$REGISTRY" "$REGISTRY_PATH" "$ROOT" "$AUTHOR"
