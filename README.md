# CineMatch — nel terminale

**CineMatch Terminal 0.2.1** porta ricerca di film, serie, anime e TV, addon JSON,
Real-Debrid/TorBox, preferiti, cronologia, 74 correnti cinematografiche e
generazione locale di prompt in un’app TUI per Linux e Windows.

```sh
curl -fsSL https://raw.githubusercontent.com/Emanueleparis98/cinematch/main/terminal/install.sh | sh
cinematch
```

Su Windows, con Scoop già installato:

```powershell
scoop bucket add cinematch https://github.com/Emanueleparis98/cinematch
scoop install cinematch/cinematch
cinematch
```

![CineMatch Terminal](terminal/docs/terminal.png)

[Installazione completa, Arch/pacman, Windows PowerShell, uso e privacy](terminal/README.md)
· [Release terminale 0.2.1](https://github.com/Emanueleparis98/cinematch/releases/tag/terminal-v0.2.1)

Per i video aggiungi i tuoi addon e installa mpv o VLC. Per i prompt non serve
alcuna API AI. L’app terminale ha un proprio canale di aggiornamento.

## Cosa include

- Ricerca nei cataloghi degli addon Stremio/Nuvio e, se collegata, in TMDB.
- Flussi da addon con torrent risolti tramite Real-Debrid o TorBox.
- Preferiti, cronologia, ripresa della visione e filtri per lingua e definizione.
- 74 correnti cinematografiche consultabili offline e un compositore di prompt
  locale per ChatGPT, DeepSeek, Claude e Gemini.
- Player esterno mpv, VLC o ffplay; nessun client torrent P2P viene avviato.
- Interfaccia Textual per Linux e Windows, con installer, pacchetto Arch e bucket
  Scoop del progetto.

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
