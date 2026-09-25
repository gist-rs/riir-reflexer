#!/usr/bin/env bash
# Build the reflexer engine wasm (crates/reflexer-wasm, wasm32-wasip1) into
# this Worker project as reflexer.wasm (gitignored — rebuilt, never
# committed here). With --site <dir>, also mirror the wasm + reflexer_host.mjs
# into a reflex-site checkout (assets/reflexer.wasm, assets/reflexer_host.js)
# so the arena's "wasm local" board runs the SAME bytes the Worker serves.
#
#   cloudflare/reflexer-worker/build.sh [--site ../reflex-site]
#
# Speed over size: the Worker's CPU budget is the binding constraint, so
# opt-level 3 + fat LTO + one codegen unit (env overrides — the repo's own
# release profile, which the native bin is measured under, is untouched).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SITE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --site) SITE="$2"; shift 2 ;;
    *) echo "build.sh: unknown arg $1" >&2; exit 2 ;;
  esac
done
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/reflexer_wasm_target}"
rustup target add wasm32-wasip1 >/dev/null 2>&1 || true
(
  cd "$ROOT"
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
  CARGO_PROFILE_RELEASE_LTO=fat \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  CARGO_PROFILE_RELEASE_PANIC=abort \
  CARGO_TARGET_DIR="$TARGET_DIR" \
    cargo build -p reflexer-wasm --release --target wasm32-wasip1 --lib
)
OUT="$TARGET_DIR/wasm32-wasip1/release/reflexer_wasm.wasm"
[ -s "$OUT" ] || { echo "build.sh: no wasm at $OUT" >&2; exit 1; }
cp "$OUT" "$HERE/reflexer.wasm"
SHA="$(shasum -a 256 "$HERE/reflexer.wasm" | cut -d' ' -f1)"
GIT="$(git -C "$ROOT" rev-parse --short HEAD)$(git -C "$ROOT" diff --quiet -- src crates Cargo.toml Cargo.lock || echo -dirty)"
echo "reflexer.wasm: $(wc -c <"$HERE/reflexer.wasm" | tr -d ' ') bytes · sha256 $SHA · reflexer $GIT"
if [ -n "$SITE" ]; then
  [ -d "$SITE/assets" ] || { echo "build.sh: --site $SITE has no assets/" >&2; exit 1; }
  cp "$HERE/reflexer.wasm" "$SITE/assets/reflexer.wasm"
  cp "$HERE/reflexer_host.mjs" "$SITE/assets/reflexer_host.js"
  printf '{"source":"gist-rs/riir-reflexer crates/reflexer-wasm + cloudflare/reflexer-worker/reflexer_host.mjs","git":"%s","wasm_sha256":"%s","host_sha256":"%s"}\n' \
    "$GIT" "$SHA" "$(shasum -a 256 "$HERE/reflexer_host.mjs" | cut -d' ' -f1)" >"$SITE/assets/reflexer.mirror.json"
  echo "mirrored → $SITE/assets/{reflexer.wasm,reflexer_host.js,reflexer.mirror.json}"
fi
