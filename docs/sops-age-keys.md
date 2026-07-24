# SOPS age keys: who can encrypt, who can decrypt

## The rule that explains every confusing symptom

**Encrypting needs only a PUBLIC recipient** (read from `.sops.yaml`), so it
works on any machine, with no key installed. **Decrypting needs a PRIVATE
identity**, which SOPS must be able to *find*.

That asymmetry produces the classic failure: *"I encrypted it fine but I can't
decrypt it."* Nine times out of ten the key is not missing — SOPS just isn't
looking where it lives.

## Where SOPS looks for private keys

1. `$SOPS_AGE_KEY` (the key material itself, inline)
2. `$SOPS_AGE_KEY_FILE` (path to a key file)
3. `~/.config/sops/age/keys.txt`  ← the default, and **only** this filename

A key sitting at `~/.config/sops/age/flux-shared-gitops` is invisible to SOPS
unless you point `SOPS_AGE_KEY_FILE` at it or concatenate it into `keys.txt`.
(That was the 2026-07-25 incident: the right key was on the box the whole
time under the wrong filename.)

## Our two recipients

| Identity | Public key | Private key lives | Purpose |
|---|---|---|---|
| shared-k8s Flux | `age1lh3s2uy…` | `flux-system/sops-age` Secret; ms02 `~/.config/sops/age/flux-shared-gitops` | The **cluster** decrypts on reconcile (kustomize-controller) |
| Sesame admin | `age1rggv7pv…` | ms02 `~/.config/sops/age/sesame-admin.agekey`; admin workstations | **Humans** decrypt/edit/verify |

Both are listed in every `.sops.yaml` creation rule, so every file is wrapped
for both. The cluster never needs the admin key; humans never need the
cluster's.

## Setup on a machine

```bash
mkdir -p ~/.config/sops/age
# Concatenate every identity you hold into the file SOPS actually reads:
cat flux-shared-gitops sesame-admin.agekey > ~/.config/sops/age/keys.txt
chmod 600 ~/.config/sops/age/keys.txt
```

On a laptop you typically want **only** `sesame-admin.agekey` — there is no
reason for a workstation to hold the cluster's identity.

## Everyday commands

```bash
sops -e -i path/to/file.secrets.env   # encrypt in place
sops -d path/to/file.secrets.env      # decrypt to stdout
sops path/to/file.secrets.env         # open decrypted in $EDITOR, re-encrypt on save
sops updatekeys path/to/file          # re-wrap after changing .sops.yaml recipients
```

`updatekeys` is the one people forget: adding a recipient to `.sops.yaml`
does **not** retro-wrap existing files. Run it for each file, then commit.

## Which files are encrypted

`.sops.yaml` matches by path:

- `deployment-configuration/profiles/**/*.secrets.env` — whole file
- `deployment-configuration/profiles/**/*.secret.yaml` — only `data` /
  `stringData` (so Secret metadata stays reviewable in diffs)

## Hygiene

- Never `cp` a `.EXAMPLE` to its real name, paste credentials, and leave it —
  encrypt immediately. Plaintext credentials on a shared build host are a
  disclosure even when untracked.
- If a credential has sat in plaintext anywhere shared, **rotate it** at the
  provider. Rotation is cheap; assuming it was fine is not.
- `git status` before committing a secrets directory. Ciphertext should show
  `ENC[AES256_GCM,…]` and a `sops_` metadata block — if you see readable
  values, stop.
