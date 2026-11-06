#!/bin/sh
# Install a verified GitHub release into an isolated uv tool environment.
set -eu
cinematch_wheel=
cinematch_python=3.13
cinematch_modify_path=1
while [ "$#" -gt 0 ]; do
    case "$1" in
        --wheel) cinematch_wheel=$2; shift 2 ;;
        --python) cinematch_python=$2; shift 2 ;;
        --no-path) cinematch_modify_path=0; shift ;;
        --help) echo 'Uso: sh install.sh [--wheel FILE] [--python VERSIONE] [--no-path]'; exit 0 ;;
        *) echo "Opzione sconosciuta: $1" >&2; exit 2 ;;
    esac
done
cinematch_temp=$(mktemp -d)
trap 'rm -rf "$cinematch_temp"' EXIT HUP INT TERM
if [ -z "$cinematch_wheel" ]; then
    cinematch_base=https://github.com/Emanueleparis98/cinematch/releases
    curl --proto '=https' --tlsv1.2 -fsSL "https://raw.githubusercontent.com/Emanueleparis98/cinematch/main/terminal/latest.json" -o "$cinematch_temp/latest.json"
    cinematch_version=$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([0-9.]*\)".*/\1/p' "$cinematch_temp/latest.json")
    case "$cinematch_version" in ''|*[!0-9.]*) echo 'Versione della release non valida.' >&2; exit 2 ;; esac
    if ! printf '%s\n' "$cinematch_version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        echo 'Versione della release non valida.' >&2; exit 2
    fi
    cinematch_wheel_name="cinematch-$cinematch_version-py3-none-any.whl"
    curl --proto '=https' --tlsv1.2 -fsSL "$cinematch_base/download/terminal-v$cinematch_version/$cinematch_wheel_name" -o "$cinematch_temp/$cinematch_wheel_name"
    curl --proto '=https' --tlsv1.2 -fsSL "$cinematch_base/download/terminal-v$cinematch_version/SHA256SUMS-terminal.txt" -o "$cinematch_temp/SHA256SUMS-terminal.txt"
    cinematch_wheel="$cinematch_temp/$cinematch_wheel_name"
fi
cinematch_wheel_dir=$(CDPATH= cd -- "$(dirname -- "$cinematch_wheel")" && pwd)
cinematch_wheel_name=$(basename -- "$cinematch_wheel")
cinematch_wheel="$cinematch_wheel_dir/$cinematch_wheel_name"
cinematch_checksums="$cinematch_wheel_dir/SHA256SUMS-terminal.txt"
if [ ! -f "$cinematch_checksums" ]; then
    echo 'Manca SHA256SUMS-terminal.txt accanto al pacchetto. Scaricalo dalla stessa release.' >&2; exit 2
fi
cinematch_expected=$(awk -v name="$cinematch_wheel_name" '$2 == name {print $1}' "$cinematch_checksums")
cinematch_actual=$(sha256sum "$cinematch_wheel" | awk '{print $1}')
if [ -z "$cinematch_expected" ] || [ "$cinematch_expected" != "$cinematch_actual" ]; then
    echo 'Checksum del pacchetto non valido; installazione annullata.' >&2; exit 2
fi
cinematch_uv=${CINEMATCH_UV:-}
if [ -z "$cinematch_uv" ]; then
    cinematch_uv=$(command -v uv || true)
fi
if [ -z "$cinematch_uv" ] && [ -x "${HOME}/.local/bin/uv" ]; then
    cinematch_uv="${HOME}/.local/bin/uv"
fi
if [ -z "$cinematch_uv" ]; then
    echo 'Installo uv dal sito ufficiale Astral…'
    curl --proto '=https' --tlsv1.2 -fsSL https://astral.sh/uv/install.sh -o "$cinematch_temp/uv-install.sh"
    UV_NO_MODIFY_PATH=1 sh "$cinematch_temp/uv-install.sh"
    cinematch_uv="${HOME}/.local/bin/uv"
fi
"$cinematch_uv" tool install --python "$cinematch_python" --force "$cinematch_wheel"
if [ "$cinematch_modify_path" -eq 1 ]; then
    "$cinematch_uv" tool update-shell
fi
cinematch_bin=$("$cinematch_uv" tool dir --bin)
"$cinematch_bin/cinematch" --version
echo 'Installazione completata. Avvia con: cinematch'
echo 'Se il comando non viene riconosciuto, riapri il terminale per aggiornare PATH.'
