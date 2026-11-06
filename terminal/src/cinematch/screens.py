from __future__ import annotations

import json
import subprocess
import webbrowser
from pathlib import Path
from typing import ClassVar

import pyperclip
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
from .models import CineError, Stream, web_url
from .prompts import KINDS, MODES, PROVIDERS, generate, period, search_currents
from .stream_filters import LANGUAGES, QUALITIES, matches_stream, source_info


def copy_text(app, value):
    try:
        pyperclip.copy(value)
    except (pyperclip.PyperclipException, OSError, RuntimeError, subprocess.SubprocessError):
        app.copy_to_clipboard(value)


class DialogScreen(ModalScreen):
    def on_resize(self, event):
        self.set_class(event.size.height < 34, "small")


class SettingsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        config = self.app.service.store.config
        with Vertical(id="settings-dialog", classes="dialog"):
            yield Static("IL TUO CINEMATCH", classes="dialog-title")
            with VerticalScroll():
                yield Label("TMDB · API v3 oppure Read Access Token")
                yield Input(value=config["tmdb_key"], password=True, id="tmdb-key", placeholder="Facoltativo: migliora ricerca e metadati in italiano")
                yield Label("Servizio debrid · una chiave per tutti gli addon")
                yield Select([("Nessuno", ""), ("Real-Debrid", "rd"), ("TorBox", "torbox")], value=config["debrid_provider"], allow_blank=False, id="debrid-provider")
                yield Input(value=config["debrid_key"], password=True, id="debrid-key", placeholder="Chiave personale del servizio")
                yield Label("Player video")
                yield Select([("Automatico · mpv, VLC, ffplay", "auto"), ("mpv · consigliato", "mpv"), ("VLC", "vlc"), ("ffplay", "ffplay")], value=config.get("player", "auto"), allow_blank=False, id="player-choice")
                yield Static("Le chiavi restano nel file di configurazione locale e non sono cifrate. Su Linux il file è accessibile solo al tuo utente. La chiave debrid viene inviata solo al servizio scelto.", classes="muted")
                yield Static(Text(str(self.app.service.store.config_path)), classes="muted")
            yield Static("", id="settings-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Salva", variant="primary", id="save-settings")
                yield Button("Verifica", id="verify-settings")
                yield Button("Chiudi", id="close")

    def save(self):
        config = self.app.service.store.config
        config.update(tmdb_key=self.query_one("#tmdb-key", Input).value.strip(),
                      debrid_provider=str(self.query_one("#debrid-provider", Select).value),
                      debrid_key=self.query_one("#debrid-key", Input).value.strip(),
                      player=str(self.query_one("#player-choice", Select).value))
        self.app.service.store.save_config()

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
            status.update(" · ".join(messages) or "Nessun servizio da verificare. I cataloghi pubblici funzionano senza chiavi.")
        except (CineError, OSError) as e:
            status.update(str(e) if isinstance(e, CineError) else "Non posso salvare le impostazioni.")

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class AddonsScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(id="addons-dialog", classes="dialog"):
            yield Static("LE TUE FONTI", classes="dialog-title")
            yield Static("Aggiungi un manifest Stremio compatibile. Le configurazioni specifiche dell’addon restano nel suo link.", classes="muted")
            yield Input(placeholder="https://…/manifest.json oppure stremio://…", password=True, id="addon-url")
            yield Button("Aggiungi addon", variant="primary", id="add-addon")
            yield DataTable(id="addon-table", cursor_type="row", zebra_stripes=True)
            yield Static("", id="addon-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Attiva / disattiva", id="toggle-addon")
                yield Button("Rimuovi", id="remove-addon")
                yield Button("Chiudi", id="close")
            yield Static("Per vedere i flussi collega i tuoi addon. Nessun addon video viene installato automaticamente.", classes="muted")

    def on_mount(self):
        self.query_one(DataTable).add_columns("Addon", "Stato", "Tipi", "Cataloghi")
        self.fill()

    def fill(self):
        table = self.query_one(DataTable)
        table.clear()
        for i, a in enumerate(self.app.service.store.config["addons"]):
            table.add_row(Text(a["name"]), "Attivo" if a.get("enabled", True) else "Spento", Text(", ".join(a["types"])), str(len(a.get("catalogs", []))), key=str(i))

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
            self.query_one("#addon-url", Input).value = ""
            self.fill()
            status.update(addon["name"]+" aggiunto. La chiave debrid globale sarà usata per i torrent restituiti dall’addon.")
        except (CineError, OSError, ValueError, TypeError):
            status.update("Impossibile aggiungere l’addon. Verifica manifest, configurazione e collegamento al sito.")
        finally:
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

    def compose(self) -> ComposeResult:
        with Vertical(id="streams-dialog", classes="dialog"):
            yield Static(Text(self.media.title), classes="dialog-title")
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
                yield Button("Riproduci", variant="primary", id="play")
                yield Button("Apri fonte", id="browser")
                yield Button("Chiudi", id="close")

    def on_mount(self):
        self.query_one(DataTable).add_columns("Definizione", "Lingua", "Fonte", "Nome", "Tipo")
        self.load()

    @work(exclusive=True, group="streams")
    async def load(self):
        status = self.query_one("#streams-status", Static)
        try:
            self.items = self.imported if self.imported is not None else await self.app.service.streams(self.media, self.video_id)
            self.fill()
            errors = self.app.service.addons.errors
            status.update("Lingua e definizione sono quelle dichiarate dalle fonti."+(" · "+" · ".join(errors[:2]) if errors else "") if self.items else "Nessun flusso. Collega un addon video compatibile da Addon; per serie e anime verifica l’episodio e gli ID supportati.")
        except CineError as e:
            status.update(str(e))

    @on(Input.Changed, "#stream-filter")
    @on(Select.Changed, "#stream-language")
    @on(Select.Changed, "#stream-quality")
    def fill(self):
        query = self.query_one("#stream-filter", Input).value
        language = str(self.query_one("#stream-language", Select).value)
        quality = str(self.query_one("#stream-quality", Select).value)
        self.app.set_stream_filters(language, quality)
        self.visible_items = [s for s in self.items if matches_stream(s, language=language, quality=quality, query=query)]
        table = self.query_one(DataTable)
        table.clear()
        for i, s in enumerate(self.visible_items):
            info = source_info(s)
            table.add_row(info.quality if info.quality != "unknown" else "—", info.language_label, Text(s.addon), Text(s.name), {"torrent": "Debrid", "external": "Browser", "page": "Player web"}.get(s.kind, s.kind.upper()), key=str(i))
        self.query_one("#stream-count", Static).update(f"{len(self.visible_items)} / {len(self.items)} FLUSSI"+(" · Nessun flusso corrisponde: cambia lingua o definizione, oppure premi Azzera." if self.items and not self.visible_items else ""))
        self.query_one("#play", Button).disabled = not self.visible_items
        self.query_one("#stream-description", Static).update("")

    @on(Button.Pressed, "#reset-picker-filters")
    def reset_filters(self):
        self.query_one("#stream-language", Select).value = "all"
        self.query_one("#stream-quality", Select).value = "all"
        self.query_one("#stream-filter", Input).value = ""
        self.fill()

    def selected(self):
        row = self.query_one(DataTable).cursor_row
        return self.visible_items[row] if 0 <= row < len(getattr(self, "visible_items", [])) else None

    @on(DataTable.RowHighlighted)
    def highlight(self):
        stream = self.selected()
        self.query_one("#stream-description", Static).update(stream.description if stream else "")

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
            await self.app.service.play(stream, self.media, self.video_id)
            status.update("Player avviato. Audio, sottotitoli e schermo intero si controllano dalla finestra video.")
        except (CineError, OSError) as e:
            status.update(str(e) if isinstance(e, CineError) else "Impossibile avviare il player.")
        finally:
            button.disabled = False

    @on(Button.Pressed, "#browser")
    def browser(self):
        stream = self.selected()
        if stream and stream.kind in {"page", "external"}:
            webbrowser.open(stream.url)
        elif self.media.url:
            webbrowser.open(self.media.url)

    @on(Button.Pressed, "#close")
    def close(self):
        self.dismiss()


class ImportScreen(DialogScreen):
    BINDINGS: ClassVar[list] = [("escape", "dismiss", "Chiudi")]

    def compose(self) -> ComposeResult:
        with Vertical(classes="dialog", id="import-dialog"):
            yield Static("APRI UN FLUSSO", classes="dialog-title")
            yield Static("Incolla un URL video HTTP/HTTPS, una lista streams JSON oppure il percorso di un file JSON locale.", classes="muted")
            yield TextArea(id="import-value")
            yield Static("", id="import-status", markup=False)
            with Horizontal(classes="actions"):
                yield Button("Apri", variant="primary", id="import-open")
                yield Button("Chiudi", id="close")

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
            yield Static("74 MODI DI GUARDARE IL CINEMA", classes="dialog-title")
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
        self.query_one(DataTable).add_columns("", "Corrente", "Periodo", "Categoria")
        self.fill()

    @on(Input.Changed, "#current-search")
    def fill(self):
        self.items = search_currents(self.query_one(Input).value)
        table = self.query_one(DataTable)
        table.clear()
        for m in self.items:
            table.add_row("●" if m["id"] in self.app.selected_currents else "", Text(m["label"]), f"{m['start']}–{m['end'] or 'oggi'}", KINDS[m["kind"]], key=m["id"])
        self.query_one("#current-selection", Static).update(f"{len(self.app.selected_currents)}/4 selezionate · le selezioni restano disponibili nel compositore")

    @on(DataTable.RowHighlighted)
    def detail(self):
        row = self.query_one(DataTable).cursor_row
        if not 0 <= row < len(self.items):
            return
        m = self.items[row]
        self.query_one("#current-detail", Static).update(f"{m['label']} / {m['original_name']}\n{period(m)} · {', '.join(m['regions'])}\n\n{m['summary']}\n{m['scope_note']}\n\nCaratteristiche: {'; '.join(m['traits'])}\nAutori: {', '.join(m['authors'])}\nOpere: {'; '.join(w['title']+' ('+str(w['year'])+')' for w in m['works'])}\n\nFonti:\n"+"\n".join(s["title"]+" — "+s["url"] for s in m["sources"]))

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
            with TabbedContent():
                with TabPane("Componi", id="compose-tab"), VerticalScroll():
                    yield Select([(v, k) for k, v in MODES.items()], value=draft.get("mode", "free"), allow_blank=False, id="prompt-mode")
                    yield Static("Correnti: "+(", ".join(m["label"] for m in search_currents() if m["id"] in self.app.selected_currents) or "nessuna · selezionale da Correnti"), classes="muted", markup=False)
                    yield Label("La tua richiesta")
                    yield TextArea(draft.get("question", ""), id="prompt-question")
                    with Horizontal(classes="form-row"):
                        yield Input(value=draft.get("count", ""), placeholder="Quantità · vuota = automatica", id="prompt-count")
                        yield Select([("Sintetico", "concise"), ("Approfondito", "detailed")], value=draft.get("depth", "concise"), allow_blank=False, id="prompt-depth")
                    yield Select([("Appartenenza storica", "historical"), ("Anche eredità e influenze", "legacy")], value=draft.get("relation", "historical"), allow_blank=False, id="prompt-relation")
                    yield Input(value=draft.get("preferences", ""), placeholder="Preferenze · lingua, durata, atmosfera…", id="prompt-preferences")
                    yield Input(value=draft.get("exclusions", ""), placeholder="Esclusioni", id="prompt-exclusions")
                    yield Input(value=draft.get("watched", ""), placeholder="Già visti · separati da punto e virgola", id="prompt-watched")
                with TabPane("Prompt modificabile", id="result-tab"):
                    yield TextArea(draft.get("output", ""), id="prompt-output")
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
