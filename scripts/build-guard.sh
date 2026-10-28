#!/usr/bin/env bash
set -euo pipefail
contract_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -x "$contract_root/../.tooling/stellar/stellar" ]; then
 guard_cli="$contract_root/../.tooling/stellar/stellar"
else
 guard_cli="$(command -v stellar)"
fi
case "$("$guard_cli" --version | head -n 1)" in
 'stellar 28.1.0 '*) ;;
 *) echo 'Install pinned Stellar CLI 28.1.0 before building.' >&2; exit 1 ;;
esac
cd "$contract_root"
"$guard_cli" contract build --manifest-path Cargo.toml --package mierkhat-guard --locked --optimize=false

exec "$guard_cli" contract build --manifest-path Cargo.toml --package mierkhat-migration --locked --optimize=false
