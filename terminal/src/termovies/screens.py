from __future__ import annotations

import json
import subprocess
import webbrowser
from pathlib import Path
from typing import ClassVar

import pyperclip
from rich.table import Table as RichTable
from rich.text import Text
from textual import on, work
from textual.app import ComposeResult
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import ModalScreen
from textual.widgets import (
    Button,
    DataTable,
    Input,
    Label,
    Select,
    Static,
    TabbedContent,
    TabPane,
    TextArea,
)

from .addons import parse_streams
from .models import CineError, Stream, plain, web_url
from .prompts import KINDS, MODES, PROVIDERS, generate, period, search_currents
from .stream_filters import LANGUAGES, QUALITIES, matches_stream, source_info


def copy_text(app, value):
    try:
        pyperclip.copy(value)
    except (pyperclip.PyperclipException, OSError, RuntimeError, subprocess.SubprocessError):
        app.copy_to_clipboard(value)


class DialogScreen(ModalScreen):
    def on_resize(self, event):
        self.set_class(event.size.height < 40, "small")


class HelpScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi"), ("f1", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(id="help-dialog", classes="dialog"):
            yield Static("GUIDA RAPIDA", classes="dialog-title")
            yield Static("Tutto raggiungibile da tastiera; il mouse resta supportato.", classes="dialog-subtitle")
            with VerticalScroll(id="help-scroll"):
                yield Static(self.shortcuts(), classes="help-table")
                yield Static("COSE DA SAPERE", classes="section-label")
                yield Static(self.notes(), classes="help-notes")
            with Horizontal(classes="actions"):
                yield Button("Chiudi", id="close")

    @staticmethod
    def shortcuts():
        table = RichTable.grid(padding=(0, 2))
        table.add_column(style="bold #a8ee78", no_wrap=True)
        table.add_column(style="#cbd8e4")
        for keys, action in [
            ("/", "Cerca"),
            ("↑ / ↓", "Naviga i risultati"),
            ("Invio", "Apri il pannello Flussi"),
            ("F1", "Questa guida"),
            ("Ctrl+R", "Ripeti l’ultima ricerca"),
            ("Ctrl+K", "Correnti cinematografiche"),
            ("Ctrl+P", "Componi un prompt"),
            ("Ctrl+A", "Addon e flussi"),
            ("Ctrl+S", "Impostazioni"),
            ("Ctrl+O", "Apri URL o JSON"),
            ("Ctrl+X", "Ferma il video"),
            ("Esc", "Chiudi il pannello attivo"),
            ("Ctrl+Q", "Esci da Termovies"),
        ]:
            table.add_row(keys, action)
        return table

    @staticmethod
    def notes():
        text = Text()
        for line in [
            "La ricerca predefinita usa Kitsu e TMDB; scegli Addon collegati per i cataloghi personali.",
            "I flussi arrivano dagli addon. I torrent richiedono Real-Debrid o TorBox configurato.",
            "Preferiti e cronologia restano sul tuo computer, nella cartella dati locale.",
            "TMDB serve per film e serie; il catalogo Anime Kitsu funziona senza chiavi.",
        ]:
            text.append("• ", style="bold #a8ee78")
            text.append(line+"\n", style="#91a3b5")
        return text

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class SettingsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        config = self.app.service.store.config
        with Vertical(id="settings-dialog", classes="dialog"):
            yield Static("IMPOSTAZIONI", classes="dialog-title")
            yield Static("Collega i servizi che vuoi usare. Le chiavi restano solo su questo computer.", classes="dialog-subtitle")
            with VerticalScroll():
                yield Static("METADATI", classes="section-label")
                with Vertical(classes="panel"):
                    yield Label("TMDB · API v3 oppure Read Access Token")
                    yield Input(value=config["tmdb_key"], password=True, id="tmdb-key",
                                placeholder="Consigliato per film e serie: ricerca e metadati in italiano")
                    yield Static("Senza chiave restano disponibili gli anime del catalogo Kitsu e i tuoi addon.", classes="hint")
                yield Static("DEBRID", classes="section-label")
                with Vertical(classes="panel"):
                    yield Label("Servizio per i torrent restituiti dagli addon")
                    with Horizontal(classes="form-row"):
                        yield Select([("Nessuno", ""), ("Real-Debrid", "rd"), ("TorBox", "torbox")],
                                     value=config["debrid_provider"], allow_blank=False, id="debrid-provider")
                        yield Input(value=config["debrid_key"], password=True, id="debrid-key",
                                    placeholder="Chiave personale del servizio")
                    yield Static("Una sola chiave globale per tutti gli addon. Termovies non avvia client torrent P2P.", classes="hint")
                yield Static("PLAYER", classes="section-label")
                with Vertical(classes="panel"):
                    yield Label("Player video")
                    yield Select([("Automatico · mpv, VLC, ffplay", "auto"), ("mpv · consigliato", "mpv"),
                                  ("VLC", "vlc"), ("ffplay", "ffplay")],
                                 value=config.get("player", "auto"), allow_blank=False, id="player-choice")
                    yield Static("mpv offre la ripresa della visione su Linux; VLC è l'alternativa più diffusa.", classes="hint")
                yield Static("DATI LOCALI", classes="section-label")
                with Vertical(classes="panel"):
                    yield Static("Configurazione  "+str(self.app.service.store.config_path), classes="path")
                    yield Static("Libreria           "+str(self.app.service.store.data_path), classes="path")
                    yield Static("Le chiavi non sono cifrate, ma il file è leggibile solo dal tuo utente (0600 su Linux).", classes="hint")
            yield Static("", id="settings-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Salva", variant="primary", id="save-settings")
                yield Button("Verifica connessioni", id="verify-settings")
                yield Button("Chiudi", id="close")

    def save(self):
        config = self.app.service.store.config
        config.update(tmdb_key=self.query_one("#tmdb-key", Input).value.strip(),
                      debrid_provider=str(self.query_one("#debrid-provider", Select).value),
                      debrid_key=self.query_one("#debrid-key", Input).value.strip(),
                      player=str(self.query_one("#player-choice", Select).value))
        self.app.service.store.save_config()
        self.app.service.clear_cache()

    @on(Button.Pressed, "#save-settings")
    def save_settings(self):
        try:
            self.save()
            self.query_one("#settings-status", Static).update("Impostazioni salvate.")
        except OSError:
            self.query_one("#settings-status", Static).update("Non posso salvare il file di configurazione.")

    @on(Button.Pressed, "#verify-settings")
    @work(exclusive=True)
    async def verify(self):
        status = self.query_one("#settings-status", Static)
        status.update("Verifico i servizi…")
        try:
            self.save()
            messages = []
            if self.app.service.store.config["tmdb_key"]:
                await self.app.service.tmdb.verify()
                messages.append("TMDB collegato")
            if self.app.service.store.config["debrid_provider"]:
                await self.app.service.debrid.verify()
                messages.append("Debrid collegato")
            status.update(" · ".join(messages) or "Nessun servizio da verificare. Kitsu funziona senza chiavi; TMDB serve per film e serie.")
        except (CineError, OSError) as e:
            status.update(str(e) if isinstance(e, CineError) else "Non posso salvare le impostazioni.")

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class AddonsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(id="addons-dialog", classes="dialog"):
            yield Static("ADDON E FONTI", classes="dialog-title")
            yield Static("Aggiungi un manifest Stremio/Nuvio. Le configurazioni personali restano nel link dell’addon.", classes="dialog-subtitle")
            yield Label("Manifest JSON", classes="field-label")
            with Horizontal(id="addon-form"):
                yield Input(placeholder="https://…/manifest.json oppure stremio://…", password=True, id="addon-url")
                yield Button("Aggiungi", variant="primary", id="add-addon")
            with Vertical(id="addon-table-wrap"):
                yield DataTable(id="addon-table", cursor_type="row", zebra_stripes=True)
                yield Static("Nessun addon personale.\nKitsu è già integrato; aggiungi un manifest per cataloghi e flussi extra.",
                             id="addon-empty", classes="empty-panel")
            yield Static("", id="addon-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Attiva / disattiva", id="toggle-addon")
                yield Button("Rimuovi", id="remove-addon")
                yield Button("Chiudi", id="close")

    def on_mount(self):
        table = self.query_one(DataTable)
        for label, width in [("Addon", 46), ("Stato", 11), ("Tipi", 20), ("Cataloghi", 10)]:
            table.add_column(label, width=width)
        self.fill()

    def fill(self):
        table = self.query_one(DataTable)
        table.clear()
        items = [a for a in self.app.service.store.config["addons"] if isinstance(a, dict)]
        for i, a in enumerate(items):
            active = a.get("enabled", True)
            table.add_row(Text(a["name"]), Text("● Attivo", style="#a8ee78") if active else Text("○ Spento", style="#7f90a2"),
                          Text(", ".join(a["types"])), str(len(a.get("catalogs", []))), key=str(i))
        table.display = bool(items)
        empty = self.query_one("#addon-empty", Static)
        empty.display = not items

    @on(Input.Submitted, "#addon-url")
    @on(Button.Pressed, "#add-addon")
    @work(exclusive=True)
    async def add(self):
        button = self.query_one("#add-addon", Button)
        button.disabled = True
        status = self.query_one("#addon-status", Static)
        status.update("Leggo il manifest…")
        try:
            addon = await self.app.service.addons.add(self.query_one("#addon-url", Input).value)
            if not self.is_mounted:
                return
            self.app.service.clear_cache()
            self.query_one("#addon-url", Input).value = ""
            self.fill()
            status.update(addon["name"]+" aggiunto. La chiave debrid globale sarà usata per i torrent restituiti dall’addon.")
        except (CineError, OSError, ValueError, TypeError):
            if self.is_mounted:
                status.update("Impossibile aggiungere l’addon. Verifica manifest, configurazione e collegamento al sito.")
        finally:
            if self.is_mounted:
                button.disabled = False

    @on(Button.Pressed, "#toggle-addon")
    @on(Button.Pressed, "#remove-addon")
    def edit(self, event):
        items = self.app.service.store.config["addons"]
        row = self.query_one(DataTable).cursor_row
        if 0 <= row < len(items):
            if event.button.id == "remove-addon":
                items.pop(row)
            else:
                items[row]["enabled"] = not items[row].get("enabled", True)
            try:
                self.app.service.store.save_config()
                self.fill()
            except OSError:
                self.query_one("#addon-status", Static).update("Non posso salvare la modifica degli addon.")

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class StreamsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def __init__(self, media, video_id="", imported=None):
        super().__init__()
        self.media, self.video_id, self.imported = media, video_id, imported
        self.items = []
        self.visible_items = []
        self.episodes = []
        self.stream_generation = 0
        self.chosen_season = None
        self.has_episode_picker = media.type in {"series", "anime"} and imported is None

    def compose(self) -> ComposeResult:
        with Vertical(id="streams-dialog", classes="dialog"):
            yield Static(Text(self.media.title), classes="dialog-title")
            yield Static("Scegli una sorgente, controlla lingua e definizione, poi premi Riproduci.", classes="dialog-subtitle")
            with Horizontal(id="stream-episode-row"):
                yield Select([("Stagione 1", 1)], value=1, allow_blank=False, id="stream-season")
                yield Select([("Caricamento episodi…", "")], value="", allow_blank=False, id="stream-episode")
            yield Static("Cerco i flussi negli addon attivi…", id="streams-status", markup=False)
            with Horizontal(id="stream-options"):
                yield Select(LANGUAGES, value=self.app.stream_language, allow_blank=False, id="stream-language")
                yield Select(QUALITIES, value=self.app.stream_quality, allow_blank=False, id="stream-quality")
                yield Button("Azzera", id="reset-picker-filters")
            yield Input(placeholder="Cerca nei flussi · nome, addon o dettagli…", id="stream-filter")
            yield Static("", id="stream-count", markup=False)
            yield DataTable(id="stream-table", cursor_type="row", zebra_stripes=True)
            yield Static("", id="stream-description", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Riproduci", variant="primary", id="play", disabled=True)
                yield Button("Apri fonte", id="browser", disabled=True)
                yield Button("Riprova", id="retry-streams")
                yield Button("Gestisci addon", id="manage-addons")
                yield Button("Chiudi", id="close")

    def on_mount(self):
        self.query_one(DataTable).add_columns("Definizione", "Lingua", "Fonte", "Nome", "Tipo")
        self.call_after_refresh(self.resize_stream_columns)
        self.query_one("#retry-streams").display = self.imported is None
        self.query_one("#stream-episode-row").display = self.has_episode_picker
        if self.has_episode_picker:
            seasons = self.media.available_seasons
            season = self.media.season_for_video(self.video_id)
            season = season if season in seasons else seasons[0]
            select = self.query_one("#stream-season", Select)
            with self.prevent(Select.Changed):
                select.set_options([("Speciali" if s == 0 else f"Stagione {s}", s) for s in seasons])
                select.value = season
            self.call_later(self.choose_season, season)
        else:
            self.call_later(self.reload_streams)

    def on_unmount(self):
        self.workers.cancel_group(self, "streams")
        self.workers.cancel_group(self, "play")
        self.workers.cancel_group(self, "stream-episodes")

    def on_resize(self, event):
        self.call_after_refresh(self.resize_stream_columns)

    def resize_stream_columns(self):
        if not self.is_mounted or not self.display:
            return
        table = self.query_one(DataTable)
        if table.size.width <= 0 or len(table.columns) != 5:
            return
        # Reserve the cell padding and the vertical scrollbar so the five
        # columns fit without a horizontal scrollbar; the stream name gets
        # all the remaining width.
        available = table.size.width - 2*table.cell_padding*5 - 2
        widths = (11, 8, 22, max(20, available - 49), 8)
        changed = False
        for key, width in zip(table.columns, widths):
            column = table.columns[key]
            if column.width != width or column.auto_width:
                column.width, column.auto_width = width, False
                changed = True
        if changed:
            table._require_update_dimensions = True
            table._clear_caches()
            table.check_idle()
            table.refresh(layout=True)

    def clear_streams(self, message):
        if not self.is_mounted or not self.display:
            return
        self.stream_generation += 1
        self.workers.cancel_group(self, "streams")
        self.items = []
        self.fill()
        self.query_one("#streams-status", Static).update(message)

    @on(Select.Changed, "#stream-season")
    def change_season(self, event):
        if self.has_episode_picker and isinstance(event.value, int):
            self.choose_season(event.value)

    def choose_season(self, season):
        if not self.is_mounted or not self.display or season != self.query_one("#stream-season", Select).value or season == self.chosen_season:
            return
        self.chosen_season = season
        preferred = self.video_id
        self.video_id = ""
        self.episodes = []
        select = self.query_one("#stream-episode", Select)
        with self.prevent(Select.Changed):
            select.set_options([("Caricamento episodi…", "")])
            select.value = ""
        select.disabled = True
        self.clear_streams("Carico gli episodi della stagione…")
        self.load_episodes(season, preferred)

    @work(exclusive=True, group="stream-episodes")
    async def load_episodes(self, season, preferred):
        try:
            episodes = await self.app.service.episodes(self.media, season)
            if (not self.is_mounted or not self.display) or self.query_one("#stream-season", Select).value != season:
                return
            self.episodes = episodes
            select = self.query_one("#stream-episode", Select)
            with self.prevent(Select.Changed):
                select.set_options([(f"Episodio {v.get('episode', i+1):02} · {plain(v.get('title') or v.get('name') or 'Episodio', 150)}", v["id"]) for i, v in enumerate(episodes)] or [("Nessun episodio disponibile", "")])
                self.video_id = preferred if any(v["id"] == preferred for v in episodes) else episodes[0]["id"] if episodes else ""
                select.value = self.video_id
            select.disabled = not episodes
            if self.video_id:
                self.reload_streams()
            else:
                self.query_one("#streams-status", Static).update("Nessun episodio disponibile per questa stagione. Scegli un’altra stagione o fonte.")
        except (CineError, ValueError, TypeError):
            if self.is_mounted and self.display and self.query_one("#stream-season", Select).value == season:
                with self.prevent(Select.Changed):
                    self.query_one("#stream-episode", Select).set_options([("Episodi non disponibili", "")])
                self.query_one("#streams-status", Static).update("Non riesco a leggere gli episodi. Scegli un’altra stagione o riprova aprendo Flussi.")

    @on(Select.Changed, "#stream-episode")
    def change_episode(self, event):
        if self.has_episode_picker and event.value != self.video_id and any(v["id"] == event.value for v in self.episodes):
            self.video_id = event.value
            self.reload_streams()

    def reload_streams(self):
        if not self.is_mounted or not self.display:
            return
        self.clear_streams("Cerco i flussi negli addon attivi…")
        self.load()

    @work(exclusive=True, group="streams")
    async def load(self):
        if not self.is_mounted or not self.display:
            return
        status = self.query_one("#streams-status", Static)
        video_id, generation = self.video_id, self.stream_generation
        try:
            items = self.imported if self.imported is not None else await self.app.service.streams(self.media, video_id)
            if (not self.is_mounted or not self.display) or generation != self.stream_generation:
                return
            self.items = items
            self.fill()
            errors = self.app.service.addons.errors
            status.update("Lingua e definizione sono quelle dichiarate dalle fonti."+(" · "+" · ".join(errors[:2]) if errors else "") if self.items else "Nessun flusso. Collega un addon video compatibile da Addon; per serie e anime verifica l’episodio e gli ID supportati.")
        except CineError as e:
            if self.is_mounted and self.display and generation == self.stream_generation:
                status.update(str(e))
        except (ValueError, TypeError, KeyError, AttributeError):
            if self.is_mounted and self.display and generation == self.stream_generation:
                status.update("Una fonte ha restituito dati non validi. Prova un altro flusso o addon.")

    @on(Input.Changed, "#stream-filter")
    @on(Select.Changed, "#stream-language")
    @on(Select.Changed, "#stream-quality")
    def fill(self):
        if not self.is_mounted or not self.display:
            return
        query = self.query_one("#stream-filter", Input).value
        self.workers.cancel_group(self, "play")
        language = str(self.query_one("#stream-language", Select).value)
        quality = str(self.query_one("#stream-quality", Select).value)
        self.app.set_stream_filters(language, quality)
        self.visible_items = [s for s in self.items if matches_stream(s, language=language, quality=quality, query=query)]
        table = self.query_one(DataTable)
        table.clear()
        for i, s in enumerate(self.visible_items):
            info = source_info(s)
            table.add_row(info.quality if info.quality != "unknown" else "—", info.language_label, Text(s.addon), Text(s.name), {"torrent": "Debrid", "external": "Browser"}.get(s.kind, s.kind.upper()), key=str(i))
        self.query_one("#stream-count", Static).update(f"{len(self.visible_items)} / {len(self.items)} FLUSSI"+(" · Nessun flusso corrisponde: cambia lingua o definizione, oppure premi Azzera." if self.items and not self.visible_items else ""))
        self.query_one("#play", Button).disabled = not self.visible_items
        self.query_one("#browser", Button).disabled = not self.visible_items
        self.query_one("#stream-description", Static).update("")
        self.call_after_refresh(self.resize_stream_columns)

    @on(Button.Pressed, "#reset-picker-filters")
    def reset_filters(self):
        self.query_one("#stream-language", Select).value = "all"
        self.query_one("#stream-quality", Select).value = "all"
        self.query_one("#stream-filter", Input).value = ""
        self.fill()

    def selected(self):
        if not self.is_mounted or not self.display:
            return None
        row = self.query_one(DataTable).cursor_row
        return self.visible_items[row] if 0 <= row < len(getattr(self, "visible_items", [])) else None

    @on(DataTable.RowHighlighted)
    def highlight(self):
        if not self.is_mounted or not self.display:
            return
        stream = self.selected()
        if not stream:
            self.query_one("#stream-description", Static).update("")
            return
        info = source_info(stream)
        text = Text()
        text.append(info.quality if info.quality != "unknown" else "Qualità non indicata", style="bold #a8ee78")
        text.append("  ·  "+info.language_label, style="#d8e4ef")
        text.append("  ·  "+stream.addon, style="#91a3b5")
        if stream.filename:
            text.append("\n"+stream.filename, style="#7f90a2")
        if stream.description:
            text.append("\n"+stream.description, style="#c3d0dc")
        self.query_one("#stream-description", Static).update(text)

    @on(DataTable.RowSelected)
    @on(Button.Pressed, "#play")
    @work(exclusive=True, group="play")
    async def play(self):
        stream = self.selected()
        if not stream:
            return
        button = self.query_one("#play", Button)
        status = self.query_one("#streams-status", Static)
        button.disabled = True
        status.update("Preparo la sorgente e avvio il player…")
        try:
            if (not self.is_mounted or not self.display):
                return
            await self.app.service.play(stream, self.media, self.video_id)
            if self.is_mounted and self.display:
                status.update("Player avviato. I controlli sono nella finestra video.")
        except (CineError, OSError) as e:
            if self.is_mounted and self.display:
                status.update(str(e) if isinstance(e, CineError) else "Impossibile avviare il player.")
        finally:
            if self.is_mounted and self.display:
                button.disabled = not bool(self.visible_items)

    @on(Button.Pressed, "#browser")
    def browser(self):
        stream = self.selected()
        if stream and stream.kind == "external":
            webbrowser.open(stream.url)
        elif self.media.url:
            webbrowser.open(self.media.url)

    @on(Button.Pressed, "#retry-streams")
    def retry(self):
        self.reload_streams()

    @on(Button.Pressed, "#manage-addons")
    def manage_addons(self):
        self.app.push_screen(AddonsScreen())

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class ImportScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(classes="dialog", id="import-dialog"):
            yield Static("APRI UN FLUSSO", classes="dialog-title")
            yield Static("Incolla un URL video HTTP/HTTPS, una lista streams JSON o il percorso di un file JSON locale.", classes="dialog-subtitle")
            yield Label("URL o JSON", classes="field-label")
            yield TextArea(id="import-value")
            yield Static("Esempi: https://cdn.example.org/video.mkv · {\"streams\": [{\"name\": \"Video\", \"url\": \"…\"}]} · ./flussi.json",
                         id="import-examples", classes="hint")
            yield Static("", id="import-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Apri", variant="primary", id="import-open")
                yield Button("Incolla appunti", id="paste-import")
                yield Button("Chiudi", id="close")

    @on(Button.Pressed, "#paste-import")
    def paste(self):
        try:
            value = pyperclip.paste()
        except (pyperclip.PyperclipException, OSError, RuntimeError, subprocess.SubprocessError):
            self.query_one("#import-status", Static).update("Gli appunti non sono disponibili: incolla con il terminale.")
            return
        if value:
            self.query_one(TextArea).load_text(value)

    @on(Button.Pressed, "#import-open")
    def submit(self):
        value = self.query_one(TextArea).text.strip()
        try:
            if value.startswith(("https://", "http://")):
                result = [Stream("Flusso diretto", url=web_url(value))]
            else:
                if not value.startswith(("[", "{")):
                    path = Path(value).expanduser()
                    if not path.is_file() or path.stat().st_size > 2*1024*1024:
                        raise CineError("Il file JSON non esiste o supera 2 MB.")
                    value = path.read_text(encoding="utf-8")
                result = parse_streams(json.loads(value), "Importato")
            self.dismiss(result)
        except (CineError, ValueError, OSError) as e:
            self.query_one("#import-status", Static).update(str(e) if isinstance(e, CineError) else "JSON o percorso non valido.")

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss(None)


class CurrentsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(classes="dialog", id="currents-dialog"):
            yield Static("74 CORRENTI CINEMATOGRAFICHE", classes="dialog-title")
            yield Static("Cerca per corrente, autore, opera o paese. Le selezioni restano nel compositore prompt.", classes="dialog-subtitle")
            yield Input(placeholder="Corrente, autore, opera, paese…", id="current-search")
            yield DataTable(id="current-table", cursor_type="row", zebra_stripes=True)
            with VerticalScroll(id="current-detail-scroll"):
                yield Static("", id="current-detail", markup=False)
            yield Static("", id="current-selection", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Seleziona / rimuovi", id="toggle-current")
                yield Button("Componi prompt", variant="primary", id="compose-current")
                yield Button("Chiudi", id="close")

    def on_mount(self):
        table = self.query_one(DataTable)
        for label, width in [("", 2), ("Corrente", 38), ("Periodo", 14), ("Categoria", 24)]:
            table.add_column(label, width=width)
        self.fill()

    @on(Input.Changed, "#current-search")
    def fill(self):
        self.items = search_currents(self.query_one(Input).value)
        table = self.query_one(DataTable)
        table.clear()
        for m in self.items:
            table.add_row(Text("●", style="#a8ee78") if m["id"] in self.app.selected_currents else " ",
                          Text(m["label"]), f"{m['start']}–{m['end'] or 'oggi'}", KINDS[m["kind"]], key=m["id"])
        selected = len(self.app.selected_currents)
        self.query_one("#current-selection", Static).update(
            f"{selected}/4 selezionate"+(" · pronte nel compositore" if selected else " · nessuna corrente selezionata"))

    @on(DataTable.RowHighlighted)
    def detail(self):
        row = self.query_one(DataTable).cursor_row
        if not 0 <= row < len(self.items):
            self.query_one("#current-detail", Static).update(Text("Nessuna corrente corrisponde alla ricerca.", style="#91a3b5"))
            return
        m = self.items[row]
        text = Text()
        text.append(m["label"], style="bold #f2f6fa")
        text.append("  /  "+m["original_name"], style="#91a3b5")
        text.append("\n"+period(m)+"  ·  "+", ".join(m["regions"]), style="#a8ee78")
        text.append("\n\n"+m["summary"], style="#c3d0dc")
        text.append("\n"+m["scope_note"], style="#91a3b5")
        text.append("\n\nCARATTERISTICHE", style="bold #a8ee78")
        text.append("\n"+" · ".join(m["traits"]), style="#c3d0dc")
        text.append("\n\nAUTORI", style="bold #a8ee78")
        text.append("\n"+", ".join(m["authors"]), style="#c3d0dc")
        text.append("\n\nOPERE", style="bold #a8ee78")
        text.append("\n"+" · ".join(f"{w['title']} ({w['year']})" for w in m["works"]), style="#c3d0dc")
        text.append("\n\nFONTI", style="bold #a8ee78")
        for source in m["sources"]:
            text.append("\n"+source["title"]+" — "+source["url"], style="#7f90a2")
        self.query_one("#current-detail", Static).update(text)

    @on(DataTable.RowSelected)
    @on(Button.Pressed, "#toggle-current")
    def toggle(self):
        row = self.query_one(DataTable).cursor_row
        if not 0 <= row < len(self.items):
            return
        mid = self.items[row]["id"]
        chosen = self.app.selected_currents
        if mid in chosen:
            chosen.remove(mid)
        elif len(chosen) < 4:
            chosen.append(mid)
        else:
            self.app.notify("Puoi selezionare al massimo quattro correnti.")
        self.fill()
        self.query_one(DataTable).move_cursor(row=row)

    @on(Button.Pressed, "#compose-current")
    def prompt(self):
        self.dismiss("prompt")

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss(None)


class PromptScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi"), ("ctrl+enter", "generate", "Genera")]

    def compose(self) -> ComposeResult:
        draft = self.app.prompt_draft
        with Vertical(classes="dialog", id="prompt-dialog"):
            yield Static("DA UN’IDEA AL PROSSIMO FILM", classes="dialog-title")
            yield Static("Generazione locale e gratuita: nessuna API AI, nessun messaggio inviato automaticamente.", classes="dialog-subtitle")
            with TabbedContent():
                with TabPane("Componi", id="compose-tab"), VerticalScroll():
                    yield Static("MODALITÀ", classes="section-label")
                    yield Select([(v, k) for k, v in MODES.items()], value=draft.get("mode", "free"), allow_blank=False, id="prompt-mode")
                    yield Static("Correnti: "+(", ".join(m["label"] for m in search_currents() if m["id"] in self.app.selected_currents) or "nessuna · selezionale da Correnti"), classes="prompt-currents", markup=False)
                    yield Static("LA TUA RICHIESTA", classes="section-label")
                    yield TextArea(draft.get("question", ""), id="prompt-question")
                    yield Static("DETTAGLI", classes="section-label")
                    with Horizontal(classes="form-row"):
                        with Vertical(classes="form-col"):
                            yield Label("Quantità", classes="field-label")
                            yield Input(value=draft.get("count", ""), placeholder="Vuota = automatica", id="prompt-count")
                        with Vertical(classes="form-col"):
                            yield Label("Profondità", classes="field-label")
                            yield Select([("Sintetico", "concise"), ("Approfondito", "detailed")], value=draft.get("depth", "concise"), allow_blank=False, id="prompt-depth")
                    yield Label("Relazione con le correnti", classes="field-label")
                    yield Select([("Appartenenza storica", "historical"), ("Anche eredità e influenze", "legacy")], value=draft.get("relation", "historical"), allow_blank=False, id="prompt-relation")
                    yield Static("RIFINITURE", classes="section-label")
                    with Horizontal(classes="form-row"):
                        with Vertical(classes="form-col"):
                            yield Label("Preferenze", classes="field-label")
                            yield Input(value=draft.get("preferences", ""), placeholder="Lingua, durata, atmosfera…", id="prompt-preferences")
                        with Vertical(classes="form-col"):
                            yield Label("Esclusioni", classes="field-label")
                            yield Input(value=draft.get("exclusions", ""), placeholder="Cosa evitare", id="prompt-exclusions")
                    yield Label("Già visti", classes="field-label")
                    yield Input(value=draft.get("watched", ""), placeholder="Titoli separati da punto e virgola", id="prompt-watched")
                with TabPane("Prompt modificabile", id="result-tab"):
                    yield TextArea(draft.get("output", ""), id="prompt-output")
                    yield Label("Apri la chat con", classes="field-label")
                    yield Select([(k, k) for k in PROVIDERS], value="DeepSeek", allow_blank=False, id="chat-provider")
                    with Horizontal(classes="actions"):
                        yield Button("Copia", id="copy-prompt")
                        yield Button("Copia e apri chat", id="open-chat")
                        yield Button("Salva .txt", id="save-prompt")
            yield Static("Generazione locale · nessuna API AI · rileggi e invia tu nella chat.", id="prompt-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Genera prompt", variant="primary", id="generate-prompt")
                yield Button("Chiudi", id="close")

    def remember(self):
        draft = {k: self.query_one("#prompt-"+k, Input).value for k in ["count", "preferences", "exclusions", "watched"]}
        draft.update({k: str(self.query_one("#prompt-"+k, Select).value) for k in ["mode", "depth", "relation"]})
        draft["question"] = self.query_one("#prompt-question", TextArea).text
        draft["output"] = self.query_one("#prompt-output", TextArea).text
        self.app.prompt_draft = draft
        return draft

    @on(Button.Pressed, "#generate-prompt")
    def action_generate(self):
        draft = self.remember()
        try:
            output = generate(mode=draft["mode"], movements=self.app.selected_currents, question=draft["question"],
                              count=int(draft["count"]) if draft["count"].strip() else None, depth=draft["depth"], relation=draft["relation"],
                              preferences=draft["preferences"], exclusions=draft["exclusions"], watched=draft["watched"].split(";"))
            self.query_one("#prompt-output", TextArea).load_text(output)
            self.query_one(TabbedContent).active = "result-tab"
            self.query_one("#prompt-status", Static).update("Prompt pronto. Puoi modificarlo, copiarlo o salvarlo.")
        except (CineError, ValueError) as e:
            self.query_one("#prompt-status", Static).update(str(e) if isinstance(e, CineError) else "La quantità deve essere un intero positivo.")

    @on(Button.Pressed, "#copy-prompt")
    @on(Button.Pressed, "#open-chat")
    @on(Button.Pressed, "#save-prompt")
    def use(self, event):
        text = self.query_one("#prompt-output", TextArea).text
        if not text.strip():
            return
        if event.button.id == "save-prompt":
            path = self.app.service.store.data_path.parent/"prompt.txt"
            try:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
                self.query_one("#prompt-status", Static).update("Salvato in "+str(path))
            except OSError:
                self.query_one("#prompt-status", Static).update("Non posso salvare il file prompt.txt.")
        else:
            copy_text(self.app, text)
            self.query_one("#prompt-status", Static).update("Prompt copiato. Incollalo nella chat e invialo quando vuoi.")
            if event.button.id == "open-chat":
                webbrowser.open(PROVIDERS[str(self.query_one("#chat-provider", Select).value)])

    def on_mount(self):
        self.draft_ready = True

    @on(Input.Changed)
    @on(Select.Changed)
    @on(TextArea.Changed)
    def draft_changed(self):
        if getattr(self, "draft_ready", False):
            self.remember()

    def action_dismiss(self):
        self.remember()
        self.dismiss()

    @on(Button.Pressed, "#close")
    def close(self):
        self.remember()
        self.dismiss()
