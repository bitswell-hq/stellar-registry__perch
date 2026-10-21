#!/usr/bin/env bash
# Rehearse scripts/bootstrap-registry.sh end to end, with --send, against a
# soroban-sim fork of testnet or mainnet: `soroban-sim serve --fork <network>`
# is a Stellar RPC facade over a forked ledger that validates and applies
# every transaction locally, with real signatures, and sends nothing to the
# network. Throwaway keys stand in for every real signer:
#
#   1. Start the facade, and give the stock stellar CLI a profile of its own
#      holding only throwaway keys (operator, admin, ci).
#   2. Read who holds the bootstrap's role on the fork (perch's manager, or
#      the root's plain-key manager while perch does not exist), then
#      fork_addSigner the operator key onto that account and fork_setBalance
#      it and the admin key; each override is recorded in the fork's report.
#   3. Run the bootstrap as that account, with STELLAR_SIGN_WITH_KEY=operator:
#      create-registry for perch and nido (it skips one that exists), then
#      `all --send`, with the admin key's secret in PERCH_ADMIN_KEY for both
#      authors' apply_doc (this checkout's perch-deploy).
#   4. Check, independently of the bootstrap, every derived address on the
#      fork: perch, nido and constructorless at their registry.path ids, the
#      four seeded tier-0 at their content addresses running their hashes,
#      both authors running the built account, resolving their infra there
#      and holding ci-publish, and every name bound. Then a dry rerun of
#      `all` must find nothing left to do.
#   5. Check the fork's report: passed, no failures, no replayable signature
#      by a signer the real network knows, and exactly this run's overrides.
#
# Usage: scripts/rehearse-bootstrap.sh [--network testnet|mainnet] [--out DIR]
#   --network  one network (default: testnet, then mainnet)
#   --out      where each network's report and logs land
#              (default target/rehearse-bootstrap/<network>)
# Env: SOROBAN_SIM, a soroban-sim binary built from SOROBAN_SIM_REV below;
#      otherwise the script installs that commit under target/soroban-sim/.
#
# It reads the live network (the fork's base, and public release assets),
# so a run reflects the chain as it is now.
set -euo pipefail

SOROBAN_SIM_REPO=https://github.com/theahaco/soroban-sim
SOROBAN_SIM_REV=3ca6e5089e89d3d290568fd73bd27c644eee1657   # main: the RPC facade (theahaco/soroban-sim#26)

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DERIVE="$REPO/scripts/derive-id.py"
BOOTSTRAP="$REPO/scripts/bootstrap-registry.sh"
MANIFEST="$REPO/deployments/testnet.json"

log() { printf '\033[1;35m[rehearse]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
sha256_file() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
sha256_text() { printf '%s' "$1" | if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1; }

networks=(testnet mainnet) out_base="$REPO/target/rehearse-bootstrap"
while [ $# -gt 0 ]; do
    case "$1" in
        --network) networks=("$2"); shift 2 ;;
        --out) out_base="$2"; shift 2 ;;
        -h|--help) awk 'NR>1 && /^#/{sub(/^# ?/,""); print; next} NR>1{exit}' "$0"; exit 0 ;;
        *) die "unknown option: $1" ;;
    esac
done
for n in "${networks[@]}"; do case "$n" in testnet|mainnet) ;; *) die "--network testnet or --network mainnet" ;; esac; done
for tool in stellar jq python3 curl cargo; do command -v "$tool" >/dev/null || die "$tool not found on PATH"; done

# --- soroban-sim, pinned -----------------------------------------------------

SIM="${SOROBAN_SIM:-$REPO/target/soroban-sim/$SOROBAN_SIM_REV/bin/soroban-sim}"
if [ ! -x "$SIM" ]; then
    [ -z "${SOROBAN_SIM:-}" ] || die "SOROBAN_SIM=$SOROBAN_SIM is not executable"
    log "installing soroban-sim $SOROBAN_SIM_REV"
    cargo install --locked --git "$SOROBAN_SIM_REPO" --rev "$SOROBAN_SIM_REV" soroban-sim-cli \
        --root "$REPO/target/soroban-sim/$SOROBAN_SIM_REV"
fi

# --- one network ---------------------------------------------------------------

SERVER_PID=""
stop_server() { if [ -n "$SERVER_PID" ]; then kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true; SERVER_PID=""; fi; }
trap stop_server EXIT

rehearse() {
    local network="$1" out passphrase url line role_source root perch constructorless nido role
    out="$out_base/$network"
    rm -rf "$out"
    mkdir -p "$out/stellar-config" "$out/stellar-data"
    case "$network" in
        testnet) passphrase="Test SDF Network ; September 2015" ;;
        mainnet) passphrase="Public Global Stellar Network ; September 2015" ;;
    esac

    # A stellar-cli profile of throwaway keys only: no real key is within
    # reach, and nothing is read from the caller's profile.
    export STELLAR_CONFIG_HOME="$out/stellar-config" STELLAR_DATA_HOME="$out/stellar-data"
    export XDG_CONFIG_HOME="$out/stellar-config" XDG_DATA_HOME="$out/stellar-data"
    unset STELLAR_ACCOUNT STELLAR_NETWORK STELLAR_SECRET_KEY STELLAR_SIGN_WITH_KEY PERCH_ADMIN_KEY PERCH_CI_KEY
    local k
    for k in operator admin ci; do stellar keys generate "$k" >/dev/null 2>&1 </dev/null; done
    local operator_g admin_g ci_g
    operator_g="$(stellar keys address operator)" admin_g="$(stellar keys address admin)" ci_g="$(stellar keys address ci)"

    log "$network: forking with soroban-sim serve"
    "$SIM" serve --fork "$network" --network-passphrase "$passphrase" --port 0 --report "$out/report" \
        >"$out/serve.log" 2>&1 &
    SERVER_PID=$!
    url=""
    for _ in $(seq 1 180); do
        line="$(grep -o 'facade on http://127\.0\.0\.1:[0-9]*' "$out/serve.log" || true)"
        [ -z "$line" ] || { url="${line#facade on }"; break; }
        kill -0 "$SERVER_PID" 2>/dev/null || { cat "$out/serve.log" >&2; die "soroban-sim serve exited"; }
        sleep 1
    done
    [ -n "$url" ] || { cat "$out/serve.log" >&2; die "soroban-sim serve did not start"; }
    rpc() { # method params-json -> result
        local r
        r="$(curl -fsS "$url" -H 'content-type: application/json' \
            -d "$(jq -cn --arg m "$1" --argjson p "$2" '{jsonrpc: "2.0", id: 1, method: $m, params: $p}')")"
        jq -e 'has("error") | not' >/dev/null <<<"$r" || die "$1: $(jq -c .error <<<"$r")"
        jq -c .result <<<"$r"
    }
    # Everything below sends only to this local fork.
    [[ "$(rpc fork_info '{}' | jq -r .facade)" == soroban-sim* ]] || die "$url is not a soroban-sim facade"
    [ "$(rpc getNetwork '{}' | jq -r .passphrase)" = "$passphrase" ] || die "the fork is not $network"
    log "$network: facade at $url, $(rpc fork_info '{}' | jq -r '"a fork of ledger \(.fork.ledger)"')"
    export STELLAR_RPC_URL="$url" STELLAR_NETWORK_PASSPHRASE="$passphrase"
    local net=(--rpc-url "$url" --network-passphrase "$passphrase")

    read -r root perch constructorless <<<"$("$DERIVE" chain "$passphrase" | tr '\n' ' ')"
    nido="$("$DERIVE" name "$root" nido "$passphrase")"
    role_source="$(awk '$1 == "root-deployer" {print $2}' "$REPO/crates/perch-smart-account/wasm/registry.path")"
    read_call() { # contract fn args... (simulated, from an account the base has)
        local id="$1"
        shift
        stellar contract invoke "${net[@]}" --id "$id" --source-account "$role_source" --send=no -- "$@" </dev/null 2>&1 |
            grep -v '^[[:space:]]*$' | tail -n 1 | tr -d '"'
    }

    # The account the bootstrap signs as: perch's manager, or, before perch
    # exists, the root's manager, which must then be a plain key.
    if [[ "$(read_call "$root" fetch_contract_id --contract-name perch)" == C* ]]; then
        role="$(read_call "$perch" manager)"
    else
        role="$(read_call "$root" manager)"
    fi
    [[ "$role" =~ ^G[A-Z2-7]{55}$ ]] || die "the bootstrap's role on $network is held by '$role', not a plain key"
    log "$network: the bootstrap signs as $role, with the throwaway operator key $operator_g"

    local labels=()
    rpc fork_addSigner "$(jq -cn --arg a "$role" --arg s "$operator_g" --arg l "operator key signs for $role" \
        '{account: $a, signer: $s, label: $l}')" >/dev/null
    labels+=("fork_addSigner: operator key signs for $role")
    rpc fork_setBalance "$(jq -cn --arg a "$role" --arg l "fund $role for the uploads" \
        '{account: $a, xlm: 1000, label: $l}')" >/dev/null
    labels+=("fork_setBalance: fund $role for the uploads")
    rpc fork_setBalance "$(jq -cn --arg a "$admin_g" --arg l "fund the admin key for apply_doc" \
        '{account: $a, xlm: 100, label: $l}')" >/dev/null
    labels+=("fork_setBalance: fund the admin key for apply_doc")

    local expiry flags
    expiry=$(( $(rpc getLatestLedger '{}' | jq -r .sequence) + 518400 ))
    flags=(--network "$network" --source "$role" --admin-key "$admin_g" --ci-key "$ci_g" --ci-expiry-ledger "$expiry")
    [ "$network" != mainnet ] || flags+=(--i-mean-mainnet)
    (
        export STELLAR_SIGN_WITH_KEY=operator
        PERCH_ADMIN_KEY="$(stellar keys secret admin)"
        export PERCH_ADMIN_KEY
        "$BOOTSTRAP" "${flags[@]}" --send create-registry --name perch
        "$BOOTSTRAP" "${flags[@]}" --send create-registry --name nido
        "$BOOTSTRAP" "${flags[@]}" --send all
    ) 2>&1 | tee "$out/bootstrap.log" >&2

    # --- every derived address, read back from the fork ---------------------
    local failures account_hash
    check() { # what got want
        if [ "$2" = "$3" ]; then
            printf '  ok    %-52s %s\n' "$1" "$3"
        else
            printf '  FAIL  %-52s %s (expected %s)\n' "$1" "${2:-nothing}" "$3"
        fi
    }
    executable() {
        stellar ledger entry fetch contract-data "${net[@]}" --contract "$1" --instance --output json </dev/null 2>/dev/null |
            jq -r '.entries[0].val.contract_data.val.contract_instance.executable.wasm // empty'
    }
    has_ci_publish() { # author registry
        local count id rule
        count="$(read_call "$1" get_context_rules_count)"
        [[ "$count" =~ ^[0-9]+$ ]] || { echo no; return; }
        for ((id = 0; id < count + 32; id++)); do
            rule="$(stellar contract invoke "${net[@]}" --id "$1" --source-account "$role_source" --send=no -- \
                get_context_rule --context_rule_id "$id" </dev/null 2>/dev/null)" || continue
            if [ "$(jq -r .name <<<"$rule")" = ci-publish ] &&
                [ "$(jq -r '.context_type.CallContract // empty' <<<"$rule")" = "$2" ]; then
                echo yes
                return
            fi
        done
        echo no
    }
    account_hash="$(sha256_file "$REPO/target/bootstrap-registry/account/perch_account.wasm")"
    {
        echo "$network: derived addresses on the fork"
        check "root.fetch_contract_id(perch)" "$(read_call "$root" fetch_contract_id --contract-name perch)" "$perch"
        check "root.fetch_contract_id(nido)" "$(read_call "$root" fetch_contract_id --contract-name nido)" "$nido"
        check "perch.fetch_contract_id(constructorless)" "$(read_call "$perch" fetch_contract_id --contract-name constructorless)" "$constructorless"
        check "constructorless runs perch-stateless-registry" "$(executable "$constructorless")" "$(jq -r .registry.wasm_hash "$MANIFEST")"
        local n v h at project author reg pins
        for n in perch-doc-compiler perch-interpreter perch-spending-limit perch-ed25519-verifier; do
            v="$(jq -r --arg n "$n" '.contracts[$n].version' "$MANIFEST")"
            h="$(jq -r --arg n "$n" '.contracts[$n].sha256' "$MANIFEST")"
            at="$("$DERIVE" content "$constructorless" "$h" "$passphrase")"
            check "constructorless.fetch_hash($n, $v)" "$(read_call "$constructorless" fetch_hash --wasm_name "$n" --version "\"$v\"")" "$h"
            check "$n at $at" "$(executable "$at")" "$h"
        done
        for project in perch nido; do
            author="$("$DERIVE" name "$role" "$project-author" "$passphrase")"
            if [ "$project" = perch ]; then reg="$constructorless"; else reg="$nido"; fi
            check "$project author $author runs" "$(executable "$author")" "$account_hash"
            pins="$(stellar contract invoke "${net[@]}" --id "$author" --source-account "$role_source" --send=no -- infra </dev/null 2>/dev/null |
                jq -c '{doc_compiler, interpreter, spending_limit}' 2>/dev/null || true)"
            check "$project author infra()" "$pins" "$(jq -cn \
                --arg c "$("$DERIVE" content "$constructorless" "$(jq -r '.contracts["perch-doc-compiler"].sha256' "$MANIFEST")" "$passphrase")" \
                --arg i "$("$DERIVE" content "$constructorless" "$(jq -r '.contracts["perch-interpreter"].sha256' "$MANIFEST")" "$passphrase")" \
                --arg s "$("$DERIVE" content "$constructorless" "$(jq -r '.contracts["perch-spending-limit"].sha256' "$MANIFEST")" "$passphrase")" \
                '{doc_compiler: $c, interpreter: $i, spending_limit: $s}')"
            check "$project author ci-publish scoped to $reg" "$(has_ci_publish "$author" "$reg")" yes
        done
        for n in $(jq -r '.contracts | to_entries[] | select(.value.address) | .key' "$MANIFEST"); do
            case "$n" in perch-doc-compiler|perch-interpreter|perch-spending-limit|perch-ed25519-verifier) continue ;; esac
            check "constructorless binds $n" "$(read_call "$constructorless" fetch_hash --wasm_name "$n" --version '"0.0.0"')" "$(sha256_text "perch-bootstrap:$n")"
        done
        for n in nido-factory nido-name-registry; do
            check "nido binds $n" "$(read_call "$nido" fetch_hash --wasm_name "$n" --version '"0.0.0"')" "$(sha256_text "perch-bootstrap:$n")"
        done
    } | tee "$out/addresses.txt" >&2
    failures="$(grep -c '^  FAIL' "$out/addresses.txt" || true)"
    [ "$failures" = 0 ] || die "$network: $failures derived-address checks failed ($out/addresses.txt)"

    # A rerun finds everything done: no step left to plan.
    "$BOOTSTRAP" "${flags[@]}" all >"$out/rerun.log" 2>&1 || { cat "$out/rerun.log" >&2; die "$network: the dry rerun failed"; }
    grep -q 'all done' "$out/rerun.log" || { cat "$out/rerun.log" >&2; die "$network: the dry rerun did not finish"; }
    ! grep -qF '  ->' "$out/rerun.log" || { grep -F '  ->' "$out/rerun.log" >&2; die "$network: the dry rerun still plans steps"; }

    # --- the fork's report ----------------------------------------------------
    local report
    report="$(rpc fork_report '{}')"
    jq '.report' <<<"$report" >"$out/report.json"
    jq -r '.markdown' <<<"$report" >"$out/report.md"
    jq -e '.passed' >/dev/null <<<"$report" || die "$network: the fork's report failed: $(jq -c .failures <<<"$report")"
    jq -e '.failures == []' >/dev/null <<<"$report" || die "$network: report failures: $(jq -c .failures <<<"$report")"
    jq -e '.report.safety.replayable == [] and ([.report.steps[].tx.replayable // [] | length] | add // 0) == 0' >/dev/null <<<"$report" ||
        die "$network: an envelope carries a signature the real network would accept: $(jq -c .report.safety <<<"$report")"
    local listed
    listed="$(jq -r '[.report.overrides[].label] | sort | join("\n")' <<<"$report")"
    [ "$listed" = "$(printf '%s\n' "${labels[@]}" | LC_ALL=C sort)" ] ||
        die "$network: the report lists overrides [$(tr '\n' ',' <<<"$listed")], this run made [$(printf '%s,' "${labels[@]}")]"
    log "$network: passed. $(jq -r '.report.fees | "\(.transactions) transactions (\(.applied) applied, \(.failed) failed, \(.rejected) rejected), \(.total_charged / 10000000) XLM charged"' <<<"$report"); overrides: ${#labels[@]}, all listed; no replayable signatures"
    stop_server
}

for n in "${networks[@]}"; do rehearse "$n"; done
log "rehearsed ${networks[*]}: reports under $out_base"
