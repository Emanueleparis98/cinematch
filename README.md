# CineMatch — nel terminale

**CineMatch Terminal 0.3.0** porta ricerca di film, serie, anime e TV, addon JSON,
Real-Debrid/TorBox, preferiti, cronologia, 74 correnti cinematografiche e
generazione locale di prompt in un’app TUI per Linux, macOS e Windows.

Linux, Ubuntu e Arch:

```sh
curl -fsSL https://github.com/Emanueleparis98/cinematch/releases/latest/download/install.sh | sh
cinematch
```

Windows, da PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://github.com/Emanueleparis98/cinematch/releases/latest/download/install.ps1 | iex"
cinematch
```

Windows, con Scoop:

```powershell
scoop bucket add cinematch https://github.com/Emanueleparis98/cinematch
scoop install cinematch/cinematch
cinematch
```

![CineMatch Terminal](terminal/docs/terminal.png)

[Installazione completa, Arch/pacman, macOS con uv/pipx, uso e privacy](terminal/README.md)
· [Release terminale 0.3.0](https://github.com/Emanueleparis98/cinematch/releases/tag/terminal-v0.3.0)

Per i video aggiungi i tuoi addon e installa mpv o VLC. Per i prompt non serve
alcuna API AI. Le release terminal-v* sono pubblicate come ultima release del
repository e includono installer Linux/Windows, pacchetto Arch e bucket Scoop.

## Aggiornamento

- Linux/Windows: riesegui lo stesso comando di installazione; impostazioni e libreria restano.
- Scoop: `scoop update; scoop update cinematch`.
- Arch: scarica il nuovo `cinematch-<versione>-1-any.pkg.tar.zst` e installalo con `sudo pacman -U`.
- macOS/Linux con uv o pipx: reinstalla da Git o dal wheel della release.

## Cosa include

- Ricerca nei cataloghi degli addon Stremio/Nuvio e, se collegata, in TMDB.
- Flussi da addon con torrent risolti tramite Real-Debrid o TorBox.
- Preferiti, cronologia, ripresa della visione e filtri per lingua e definizione.
- 74 correnti cinematografiche consultabili offline e un compositore di prompt
  locale per ChatGPT, DeepSeek, Claude e Gemini.
- Player esterno mpv, VLC o ffplay; nessun client torrent P2P viene avviato.
- Interfaccia Textual per Linux, macOS e Windows, con installer, pacchetto
  Arch e bucket Scoop del progetto.

Il flusso di ricerca e riproduzione usa esclusivamente gli addon collegati e i
servizi configurati dall’utente. La disponibilità dei cataloghi e dei flussi
dipende dai servizi esterni: un titolo trovato non garantisce una sorgente
riproducibile.

## Comandi senza interfaccia

```sh
cinematch search "The Matrix" --type movie
cinematch search "Cowboy Bebop" --type anime --json
cinematch currents "Rossellini"
cinematch prompt "Consigliami tre anime brevi, senza spoiler"
cinematch doctor
cinematch configure
cinematch addons add
cinematch addons list
```

## Sviluppo

```sh
uv venv --python 3.13
uv pip install -e '.[dev]' build
.venv/bin/python -m pytest -q
.venv/bin/python terminal/scripts/build-release.py
.venv/bin/python terminal/scripts/check-install.py
```

Su Windows usa `.venv\Scripts\python.exe`. Il workflow **CineMatch Terminal**
controlla Linux e Windows con Python 3.11, 3.13 e 3.14, l’installazione in un
ambiente isolato e il pacchetto nativo Arch.

## Plugin delle correnti

Il catalogo canonico delle 74 correnti e il generatore CLI restano in
[`plugins/cinema-currents`](plugins/cinema-currents/README.md). CineMatch Terminal
usa il catalogo incluso; il plugin può essere compilato e usato separatamente.
