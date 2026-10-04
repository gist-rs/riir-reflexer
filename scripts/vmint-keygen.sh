#!/bin/sh
# vmint-keygen.sh — vessel-mint key genesis (one-time owner act)
#
#   ./vmint-keygen.sh <worker> [secret] [--env <env>] [--selftest <reflexer-bin>]
#     Generate the Ed25519 MINT keypair, optionally self-test it by minting a
#     scratch vessel, push the seed to the CF Worker secret, print the
#     verifying key to RECORD. Seed never hits stdout.
#
#   ./vmint-keygen.sh --root <out-file>
#     Generate the OFFLINE ROOT key instead. Nothing touches CF. Seed goes to
#     <out-file> (0600) + <out-file>.pub — move to cold storage immediately.
#
# - CF secrets are write-only: a lost mint seed is recovered by ROOT
#   re-delegation (new key + revoke old), never by reading it back.
# - Runs under `env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN ...` (the
#   workspace wrangler token-alias hazard).
set -eu

hex32() { od -An -tx1 -v | tr -d ' \n'; }
wiperm() { for f in $1; do [ -f "$f" ] || continue; rm -P -- "$f" 2>/dev/null || rm -f -- "$f"; done; }

CLEAN=""
trap '[ -n "$CLEAN" ] && wiperm "$CLEAN" || :' EXIT

MODE="mint"; CF_ENV=""; SELFTEST=""; POS1=""; POS2=""
while [ $# -gt 0 ]; do
    case "$1" in
        --root)     MODE="root" ;;
        --env)      CF_ENV="${2:?}"; shift ;;
        --selftest) SELFTEST="${2:?}"; shift ;;
        -h|--help)  sed -n '2,12p' "$0"; exit 0 ;;
        -*)         echo "unknown flag: $1" >&2; exit 2 ;;
        *)          if   [ -z "$POS1" ]; then POS1="$1"
                    elif [ -z "$POS2" ]; then POS2="$1"
                    else echo "extra arg: $1" >&2; exit 2; fi ;;
    esac
    shift
done

# ── keypair: seed hex + verifying hex, consistent by construction ──────────
# NOTE: hex32 emits NO trailing newline (tr strips it) — capture each value
# and print with explicit '\n's, else the two lines concatenate (the
# "bad key lengths" bug: 128-char line 1, empty line 2).
gen_pair() {
    # Generator ladder: PATH openssl (must support ed25519 — LibreSSL 3.3 on
    # macOS does NOT), then Homebrew openssl@3 keg paths (there is no binary
    # NAMED openssl@3 — command -v never matched), then uv + PyNaCl.
    for OSSL in openssl \
               /opt/homebrew/opt/openssl@3/bin/openssl \
               /usr/local/opt/openssl@3/bin/openssl; do
        command -v "$OSSL" >/dev/null 2>&1 || continue
        "$OSSL" genpkey -algorithm ed25519 -out /dev/null >/dev/null 2>&1 || continue
        T="$(mktemp)"; CLEAN="$CLEAN $T"
        "$OSSL" genpkey -algorithm ed25519 -outform DER -out "$T" 2>/dev/null || continue
        seed_hex="$(tail -c 32 "$T" | hex32)"
        pub_hex="$("$OSSL" pkey -in "$T" -inform DER -pubout -outform DER 2>/dev/null | tail -c 32 | hex32)"
        [ "${#seed_hex}" -eq 64 ] && [ "${#pub_hex}" -eq 64 ] || {
            echo "keygen: $OSSL produced malformed output (seed=${#seed_hex} pub=${#pub_hex}), trying next" >&2
            continue
        }
        printf '%s\n%s\n' "$seed_hex" "$pub_hex"
        return 0
    done
    if command -v uv >/dev/null 2>&1; then
        uv run -q --with pynacl python -c 'from nacl.signing import SigningKey
k = SigningKey.generate()
print(k.encode().hex()); print(k.verify_key.encode().hex())'
        return 0
    fi
    echo "no ed25519 keygen: need openssl w/ ed25519 (brew openssl@3) or uv" >&2
    exit 1
}

KEYS="$(gen_pair)"
seed="$(printf '%s\n' "$KEYS" | sed -n 1p)"
pub="$(printf '%s\n'  "$KEYS" | sed -n 2p)"
if [ "${#seed}" -ne 64 ] || [ "${#pub}" -ne 64 ]; then
    echo "bad key lengths (seed=${#seed} pub=${#pub})" >&2
    exit 1
fi
case "$seed" in *[!0-9a-f]*|'') echo "seed not hex" >&2; exit 1 ;; esac
case "$pub"  in *[!0-9a-f]*|'') echo "pub not hex"  >&2; exit 1 ;; esac

# ── root mode: cold key, never CF ──────────────────────────────────────────
if [ "$MODE" = "root" ]; then
    [ -n "$POS1" ] || { echo "usage: $0 --root <out-file>" >&2; exit 2; }
    umask 077
    DIR="$(dirname "$POS1")"
    [ -d "$DIR" ] || mkdir -p "$DIR" \
        || { echo "cannot create parent dir: $DIR" >&2; exit 1; }
    printf '%s\n' "$seed" > "$POS1"
    printf '%s\n' "$pub"   > "$POS1.pub"
    echo "root key:  $POS1 (0600) + $POS1.pub"
    echo "verifying: $pub"
    echo "MOVE IT OFFLINE NOW — the root signs delegations/revocations and never touches CF or any repo."
    exit 0
fi

# ── mint mode ───────────────────────────────────────────────────────────────
[ -n "$POS1" ] || { echo "usage: $0 <worker> [secret] [--env <env>] [--selftest <bin>]" >&2; exit 2; }
WORKER="$POS1"; SECRET="${POS2:-VESSEL_MINT_SEED}"

if [ -n "$SELFTEST" ]; then
    KF="$(mktemp)"; PIN="$(mktemp)"; V="$(mktemp)"
    CLEAN="$CLEAN $KF $PIN $V"
    printf '%s\n' "$seed" > "$KF"; chmod 600 "$KF"
    printf 'vmint keygen selftest' > "$PIN"
    OUT="$("$SELFTEST" sign --in "$PIN" --out "$V" --key-id 1 --key-file "$KF")"
    printf '%s' "$OUT" | grep -q "$pub" \
        || { echo "selftest: reflexer's verifying_key != derived pubkey" >&2; exit 1; }
    VP="$("$SELFTEST" --vessel-print "$V" --vessel-pubkey "$pub")"
    case "$VP" in
        *'"signature":"verified"'*|*'"signature": "verified"'*) ;;
        *) echo "selftest: vessel did not verify: $VP" >&2; exit 1 ;;
    esac
    echo "selftest: OK — minted + verified against the derived verifying key"
fi

if [ -n "$CF_ENV" ]; then set -- --env "$CF_ENV"; else set --; fi
printf '%s' "$seed" | env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN \
                      -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL \
    npx wrangler secret put "$SECRET" --worker "$WORKER" "$@"

echo
echo "✓ secret $SECRET pushed to worker '$WORKER'"
echo "RECORD this verifying key (delegation cert / issue / future DEFAULT_PINS):"
echo "  $pub"
echo "key_id at mint: 1  (rotate = new key + new key_id + root-signed revocation)"
