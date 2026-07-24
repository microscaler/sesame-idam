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
3. The **platform-specific** user config dir + `/sops/age/keys.txt`:

| OS | Default path |
|---|---|
| Linux (ms02) | `~/.config/sops/age/keys.txt` |
| **macOS** | **`~/Library/Application Support/sops/age/keys.txt`** |

> **The macOS trap.** SOPS uses Go's `os.UserConfigDir()`, which on macOS is
> `~/Library/Application Support` — *not* `~/.config`. A key at
> `~/.config/sops/age/keys.txt` on a Mac is invisible: encryption still works
> (public recipient), decryption fails with *"no master key was able to
> decrypt the file"*. Put the key in the Library path, or export
> `SOPS_AGE_KEY_FILE`. Both were hit on 2026-07-25.

Likewise, the filename matters: a key at
`~/.config/sops/age/flux-shared-gitops` is invisible unless you point
`SOPS_AGE_KEY_FILE` at it or concatenate it into `keys.txt`.

### Quick diagnosis

```bash
# Works with an explicit path but not without it?  → it's discovery.
SOPS_AGE_KEY_FILE=$HOME/.config/sops/age/keys.txt sops -d file.secrets.env
```

If that succeeds, move/copy the key to your platform's default path above.

## Our two recipients

| Identity | Public key | Private key lives | Purpose |
|---|---|---|---|
| shared-k8s Flux | `age1lh3s2uy…` | `flux-system/sops-age` Secret; ms02 `~/.config/sops/age/flux-shared-gitops` | The **cluster** decrypts on reconcile (kustomize-controller) |
| Sesame admin | `age1rggv7pv…` | ms02 `~/.config/sops/age/sesame-admin.agekey`; admin workstations | **Humans** decrypt/edit/verify |

Both are listed in every `.sops.yaml` creation rule, so every file is wrapped
for both. The cluster never needs the admin key; humans never need the
cluster's.

## Setup on a machine

**Linux (e.g. ms02)** — concatenate every identity you hold into the one file
SOPS reads:

```bash
mkdir -p ~/.config/sops/age
cat flux-shared-gitops sesame-admin.agekey > ~/.config/sops/age/keys.txt
chmod 600 ~/.config/sops/age/keys.txt
```

**macOS workstation** — admin identity only, in the Library path:

```bash
mkdir -p "$HOME/Library/Application Support/sops/age"
scp ms02:~/.config/sops/age/sesame-admin.agekey \
    "$HOME/Library/Application Support/sops/age/keys.txt"
chmod 600 "$HOME/Library/Application Support/sops/age/keys.txt"
```

A laptop should hold **only** `sesame-admin.agekey`. There is no reason for a
workstation to carry the cluster's own identity — that's the separation the
two-recipient setup buys.

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
