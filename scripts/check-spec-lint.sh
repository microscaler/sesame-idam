#!/usr/bin/env bash
# Lint every live OpenAPI spec with brrtrouter-gen at the BRRTRouter rev the services build against, failing on any
# error (BRRTRouter linter: e.g. OpenAPI 3.0 `nullable: true` in a 3.1 document, which 3.1 ignores, so a null then
# fails response validation: PriceWhisperer portfolio /equity-curve 500s, pp 25 Oct 2026).
#
#   scripts/check-spec-lint.sh                 # the live specs below
#   scripts/check-spec-lint.sh path/a.yaml ... # just these
#
# brrtrouter-gen is installed once per rev (unless BRRTROUTER_GEN names a binary) under ${BRRTROUTER_GEN_CACHE:-$HOME/.cache/brrtrouter-gen}/<rev>.
set -euo pipefail
cd "$(dirname "$0")/.."

REV=$(sed -n 's/^brrtrouter = { git = "https:\/\/github.com\/microscaler\/BRRTRouter.git", rev = "\([0-9a-f]*\)".*/\1/p' microservices/Cargo.toml | head -1)
test -n "$REV" || { echo "no BRRTRouter rev in microservices/Cargo.toml"; exit 1; }
ROOT="${BRRTROUTER_GEN_CACHE:-$HOME/.cache/brrtrouter-gen}/$REV"
GEN="${BRRTROUTER_GEN:-$ROOT/bin/brrtrouter-gen}"  # BRRTROUTER_GEN: use a local build instead
if [ ! -x "$GEN" ]; then
  echo "installing brrtrouter-gen at BRRTRouter $REV"
  cargo install --quiet --locked --git https://github.com/microscaler/BRRTRouter.git --rev "$REV" \
    --bin brrtrouter-gen --root "$ROOT" brrtrouter
fi

if [ $# -gt 0 ]; then
  SPECS=("$@")
else
  mapfile -t SPECS < <(ls openapi/idam/*/openapi.yaml 2>/dev/null | sort)
fi

failed=0
for spec in "${SPECS[@]}"; do
  if "$GEN" lint --spec "$spec" --fail-on-error > /tmp/spec-lint.$$ 2>&1; then
    echo "ok    $spec"
  else
    echo "FAIL  $spec"
    grep -iE "error" /tmp/spec-lint.$$ | head -20 | sed 's/^/      /'
    failed=1
  fi
done
rm -f /tmp/spec-lint.$$
exit $failed
