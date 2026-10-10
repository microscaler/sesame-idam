#!/usr/bin/env bash
# Pre-commit: every secret file given (or, with no arguments, every tracked one) is SOPS-encrypted. A secret file is
# *.secrets.env, *.secrets.yaml or *.secret.yaml (the EXAMPLE templates are not secrets). Encrypted means it carries
# SOPS metadata: sops_version= in a dotenv file, a top-level sops: key in YAML.
set -euo pipefail
if [ "$#" -eq 0 ]; then
  mapfile -t files < <(git ls-files)
else
  files=("$@")
fi
rc=0
for f in "${files[@]}"; do
  case "$f" in
    *.EXAMPLE) continue ;;
    *.secrets.env) grep -q '^sops_version=' "$f" || { echo "not SOPS-encrypted: $f (sops --encrypt --in-place $f)"; rc=1; } ;;
    *.secrets.yaml|*.secrets.yml|*.secret.yaml|*.secret.yml)
      grep -q '^sops:' "$f" || { echo "not SOPS-encrypted: $f (sops --encrypt --in-place $f)"; rc=1; } ;;
  esac
done
exit $rc
