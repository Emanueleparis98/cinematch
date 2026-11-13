#!/bin/sh
# Install a verified GitHub release into an isolated uv tool environment.
set -eu
termovies_wheel=
termovies_python=3.13
termovies_modify_path=1
while [ "$#" -gt 0 ]; do
    case "$1" in
        --wheel) termovies_wheel=$2; shift 2 ;;
        --python) termovies_python=$2; shift 2 ;;
        --no-path) termovies_modify_path=0; shift ;;
        --help) echo 'Uso: sh install.sh [--wheel FILE] [--python VERSIONE] [--no-path]'; exit 0 ;;
        *) echo "Opzione sconosciuta: $1" >&2; exit 2 ;;
    esac
done
termovies_temp=$(mktemp -d)
trap 'rm -rf "$termovies_temp"' EXIT HUP INT TERM
if [ -z "$termovies_wheel" ]; then
    termovies_base=https://github.com/Emanueleparis98/termovies/releases
    curl --proto '=https' --tlsv1.2 -fsSL "$termovies_base/latest/download/latest.json" -o "$termovies_temp/latest.json"
    termovies_version=$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([0-9.]*\)".*/\1/p' "$termovies_temp/latest.json")
    case "$termovies_version" in ''|*[!0-9.]*) echo 'Versione della release non valida.' >&2; exit 2 ;; esac
    if ! printf '%s\n' "$termovies_version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        echo 'Versione della release non valida.' >&2; exit 2
    fi
    termovies_wheel_name="termovies-$termovies_version-py3-none-any.whl"
    curl --proto '=https' --tlsv1.2 -fsSL "$termovies_base/download/terminal-v$termovies_version/$termovies_wheel_name" -o "$termovies_temp/$termovies_wheel_name"
    curl --proto '=https' --tlsv1.2 -fsSL "$termovies_base/download/terminal-v$termovies_version/SHA256SUMS" -o "$termovies_temp/SHA256SUMS"
    termovies_wheel="$termovies_temp/$termovies_wheel_name"
fi
termovies_wheel_dir=$(CDPATH= cd -- "$(dirname -- "$termovies_wheel")" && pwd)
termovies_wheel_name=$(basename -- "$termovies_wheel")
termovies_wheel="$termovies_wheel_dir/$termovies_wheel_name"
termovies_checksums="$termovies_wheel_dir/SHA256SUMS"
if [ ! -f "$termovies_checksums" ]; then
    echo 'Manca SHA256SUMS accanto al pacchetto. Scaricalo dalla stessa release.' >&2; exit 2
fi
termovies_expected=$(awk -v name="$termovies_wheel_name" '$2 == name {print $1}' "$termovies_checksums")
termovies_actual=$(sha256sum "$termovies_wheel" | awk '{print $1}')
if [ -z "$termovies_expected" ] || [ "$termovies_expected" != "$termovies_actual" ]; then
    echo 'Checksum del pacchetto non valido; installazione annullata.' >&2; exit 2
fi
termovies_uv=${TERMOVIES_UV:-}
if [ -z "$termovies_uv" ]; then
    termovies_uv=$(command -v uv || true)
fi
if [ -z "$termovies_uv" ] && [ -x "${HOME}/.local/bin/uv" ]; then
    termovies_uv="${HOME}/.local/bin/uv"
fi
if [ -z "$termovies_uv" ]; then
    echo 'Installo uv dal sito ufficiale Astral…'
    curl --proto '=https' --tlsv1.2 -fsSL https://astral.sh/uv/install.sh -o "$termovies_temp/uv-install.sh"
    UV_NO_MODIFY_PATH=1 sh "$termovies_temp/uv-install.sh"
    termovies_uv="${HOME}/.local/bin/uv"
fi
"$termovies_uv" tool install --python "$termovies_python" --force "$termovies_wheel"
if [ "$termovies_modify_path" -eq 1 ]; then
    "$termovies_uv" tool update-shell
fi
termovies_bin=$("$termovies_uv" tool dir --bin)
"$termovies_bin/termovies" --version
echo 'Installazione completata. Avvia con: termovies'
echo 'Se il comando non viene riconosciuto, riapri il terminale per aggiornare PATH.'
