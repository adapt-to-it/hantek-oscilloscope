#!/usr/bin/env bash
# Pubblica una nuova versione: aggiorna i numeri di versione, crea il commit e il tag v<versione>
# e li invia a GitHub. Il tag avvia il workflow Release, che costruisce e pubblica gli installer.
#
# Uso: scripts/release.sh 0.2.0
set -euo pipefail

version="${1:-}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
  echo "Uso: scripts/release.sh <versione>, per esempio 0.2.0 oppure 0.2.0-beta.1" >&2
  exit 1
fi

cd "$(git rev-parse --show-toplevel)"

if [ "$(git branch --show-current)" != "main" ]; then
  echo "La release parte solo da main." >&2
  exit 1
fi
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "Ci sono modifiche non committate: committale o annullale prima della release." >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/v$version" >/dev/null; then
  echo "Il tag v$version esiste già." >&2
  exit 1
fi

git pull --ff-only

# Versione del workspace Cargo (prima riga version = "..." dopo [workspace.package]).
perl -0pi -e 's/(\[workspace\.package\]\s*\nversion = ")[^"]+"/${1}'"$version"'"/' Cargo.toml
# Versione dell'app Tauri e del pacchetto npm dell'interfaccia.
node -e '
  const fs = require("fs");
  for (const p of ["crates/h6022-app/tauri.conf.json", "ui/package.json"]) {
    const j = JSON.parse(fs.readFileSync(p, "utf8"));
    j.version = process.argv[1];
    fs.writeFileSync(p, JSON.stringify(j, null, 2) + "\n");
  }
' "$version"
(cd ui && npm install --package-lock-only --silent)
cargo update --workspace --quiet

git add Cargo.toml Cargo.lock crates/h6022-app/tauri.conf.json ui/package.json ui/package-lock.json
git commit -m "Versione $version"
git tag -a "v$version" -m "Versione $version"
git push origin main "v$version"

echo "Tag v$version inviato. Avanzamento: https://github.com/adapt-to-it/hantek-oscilloscope/actions"
