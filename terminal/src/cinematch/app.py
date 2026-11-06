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
    ImportScreen,
    PromptScreen,
    SettingsScreen,
    StreamsScreen,
)

SECTIONS = [("Scopri", "all"), ("Film", "movie"), ("Serie", "series"), ("Anime", "anime"), ("TV", "tv"), ("Preferiti", "favorites"), ("Cronologia", "history")]


class CinemaApp(App):
    TITLE = "CineMatch"
    SUB_TITLE = "La prossima storia comincia qui"
    CSS_PATH = "app.tcss"
    BINDINGS: ClassVar[list] = [Binding("ctrl+q", "quit", "Esci"), Binding("slash", "search_focus", "Cerca"),
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

    def compose(self) -> ComposeResult:
        with Horizontal(id="masthead"):
            yield Static("[bold]CINE[/bold][#f3be71 bold]MATCH[/]", id="brand")
            yield Static("LA PROSSIMA STORIA COMINCIA QUI", id="tagline")
            yield Static(f"TERMINAL / {__version__}", id="edition")
        with Horizontal(id="workspace"):
            with VerticalScroll(id="sidebar"):
                yield Label("LA TUA SALA", classes="eyebrow")
                for title, section in SECTIONS:
                    yield Button(title, id="nav-"+section, classes="nav active" if section == "all" else "nav")
                yield Label("ISPIRAZIONE", classes="eyebrow")
                yield Button("Correnti  ↗", id="currents", classes="nav")
                yield Button("Componi prompt", id="prompt", classes="nav")
                yield Label("IL TUO SETUP", classes="eyebrow")
                yield Button("Addon", id="addons", classes="nav")
                yield Button("Impostazioni", id="settings", classes="nav")
                yield Button("Apri URL / JSON", id="import", classes="nav")
                yield Static("Ricerca. Esplora.\nTrova la tua prossima storia.", classes="sidebar-note")
            with Vertical(id="content"):
                yield Label("COSA GUARDIAMO OGGI?", id="section-title", classes="eyebrow")
                with Horizontal(id="search-bar"):
                    yield Input(placeholder="Film, serie, anime o canale…", id="search")
                    yield Button("Cerca", variant="primary", id="search-button")
                with Horizontal(id="filters"):
                    yield Select(SECTIONS, value="all", allow_blank=False, id="section")
                    yield Select([("Cataloghi + TMDB", "all"), ("Solo addon", "addons"), ("TMDB", "tmdb"), ("CB01", "cb01")], value="all", allow_blank=False, id="source")
                    yield Input(placeholder="Filtra i risultati…", id="local-filter")
                yield Static("Collego i cataloghi…" if self.autoload else "Pronto · configura i tuoi addon per i flussi", id="status", markup=False)
                yield DataTable(id="results", cursor_type="row", zebra_stripes=True)
                with Horizontal(id="list-actions"):
                    yield Button("Altri risultati", id="more")
                    yield Static("", id="playback", markup=False)
                    yield Button("Stop video", id="stop")
                with Vertical(id="detail"):
                    yield Static("Scegli la tua prossima storia", id="media-title", markup=False)
                    yield Static("Cataloghi pubblici per film e serie. Addon personali per le sorgenti video. Le 74 correnti e i prompt funzionano anche offline.", id="media-meta", markup=False)
                    with VerticalScroll(id="overview-scroll"):
                        yield Static("", id="media-overview", markup=False)
                    with Horizontal(id="episode-row"):
                        yield Select([("Stagione 1", 1)], value=1, allow_blank=False, id="season")
                        yield Select([("Seleziona un episodio", "")], value="", allow_blank=False, id="episode")
                    with Horizontal(classes="actions"):
                        yield Button("Flussi  ↗", variant="primary", id="streams")
                        yield Button("☆ Preferito", id="favorite")
                        yield Button("Apri scheda", id="browser")
                        yield Button("Ispira un prompt", id="inspire")
        yield Footer()

    def on_mount(self):
        self.query_one("#results", DataTable).add_columns("Titolo", "Anno", "Formato", "Catalogo")
        self.query_one("#episode-row").display = False
        self.query_one("#search").focus()
        self.set_interval(5, self.track_playback)
        if self.autoload:
            self.initialize()
        else:
            self.ready = True

    def on_resize(self, event):
        self.screen_stack[0].set_class(event.size.width < 110, "compact")
        self.screen_stack[0].set_class(event.size.height < 40, "short")

    @work(exclusive=True, group="initialize")
    async def initialize(self):
        await self.service.addons.initialize()
        self.ready = True
        self.search()

    @on(Button.Pressed)
    def buttons(self, event):
        bid = event.button.id or ""
        if bid.startswith("nav-"):
            self.query_one("#section", Select).value = bid.removeprefix("nav-")
        elif bid in {"addons", "settings", "prompt", "currents", "import"}:
            getattr(self, "action_"+bid)()

    @on(Select.Changed, "#section")
    @on(Select.Changed, "#source")
    def change_section(self, event):
        if not self.ready:
            return
        if event.select.id == "section":
            value = event.value
            for _, key in SECTIONS:
                self.query_one("#nav-"+key).set_class(value == key, "active")
        self.search()

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
        query = self.query_one("#search", Input).value.strip()
        section = str(self.query_one("#section", Select).value)
        source = str(self.query_one("#source", Select).value)
        status = self.query_one("#status", Static)
        status.update("Cerco nei cataloghi…")
        try:
            if section in {"favorites", "history"}:
                items = self.service.store.library(history=section == "history")
                items = [m for m in items if normalized(query) in normalized(m.title)]
            else:
                items = await self.service.search(query, section, source, self.skip)
            if append:
                existing = {m.key for m in self.items}
                self.items += [m for m in items if m.key not in existing]
            else:
                self.items = items
                self.current = None
                self.detail_generation += 1
                self.query_one("#episode-row").display = False
            self.fill()
            self.query_one("#more", Button).disabled = section in {"favorites", "history"} or not items
            errors = " · ".join(self.service.errors[:2])
            if not self.items and section == "tv":
                status.update("Aggiungi un addon con catalogo TV/channel per cercare e vedere i tuoi canali.")
            elif not self.items and source == "tmdb" and not self.service.store.config["tmdb_key"]:
                status.update("Collega TMDB nelle impostazioni oppure scegli Cataloghi + TMDB.")
            else:
                status.update(f"{len(self.items)} titoli"+(" · "+errors if errors else " · seleziona un titolo per dettagli ed episodi"))
        except (CineError, OSError) as e:
            status.update(str(e) if isinstance(e, CineError) else "Non riesco a leggere i dati locali.")

    @on(Input.Changed, "#local-filter")
    def fill(self):
        query = normalized(self.query_one("#local-filter", Input).value)
        self.visible_items = [m for m in self.items if query in normalized(m.title+" "+m.year+" "+m.source)]
        table = self.query_one("#results", DataTable)
        table.clear()
        for i, m in enumerate(self.visible_items):
            table.add_row(Text(m.title), Text(m.year), {"movie": "Film", "series": "Serie", "anime": "Anime", "tv": "TV", "channel": "Canale"}.get(m.type, m.type), Text(m.source), key=str(i))

    @on(DataTable.RowHighlighted, "#results")
    def highlight(self):
        row = self.query_one("#results", DataTable).cursor_row
        if 0 <= row < len(getattr(self, "visible_items", [])):
            self.detail_generation += 1
            self.show_detail(self.visible_items[row], self.detail_generation)

    @work(exclusive=True, group="detail")
    async def show_detail(self, media, generation):
        self.current = media
        self.query_one("#media-title", Static).update(media.title)
        self.query_one("#media-overview", Static).update(media.overview)
        self.query_one("#episode-row").display = False
        self.query_one("#streams", Button).disabled = True
        try:
            detail = await self.service.details(media)
            if generation != self.detail_generation:
                return
            detail.resume_video_id = media.resume_video_id
            self.current = detail
            self.query_one("#media-title", Static).update(detail.title)
            meta = [detail.year, " / ".join(detail.genres), "★ "+detail.rating if detail.rating else "", "Regia: "+", ".join(detail.directors) if detail.directors else ""]
            self.query_one("#media-meta", Static).update(" · ".join(v for v in meta if v))
            self.query_one("#media-overview", Static).update(detail.overview+("\n\nCast: "+", ".join(detail.cast) if detail.cast else ""))
            series = detail.type in {"series", "anime"}
            self.query_one("#episode-row").display = series
            if series:
                seasons = detail.seasons or sorted({int(v.get("season") or 1) for v in detail.videos}) or [1]
                select = self.query_one("#season", Select)
                select.set_options([(f"Stagione {s}" if s else "Speciali", s) for s in seasons])
                resume = next((v.get("season") or 1 for v in detail.videos if v["id"] == detail.resume_video_id), None)
                if resume is None and detail.resume_video_id:
                    parts = detail.resume_video_id.rsplit(":", 2)
                    resume = int(parts[-2]) if len(parts) == 3 and parts[-2].isdigit() else None
                chosen = resume if resume in seasons else seasons[0]
                select.value = chosen
                self.load_episodes(detail, chosen)
        except (CineError, ValueError, TypeError) as e:
            if generation == self.detail_generation:
                self.query_one("#media-meta", Static).update(str(e) if isinstance(e, CineError) else "Scheda parziale: alcuni metadati non sono disponibili.")
        finally:
            if generation == self.detail_generation:
                self.query_one("#streams", Button).disabled = False

    @on(Select.Changed, "#season")
    def change_season(self, event):
        if self.current and isinstance(event.value, int):
            self.load_episodes(self.current, event.value)

    @work(exclusive=True, group="episodes")
    async def load_episodes(self, media, season):
        try:
            episodes = await self.service.episodes(media, season)
            if self.current is not media:
                return
            select = self.query_one("#episode", Select)
            select.set_options([(f"{v.get('episode', i+1):02} · {plain_title(v)}", v["id"]) for i, v in enumerate(episodes)] or [("Nessun episodio disponibile", "")])
            select.value = media.resume_video_id if any(v["id"] == media.resume_video_id for v in episodes) else episodes[0]["id"] if episodes else ""
            if not episodes and media.source == "CB01":
                self.query_one("#status", Static).update("La pagina non espone singoli episodi leggibili. Usa Apri scheda per verificare l’eventuale cartella della serie.")
        except (CineError, ValueError, TypeError):
            self.query_one("#status", Static).update("Non riesco a leggere gli episodi. Riprova o scegli un altro catalogo.")

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
        if self.current:
            added = self.service.store.favorite(self.current)
            self.notify("Aggiunto ai preferiti" if added else "Rimosso dai preferiti")

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
        if self.service.current:
            await self.service.save_progress()
            position = int(self.service.player.position)
            active = self.service.player.process and self.service.player.process.returncode is None
            self.query_one("#playback", Static).update(("● " if active else "○ ")+self.service.current.title+f" · {position//60}:{position%60:02}")

    @on(Button.Pressed, "#stop")
    @work(exclusive=True, group="stop")
    async def stop(self):
        await self.service.save_progress()
        await self.service.player.stop()
        self.query_one("#playback", Static).update("Video interrotto")

    def action_stop(self):
        self.stop()

    def action_search_focus(self):
        self.query_one("#search").focus()

    def action_settings(self):
        self.push_screen(SettingsScreen())

    def action_addons(self):
        self.push_screen(AddonsScreen())

    def action_prompt(self):
        self.push_screen(PromptScreen())

    def action_currents(self):
        self.push_screen(CurrentsScreen(), lambda result: self.action_prompt() if result == "prompt" else None)

    def action_import(self):
        def opened(streams):
            if streams:
                self.push_screen(StreamsScreen(self.current or Media("import", "movie", "Flusso importato"), imported=streams))
        self.push_screen(ImportScreen(), opened)


def plain_title(video):
    return str(video.get("title") or video.get("name") or "Episodio")[:150]
