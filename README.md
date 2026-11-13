# Termovies — nel terminale

**Termovies Terminal 0.4.2** porta ricerca di film, serie, anime e TV, addon JSON,
Real-Debrid/TorBox, preferiti, cronologia, 74 correnti cinematografiche e
generazione locale di prompt in un’app TUI per Linux, macOS e Windows.

[**Sito ufficiale e download →**](https://emanueleparis98.github.io/termovies/)

Linux, Ubuntu e Arch:

```sh
curl -fsSL https://github.com/Emanueleparis98/termovies/releases/latest/download/install.sh | sh
termovies
```

Windows, da PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://github.com/Emanueleparis98/termovies/releases/latest/download/install.ps1 | iex"
termovies
```

Windows, con Scoop:

```powershell
scoop bucket add termovies https://github.com/Emanueleparis98/termovies
scoop install termovies/termovies
termovies
```

![Termovies Terminal](terminal/docs/terminal.png)

[Installazione completa, Arch/pacman, macOS con uv/pipx, uso e privacy](terminal/README.md)
· [Release terminale 0.4.2](https://github.com/Emanueleparis98/termovies/releases/tag/terminal-v0.4.2)

Per i video aggiungi i tuoi addon e installa mpv o VLC. Per i prompt non serve
alcuna API AI. Le release terminal-v* sono pubblicate come ultima release del
repository e includono installer Linux/Windows, pacchetto Arch e bucket Scoop.

## Aggiornamento

- Linux/Windows: riesegui lo stesso comando di installazione; impostazioni e libreria restano.
- Scoop: `scoop update; scoop update termovies`.
- Arch: scarica il nuovo `termovies-<versione>-1-any.pkg.tar.zst` e installalo con `sudo pacman -U`.
- macOS/Linux con uv o pipx: reinstalla da Git o dal wheel della release.

## Cosa include

- Ricerca in TMDB per film e serie e nel catalogo Anime Kitsu integrato; gli
  addon Stremio/Nuvio restano disponibili per cataloghi e flussi personali.
- Flussi da addon con torrent risolti tramite Real-Debrid o TorBox.
- Preferiti, cronologia, ripresa della visione e filtri per lingua e definizione.
- 74 correnti cinematografiche consultabili offline e un compositore di prompt
  locale per ChatGPT, DeepSeek, Claude e Gemini.
- Player esterno mpv, VLC o ffplay; nessun client torrent P2P viene avviato.
- Interfaccia Textual per Linux, macOS e Windows, con installer, pacchetto
  Arch e bucket Scoop del progetto.

La ricerca predefinita usa TMDB e Kitsu; scegliendo **Addon collegati** si
includono anche i cataloghi degli addon personali. La riproduzione usa gli addon
e i servizi configurati dall’utente. La disponibilità dei cataloghi e dei flussi
dipende dai servizi esterni: un titolo trovato non garantisce una sorgente
riproducibile.

## Comandi senza interfaccia

```sh
termovies search "The Matrix" --type movie
termovies search "Cowboy Bebop" --type anime --json
termovies currents "Rossellini"
termovies prompt "Consigliami tre anime brevi, senza spoiler"
termovies doctor
termovies configure
termovies addons add
termovies addons list
```

## Sviluppo

```sh
uv venv --python 3.13
uv pip install -e '.[dev]' build
.venv/bin/python -m pytest -q
.venv/bin/python terminal/scripts/build-release.py
.venv/bin/python terminal/scripts/check-install.py
```

Su Windows usa `.venv\Scripts\python.exe`. Il workflow **Termovies Terminal**
controlla Linux e Windows con Python 3.11, 3.13 e 3.14, l’installazione in un
ambiente isolato e il pacchetto nativo Arch.

## Plugin delle correnti

Il catalogo canonico delle 74 correnti e il generatore CLI restano in
[`plugins/cinema-currents`](plugins/cinema-currents/README.md). Termovies Terminal
usa il catalogo incluso; il plugin può essere compilato e usato separatamente.
