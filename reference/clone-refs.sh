#!/usr/bin/env bash
# Clone the reference implementations rauth-rs is surveyed against: Rust
# providers and libraries, and the Go projects used for protocol and domain design.
#
# Shallow clones into reference/src/, which is gitignored — these are upstream
# repos, not ours. Run after a fresh clone; re-run to refresh.
#
# Kanidm is deliberately absent: its AGENTS.md opts out of AI use. See
# reference/README.md. Check every new entry for an opt-out before adding it.
set -euo pipefail

cd "$(dirname "$0")"
mkdir -p src && cd src

clone_shallow() {
  local repo="$1" name="$2"
  if [ -d "$name" ]; then
    echo "==> $name: refreshing"
    git -C "$name" fetch --depth 1 --quiet origin && git -C "$name" reset --hard --quiet origin/HEAD 2>/dev/null || true
  else
    echo "==> $name: cloning"
    git clone --depth 1 --quiet "https://github.com/$repo.git" "$name"
  fi
  printf '    %s @ %s\n' "$name" "$(git -C "$name" rev-parse --short HEAD)"
}

# Providers
clone_shallow sebadob/rauthy       rauthy
clone_shallow ferriskey/ferriskey  ferriskey
clone_shallow ELares/IronAuth      ironauth
clone_shallow geheb/gtid           gtid
clone_shallow bpteodor/flipid      flipid

# Server libraries
clone_shallow 197g/oxide-auth      oxide-auth
clone_shallow MattJackson/oauth-as oauth-as

# Client libraries — protocol types and a candidate test client
clone_shallow ramosbugs/openidconnect-rs openidconnect-rs
clone_shallow ramosbugs/oauth2-rs        oauth2-rs

# Go: protocol and domain design
clone_shallow ory/hydra  hydra
clone_shallow ory/kratos kratos
clone_shallow dexidp/dex dex

# Zitadel is large and AGPL-3.0: sparse clone of the parts read for design only.
if [ ! -d zitadel ]; then
  echo "==> zitadel: cloning (sparse)"
  git clone --depth 1 --quiet --filter=blob:none --no-checkout \
    https://github.com/zitadel/zitadel.git zitadel
  git -C zitadel sparse-checkout init --cone
  git -C zitadel sparse-checkout set apps/login \
    internal/eventstore internal/command internal/query internal/repository internal/domain
  git -C zitadel checkout --quiet
else
  echo "==> zitadel: refreshing"
  git -C zitadel fetch --depth 1 --quiet origin && git -C zitadel reset --hard --quiet origin/HEAD 2>/dev/null || true
fi
printf '    zitadel @ %s\n' "$(git -C zitadel rev-parse --short HEAD)"

echo
echo "Done. See reference/README.md for what to read in each."
