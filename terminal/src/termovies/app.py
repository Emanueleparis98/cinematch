from __future__ import annotations

import webbrowser
from typing import ClassVar

from rich.text import Text
from textual import on, work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.widgets import Button, DataTable, Footer, Input, Label, Select, Static

from . import __version__
from .models import CineError, Media, normalized
from .screens import (
    AddonsScreen,
    CurrentsScreen,
    HelpScreen,
    ImportScreen,
    PromptScreen,
    SettingsScreen,
    StreamsScreen,
)
from .stream_filters import LANGUAGES, QUALITIES

SECTIONS = [("Tutti i tipi", "all"), ("Film", "movie"), ("Serie TV", "series"), ("Anime", "anime"), ("Canali TV", "tv"), ("Preferiti", "favorites"), ("Cronologia", "history")]


class CinemaApp(App):
    TITLE = "Termovies"
    SUB_TITLE = "Cerca il titolo. Scegli il flusso. Guarda."
    CSS_PATH = "app.tcss"
    BINDINGS: ClassVar[list] = [Binding("ctrl+q", "quit", "Esci"), Binding("slash", "search_focus", "Cerca"),
                Binding("f1", "help", "Aiuto"), Binding("ctrl+r", "refresh", "Ricarica"),
                Binding("ctrl+a", "addons", "Addon"), Binding("ctrl+p", "prompt", "Prompt"),
                Binding("ctrl+k", "currents", "Correnti"), Binding("ctrl+o", "import", "Apri flusso"),
                Binding("ctrl+s", "settings", "Impostazioni"), Binding("ctrl+x", "stop", "Stop")]

    def __init__(self, service, *, autoload=True):
        super().__init__()
        self.service = service
        self.autoload = autoload
        self.items = []
        self.current = None
        self.selected_currents = []
        self.prompt_draft = {}
        self.skip = 0
        self.episodes_generation = 0
        self.detail_generation = 0
        self.ready = False
        self.initialized = not autoload
        self.empty_title = "Inizia da un titolo"
        self.empty_hint = "Cerca in Kitsu e TMDB, oppure apri i tuoi addon per i cataloghi personali."
        self.show_examples = True
        config = service.store.config
        self.stream_language = config.get("stream_language", "all")
        self.stream_quality = config.get("stream_quality", "all")
        if self.stream_language not in {value for _, value in LANGUAGES}:
            self.stream_language = "all"
        if self.stream_quality not in {value for _, value in QUALITIES}:
            self.stream_quality = "all"

    def compose(self) -> ComposeResult:
        with Horizontal(id="masthead"):
            yield Static("[bold]▰ TERMO[/bold][#a8ee78 bold]VIES[/]", id="brand")
            yield Static("IL TUO CINEMA, DAL TERMINALE", id="tagline")
            with Horizontal(id="status-strip"):
                yield Static("", id="status-tmdb", classes="status-chip")
                yield Static("", id="status-debrid", classes="status-chip")
                yield Static("", id="status-addons", classes="status-chip")
            yield Static(f"v{__version__}", id="edition")
        with Horizontal(id="workspace"):
            with VerticalScroll(id="sidebar"):
                yield Label("ESPLORA", classes="eyebrow")
                yield Label("Contenuto", classes="field-label")
                yield Select(SECTIONS, value="all", allow_blank=False, id="section")
                yield Label("Fonte", classes="field-label")
                yield Select([("Kitsu + TMDB", "all"), ("Addon collegati", "addons"), ("TMDB", "tmdb")], value="all", allow_blank=False, id="source")
                yield Label("Lingua dei flussi", classes="field-label")
                yield Select(LANGUAGES, value=self.stream_language, allow_blank=False, id="language")
                yield Label("Definizione", classes="field-label")
                yield Select([("Tutte" if value == "all" else label, value) for label, value in QUALITIES], value=self.stream_quality, allow_blank=False, id="quality")
                yield Static("I filtri valgono per il prossimo titolo o episodio.", classes="sidebar-note")
                yield Button("Azzera filtri", id="reset-stream-filters", classes="subtle")
                yield Static("", classes="sidebar-divider")
                yield Label("STRUMENTI", classes="eyebrow")
                yield Button("◈  Addon e flussi", id="addons")
                yield Button("◎  Correnti", id="currents")
                yield Button("✎  Componi prompt", id="prompt")
                yield Button("⚙  Impostazioni", id="settings")
                yield Button("↗  Apri URL / JSON", id="import")
                yield Button("?  Guida rapida", id="help")
                yield Static("", id="player-info")
            with Vertical(id="content"):
                yield Label("CERCA FILM, SERIE TV E ANIME", id="section-title", classes="eyebrow")
                with Horizontal(id="search-bar"):
                    yield Input(placeholder="Cerca un film, una serie o un anime…", id="search")
                    yield Button("Cerca", variant="primary", id="search-button")
                    yield Button("Pulisci", id="clear-search", classes="subtle")
                with Horizontal(id="summary"):
                    yield Static("0 RISULTATI", id="result-count")
                    yield Static("", id="playback", markup=False)
                    yield Button("Altri risultati", id="more", disabled=True)
                with Vertical(id="results-area"):
                    yield DataTable(id="results", cursor_type="row", zebra_stripes=True)
                    with Vertical(id="results-empty"):
                        yield Static("▰  TERMOVIES", id="empty-mark")
                        yield Static("", id="empty-title")
                        yield Static("", id="empty-hint")
                        with Horizontal(id="empty-actions"):
                            yield Button("The Matrix", id="example-movie", classes="example")
                            yield Button("Cowboy Bebop", id="example-anime", classes="example")
                            yield Button("Severance", id="example-series", classes="example")
                with Vertical(id="detail"):
                    yield Static("Nessun titolo selezionato", id="media-title")
                    yield Static("", id="media-meta")
                    with VerticalScroll(id="overview-scroll"):
                        yield Static("", id="media-overview")
                    with Horizontal(id="episode-row"):
                        yield Select([("Stagione 1", 1)], value=1, allow_blank=False, id="season")
                        yield Select([("Seleziona un episodio", "")], value="", allow_blank=False, id="episode")
                    with Horizontal(id="detail-actions", classes="actions"):
                        yield Button("Flussi  ↗", variant="primary", id="streams", disabled=True)
                        yield Button("☆ Preferito", id="favorite", disabled=True)
                        yield Button("Apri scheda", id="browser", disabled=True)
                        yield Button("Stop video", id="stop")
                        yield Button("Ispira un prompt", id="inspire")
                yield Static("Pronto · scrivi un titolo e premi Cerca.", id="status", markup=False)
        yield Footer(show_command_palette=False)

    def on_mount(self):
        self.theme = "textual-dark"
        table = self.query_one("#results", DataTable)
        for label, width in [("Titolo", 52), ("Anno", 7), ("Formato", 9), ("Voto", 7), ("Fonte", 16)]:
            table.add_column(label, width=width)
        self.query_one("#episode-row").display = False
        self.query_one("#search").focus()
        self.refresh_status()
        self.show_welcome()
        self.call_after_refresh(self.resize_result_columns)
        self.set_interval(5, self.track_playback)
        self.ready = True

    def on_resize(self, event):
        self.screen_stack[0].set_class(event.size.width < 110, "compact")
        self.screen_stack[0].set_class(event.size.height < 40, "short")
        if self.is_running:
            self.query_one("#more", Button).label = "Altri" if event.size.width < 110 else "Altri risultati"
            self.query_one(Footer).compact = event.size.width < 110
        self.call_after_refresh(self.resize_result_columns)
        self.call_after_refresh(self.update_detail_visibility)
        self.call_after_refresh(self.render_empty)

    def refresh_status(self):
        config = self.service.store.config
        tmdb = bool(config.get("tmdb_key"))
        provider = config.get("debrid_provider") or ""
        debrid = {"rd": "RD", "torbox": "TORBOX"}.get(provider, "DEBRID")
        addons = len([a for a in config.get("addons", []) if isinstance(a, dict) and a.get("enabled", True)])
        player = str(self.service.player.available() or "").replace("\\", "/").rsplit("/", 1)[-1]
        for widget_id, label, active in [
            ("status-tmdb", "TMDB", tmdb),
            ("status-debrid", debrid, bool(provider)),
            ("status-addons", f"{addons} ADDON", addons > 0),
        ]:
            chip = self.query_one("#"+widget_id, Static)
            chip.update(f"[#a8ee78]●[/] {label}" if active else f"[#536474]●[/] {label}")
            chip.set_class(active, "on")
        self.query_one("#edition", Static).update(f"v{__version__}")
        self.query_one("#player-info", Static).update("PLAYER · "+(player.upper() if player else "NON TROVATO"))

    def resize_result_columns(self):
        if not self.is_running:
            return
        table = self.query_one("#results", DataTable)
        if table.size.width <= 0 or len(table.columns) != 5:
            return
        available = table.size.width - 2*table.cell_padding*5 - 2
        widths = (max(28, available - 39), 7, 9, 7, 16)
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

    def set_empty(self, title, hint, examples=False):
        self.empty_title, self.empty_hint, self.show_examples = title, hint, examples
        self.render_empty()

    def show_welcome(self):
        self.set_empty("Inizia da un titolo",
                       "Cerca in Kitsu e TMDB, oppure scegli Addon collegati per i tuoi cataloghi.",
                       examples=True)

    def render_empty(self):
        if not self.is_running:
            return
        empty = self.query_one("#results-empty", Vertical)
        visible = bool(getattr(self, "visible_items", []))
        self.query_one("#results", DataTable).display = visible
        empty.display = not visible
        if not visible:
            self.query_one("#empty-title", Static).update(self.empty_title)
            self.query_one("#empty-hint", Static).update(self.empty_hint)
            short = self.screen_stack[0].has_class("short")
            self.query_one("#empty-actions").display = self.show_examples and not short

    def update_detail_visibility(self):
        if not self.is_running:
            return
        short = self.screen_stack[0].has_class("short")
        self.query_one("#detail").display = bool(self.current) or not short

    @on(Button.Pressed)
    def buttons(self, event):
        bid = event.button.id or ""
        if bid in {"addons", "settings", "prompt", "currents", "import", "help"}:
            getattr(self, "action_"+bid)()

    @on(Button.Pressed, "#clear-search")
    def clear_search(self):
        self.query_one("#search", Input).value = ""
        self.items = []
        self.skip = 0
        self.clear_detail()
        self.show_welcome()
        self.fill()
        self.query_one("#more", Button).disabled = True
        self.query_one("#status", Static).update("Pronto · scrivi un titolo e premi Cerca.")
        self.query_one("#search", Input).focus()

    @on(Button.Pressed, ".example")
    def example_search(self, event):
        section, query = {
            "example-movie": ("movie", "The Matrix"),
            "example-anime": ("anime", "Cowboy Bebop"),
            "example-series": ("series", "Severance"),
        }.get(event.button.id or "", ("all", ""))
        if not query:
            return
        with self.prevent(Select.Changed):
            self.query_one("#section", Select).value = section
            self.query_one("#source", Select).value = "all"
        self.query_one("#search", Input).value = query
        self.search()

    @on(Select.Changed, "#section")
    @on(Select.Changed, "#source")
    def change_section(self, event):
        if not self.ready:
            return
        self.search()

    @on(Select.Changed, "#language")
    @on(Select.Changed, "#quality")
    def stream_filter_changed(self):
        if self.ready:
            self.set_stream_filters(str(self.query_one("#language", Select).value), str(self.query_one("#quality", Select).value))

    def set_stream_filters(self, language, quality):
        if (language, quality) == (self.stream_language, self.stream_quality):
            return
        self.stream_language, self.stream_quality = language, quality
        self.query_one("#language", Select).value = language
        self.query_one("#quality", Select).value = quality
        self.service.store.config.update(stream_language=language, stream_quality=quality)
        try:
            self.service.store.save_config()
        except OSError:
            self.notify("Filtri applicati per questa sessione; non riesco a salvarli.", severity="warning")

    @on(Button.Pressed, "#reset-stream-filters")
    def reset_stream_filters(self):
        self.set_stream_filters("all", "all")

    @on(Input.Submitted, "#search")
    @on(Button.Pressed, "#search-button")
    def search(self):
        self.skip = 0
        self.load_results()

    @on(Button.Pressed, "#more")
    def more(self):
        self.skip += 1
        self.load_results(append=True)

    @work(exclusive=True, group="search")
    async def load_results(self, append=False):
        if not self.is_running:
            return
        query = self.query_one("#search", Input).value.strip()
        section = str(self.query_one("#section", Select).value)
        source = str(self.query_one("#source", Select).value)
        status = self.query_one("#status", Static)
        if not query and section not in {"favorites", "history"}:
            self.items = []
            self.clear_detail()
            self.show_welcome()
            self.fill()
            self.query_one("#more", Button).disabled = True
            status.update("Pronto · scrivi un titolo e premi Cerca.")
            return
        status.update("Cerco i titoli nelle fonti selezionate…")
        self.set_empty("Cerco tra le fonti…", "Interrogo Kitsu, TMDB e gli addon selezionati.")
        table = self.query_one("#results", DataTable)
        table.loading = True
        try:
            if section in {"favorites", "history"}:
                items = self.service.store.library(history=section == "history")
                items = [m for m in items if normalized(query) in normalized(m.title)]
            else:
                if not self.initialized and source in {"all", "addons"}:
                    await self.service.addons.initialize()
                    self.initialized = True
                items = await self.service.search(query, section, source, self.skip)
            if not self.is_running:
                return
            if append:
                existing = {m.key for m in self.items}
                self.items += [m for m in items if m.key not in existing]
            else:
                self.items = items
                self.clear_detail()
            self.fill()
            self.query_one("#more", Button).disabled = section in {"favorites", "history"} or not items
            errors = " · ".join(self.service.errors[:2])
            if not self.items and section == "tv":
                self.set_empty("Nessun canale trovato", "Collega un addon TV da Addon e flussi per vedere i canali.")
                status.update("Nessun canale trovato · collega un addon TV da Addon e flussi.")
            elif not self.items and source == "tmdb" and not self.service.store.config["tmdb_key"]:
                self.set_empty("TMDB non è collegato", "Apri Impostazioni e inserisci una chiave API v3 o un Read Access Token.")
                status.update("Collega TMDB nelle impostazioni per cercare film e serie; Kitsu copre gli anime.")
            elif not self.items and section == "favorites":
                self.set_empty("Nessun preferito", "Apri una scheda e premi ☆ Preferito per ritrovarla qui.")
                status.update("Nessun preferito salvato.")
            elif not self.items and section == "history":
                self.set_empty("Cronologia vuota", "I titoli che apri o guardi compariranno qui.")
                status.update("Nessuna attività nella cronologia.")
            elif not self.items:
                self.set_empty("Nessun risultato", "Prova un altro titolo, cambia sezione o scegli Addon collegati.")
                status.update("Nessun risultato"+((" · "+errors) if errors else " · prova un altro titolo o cambia fonte."))
            else:
                status.update(f"{len(self.items)} titoli"+(" · "+errors if errors else " · seleziona un titolo per dettagli ed episodi"))
        except (CineError, OSError, ValueError, TypeError) as e:
            if self.is_running:
                message = str(e) if isinstance(e, CineError) else "Non riesco a leggere i dati locali."
                self.set_empty("Ricerca non disponibile", message)
                status.update(message)
        finally:
            if self.is_running:
                table.loading = False

    def clear_detail(self):
        if not self.is_running:
            return
        self.current = None
        self.detail_generation += 1
        self.episodes_generation += 1
        self.workers.cancel_group(self, "detail")
        self.workers.cancel_group(self, "episodes")
        self.query_one("#episode-row").display = False
        self.query_one("#media-title", Static).update(Text("Nessun titolo selezionato", style="bold #7f90a2"))
        self.query_one("#media-meta", Static).update("Scegli un risultato per leggere trama, cast e azioni.")
        self.query_one("#media-overview", Static).update("")
        self.query_one("#favorite", Button).label = "☆ Preferito"
        self.query_one("#detail").set_class(False, "selected")
        self.update_detail_visibility()
        for bid in ("streams", "favorite", "browser"):
            self.query_one("#"+bid, Button).disabled = True

    def fill(self):
        if not self.is_running:
            return
        self.visible_items = self.items[:]
        table = self.query_one("#results", DataTable)
        table.clear()
        for i, m in enumerate(self.visible_items):
            kind = {"movie": "Film", "series": "Serie", "anime": "Anime", "tv": "TV", "channel": "Canale"}.get(m.type, m.type)
            table.add_row(Text(m.title), Text(m.year), Text(kind), Text(("★ "+m.rating) if m.rating else "—"), Text(m.source), key=str(i))
        self.query_one("#result-count", Static).update(f"{len(self.visible_items)} RISULTATI")
        if not self.visible_items:
            self.clear_detail()
        self.render_empty()
        self.call_after_refresh(self.resize_result_columns)

    @on(DataTable.RowHighlighted, "#results")
    def highlight(self, event: DataTable.RowHighlighted):
        # Use the message payload rather than querying the DOM: a highlight can
        # already be queued while the app is being torn down in tests.
        row = event.cursor_row
        if self.is_running and 0 <= row < len(getattr(self, "visible_items", [])):
            self.detail_generation += 1
            self.show_detail(self.visible_items[row], self.detail_generation)

    @work(exclusive=True, group="detail")
    async def show_detail(self, media, generation):
        if not self.is_running:
            return
        self.current = media
        self.query_one("#detail").display = True
        for bid in ("favorite", "browser"):
            self.query_one("#"+bid, Button).disabled = False
        self.query_one("#detail").set_class(True, "selected")
        self.query_one("#media-title", Static).update(Text(media.title, style="bold #f2f6fa"))
        self.query_one("#media-meta", Static).update(self.detail_meta(media))
        self.query_one("#media-overview", Static).update(self.detail_overview(media))
        self.update_favorite_button(media)
        self.query_one("#episode-row").display = False
        self.query_one("#streams", Button).disabled = True
        try:
            detail = await self.service.details(media)
            if generation != self.detail_generation or not self.is_running:
                return
            detail.resume_video_id = media.resume_video_id
            self.current = detail
            self.query_one("#media-title", Static).update(Text(detail.title, style="bold #f2f6fa"))
            self.query_one("#media-meta", Static).update(self.detail_meta(detail))
            self.query_one("#media-overview", Static).update(self.detail_overview(detail))
            self.update_favorite_button(detail)
            series = detail.type in {"series", "anime"}
            self.query_one("#episode-row").display = series
            if series:
                seasons = detail.available_seasons
                select = self.query_one("#season", Select)
                with self.prevent(Select.Changed):
                    select.set_options([(f"Stagione {s}" if s else "Speciali", s) for s in seasons])
                    resume = detail.season_for_video(detail.resume_video_id)
                    chosen = resume if resume in seasons else seasons[0]
                    select.value = chosen
                self.episodes_generation += 1
                self.load_episodes(detail, chosen, self.episodes_generation)
        except (CineError, ValueError, TypeError) as e:
            if generation == self.detail_generation and self.is_running:
                self.query_one("#media-meta", Static).update(str(e) if isinstance(e, CineError) else "Scheda parziale: alcuni metadati non sono disponibili.")
        finally:
            if generation == self.detail_generation and self.is_running:
                self.query_one("#streams", Button).disabled = False

    def detail_meta(self, media):
        text = Text()
        if media.year:
            text.append(media.year, style="bold #d8e4ef")
            text.append("  ")
        kind = {"movie": "FILM", "series": "SERIE", "anime": "ANIME", "tv": "TV", "channel": "CANALE"}.get(media.type, media.type.upper())
        text.append(f" {kind} ", style="bold #142012 on #a8ee78")
        if media.rating:
            text.append(f"  ★ {media.rating}", style="bold #a8ee78")
        if media.genres:
            text.append("  ·  "+" / ".join(media.genres[:4]), style="#91a3b5")
        if media.directors:
            text.append("\nRegia: "+", ".join(media.directors[:3]), style="#7f90a2")
        return text

    def detail_overview(self, media):
        text = Text(media.overview or "Nessuna trama disponibile per questo titolo.", style="#c3d0dc")
        if media.cast:
            text.append("\n\nCast: "+", ".join(media.cast[:8]), style="#8fa1b3")
        return text

    def is_favorite(self, media):
        return any(item.get("id") == media.id and item.get("type") == media.type
                   for item in self.service.store.data.get("favorites", []) if isinstance(item, dict))

    def update_favorite_button(self, media):
        active = self.is_favorite(media)
        button = self.query_one("#favorite", Button)
        button.label = "★ Nei preferiti" if active else "☆ Preferito"
        button.set_class(active, "active")

    @on(Select.Changed, "#season")
    def change_season(self, event):
        if self.current and isinstance(event.value, int):
            self.episodes_generation += 1
            self.load_episodes(self.current, event.value, self.episodes_generation)

    def set_episode_options(self, media, episodes):
        if not self.is_running or self.current is not media:
            return
        select = self.query_one("#episode", Select)
        select.set_options([(f"{v.get('episode', i+1):02} · {plain_title(v)}", v["id"]) for i, v in enumerate(episodes)] or [("Nessun episodio disponibile", "")])
        select.value = media.resume_video_id if any(v["id"] == media.resume_video_id for v in episodes) else episodes[0]["id"] if episodes else ""

    @work(exclusive=True, group="episodes")
    async def load_episodes(self, media, season, generation):
        if not self.is_running:
            return
        try:
            episodes = await self.service.episodes(media, season)
            if generation != self.episodes_generation or self.current is not media or not self.is_running:
                return
            self.set_episode_options(media, episodes)
        except (CineError, ValueError, TypeError):
            if generation == self.episodes_generation and self.is_running:
                self.query_one("#status", Static).update("Non riesco a leggere gli episodi. Riprova o scegli un’altra fonte.")

    @on(DataTable.RowSelected, "#results")
    @on(Button.Pressed, "#streams")
    def streams(self):
        if not self.current or self.query_one("#streams", Button).disabled:
            return
        series = self.current.type in {"series", "anime"}
        episode = str(self.query_one("#episode", Select).value) if series else ""
        if series and not episode:
            self.notify("Seleziona un episodio disponibile.")
            return
        self.push_screen(StreamsScreen(self.current, episode))

    @on(Button.Pressed, "#favorite")
    def favorite(self):
        if not self.current:
            return
        try:
            added = self.service.store.favorite(self.current)
        except OSError:
            self.notify("Non posso salvare i preferiti.", severity="error")
            return
        self.update_favorite_button(self.current)
        self.notify("Aggiunto ai preferiti" if added else "Rimosso dai preferiti",
                    title="Termovies", severity="information" if added else "warning")

    @on(Button.Pressed, "#browser")
    def browser(self):
        if self.current:
            url = self.current.url or ("https://www.imdb.com/title/"+self.current.imdb+"/" if self.current.imdb else "")
            if url:
                webbrowser.open(url)

    @on(Button.Pressed, "#inspire")
    def inspire(self):
        if self.current:
            self.prompt_draft.update(mode="free", question=f"Consigliami opere affini a {self.current.title} ({self.current.year}), spiegando i collegamenti e senza riproporre questo titolo.")
        self.action_prompt()

    @work(exclusive=True, group="progress")
    async def track_playback(self):
        if not self.is_running:
            return
        current = self.service.current
        if not current:
            return
        try:
            await self.service.save_progress()
        except OSError:
            return
        position = int(self.service.player.position)
        active = self.service.player.process and self.service.player.process.returncode is None
        self.query_one("#playback", Static).update(("● " if active else "○ ")+current.title+f" · {position//60}:{position%60:02}")

    @on(Button.Pressed, "#stop")
    @work(exclusive=True, group="stop")
    async def stop(self):
        try:
            await self.service.save_progress()
        finally:
            await self.service.player.stop()
            self.service.current = None
            self.service.video_id = ""
            if self.is_running:
                self.query_one("#playback", Static).update("Video interrotto")

    def action_stop(self):
        self.stop()

    def action_search_focus(self):
        self.query_one("#search").focus()

    def action_refresh(self):
        if self.query_one("#search", Input).value.strip():
            self.search()
        else:
            self.query_one("#search", Input).focus()

    def action_help(self):
        self.push_screen(HelpScreen())

    def action_settings(self):
        self.push_screen(SettingsScreen(), lambda _: self.refresh_status())

    def action_addons(self):
        self.push_screen(AddonsScreen(), lambda _: self.refresh_status())

    def action_prompt(self):
        self.push_screen(PromptScreen())

    def action_currents(self):
        self.push_screen(CurrentsScreen(), lambda result: self.action_prompt() if result == "prompt" else None)

    def action_import(self):
        def opened(streams):
            if streams is not None:
                self.push_screen(StreamsScreen(self.current or Media("import", "movie", "Flusso importato"), imported=streams))
        self.push_screen(ImportScreen(), opened)


def plain_title(video):
    return str(video.get("title") or video.get("name") or "Episodio")[:150]
