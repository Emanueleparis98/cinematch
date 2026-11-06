import json

import httpx
import pytest
from cinematch.addons import parse_manifest
from cinematch.app import CinemaApp
from cinematch.models import Media
from cinematch.net import Network
from cinematch.screens import (
    AddonsScreen,
    CurrentsScreen,
    ImportScreen,
    PromptScreen,
    SettingsScreen,
    StreamsScreen,
)
from cinematch.service import Cinema
from cinematch.store import Store
from textual.widgets import Button, DataTable, Input, Select, TextArea

MANIFEST = {"id": "demo", "name": "Demo", "types": ["series", "movie", "tv"], "resources": ["catalog", "meta", "stream"],
            "catalogs": [{"type": "series", "id": "top", "extra": [{"name": "search"}]}]}


def cinema(tmp_path, handler=None):
    async def api(request):
        path = str(request.url)
        if "/catalog/" in path:
            return httpx.Response(200, json={"metas": [{"id": "tt12345", "type": "series", "name": "Serie di prova", "year": "2025"}]})
        if "/meta/" in path:
            return httpx.Response(200, json={"meta": {"id": "tt12345", "type": "series", "name": "Serie di prova", "description": "Una storia.", "videos": [
                {"id": "tt12345:1:1", "season": 1, "episode": 1, "title": "Primo"},
                {"id": "tt12345:2:1", "season": 2, "episode": 1, "title": "Secondo"}]}})
        if "/stream/" in path:
            return httpx.Response(200, json={"streams": [{"name": "1080p ITA", "url": "https://cdn.example.org/video.mkv"}, {"name": "720p ENG", "url": "https://cdn.example.org/other.mp4"}]})
        return httpx.Response(200, json=MANIFEST)
    store = Store(tmp_path)
    store.config["addons"] = [parse_manifest(MANIFEST, "https://demo.example.org/manifest.json")]
    return Cinema(Network(httpx.AsyncClient(transport=httpx.MockTransport(handler or api))), store)


async def test_complete_series_flow_selects_season_episode_and_stream(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.query_one("#section", Select).value = "series"
            app.query_one("#search", Input).value = "prova"
            app.search()
            await pilot.pause()
            await app.workers.wait_for_complete()
            await pilot.pause()
            assert app.current and len(app.current.videos) == 2
            assert app.query_one("#episode", Select).value == "tt12345:1:1"
            app.query_one("#season", Select).value = 2
            await pilot.pause()
            await app.workers.wait_for_complete()
            assert app.query_one("#episode", Select).value == "tt12345:2:1"
            app.streams()
            await pilot.pause()
            await app.workers.wait_for_complete()
            assert isinstance(app.screen, StreamsScreen)
            assert app.screen.video_id == "tt12345:2:1"
            assert app.screen.query_one(DataTable).row_count == 2
            app.screen.query_one(Input).value = "ITA"
            await pilot.pause()
            assert app.screen.query_one(DataTable).row_count == 1
    finally:
        await service.close()


async def test_current_selection_prompt_generation_edit_and_escape_preserves_draft(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.action_currents()
            await pilot.pause()
            screen = app.screen
            screen.query_one(Input).value = "Rossellini"
            await pilot.pause()
            screen.toggle()
            assert len(app.selected_currents) == 1
            screen.prompt()
            await pilot.pause()
            assert isinstance(app.screen, PromptScreen)
            app.screen.query_one("#prompt-mode", Select).value = "recommendations"
            app.screen.query_one("#prompt-count", Input).value = "7"
            app.screen.query_one("#prompt-depth", Select).value = "detailed"
            app.screen.action_generate()
            await pilot.pause()
            output = app.screen.query_one("#prompt-output", TextArea).text
            assert "7 titoli" in output and "Rossellini" in output
            app.screen.query_one("#prompt-output", TextArea).load_text(output+"\nUna mia aggiunta.")
            await pilot.press("escape")
            await pilot.pause()
            app.action_prompt()
            await pilot.pause()
            assert "Una mia aggiunta" in app.screen.query_one("#prompt-output", TextArea).text
    finally:
        await service.close()


@pytest.mark.parametrize("size", [(80, 24), (100, 35), (140, 48)])
async def test_every_screen_opens_and_closes_at_supported_sizes(tmp_path, size):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=size) as pilot:
            await pilot.pause()
            for screen in [SettingsScreen(), AddonsScreen(), CurrentsScreen(), PromptScreen(), ImportScreen(), StreamsScreen(Media("demo", "movie", "Un film"), imported=[])]:
                app.push_screen(screen)
                await pilot.pause()
                close = screen.query_one("#close", Button)
                assert close.region.bottom <= size[1], (type(screen).__name__, close.region, size)
                await pilot.press("escape")
                await pilot.pause()
            assert not isinstance(app.screen, (SettingsScreen, AddonsScreen, CurrentsScreen, PromptScreen, ImportScreen, StreamsScreen))
    finally:
        await service.close()


async def test_addon_management_and_settings_are_persisted(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.action_addons()
            await pilot.pause()
            screen = app.screen
            screen.query_one("#addon-url", Input).value = "https://new.example.org/manifest.json"
            screen.add()
            await pilot.pause()
            await app.workers.wait_for_complete()
            assert len(service.store.config["addons"]) == 2
            await pilot.press("escape")
            await pilot.pause()
            app.action_settings()
            await pilot.pause()
            app.screen.query_one("#debrid-provider", Select).value = "rd"
            app.screen.query_one("#debrid-key", Input).value = "my-secret"
            app.screen.save_settings()
            assert Store(tmp_path).config["debrid_key"] == "my-secret"
            assert "my-secret" not in str(app.export_screenshot())
    finally:
        await service.close()


async def test_import_json_creates_stream_picker(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.action_import()
            await pilot.pause()
            app.screen.query_one(TextArea).load_text(json.dumps({"streams": [{"name": "Video", "url": "https://cdn.example.org/demo.mp4"}]}))
            app.screen.submit()
            await pilot.pause()
            await app.workers.wait_for_complete()
            assert isinstance(app.screen, StreamsScreen) and len(app.screen.items) == 1
    finally:
        await service.close()


async def test_kitsu_episode_id_is_not_misread_as_season_number(tmp_path):
    service = cinema(tmp_path)
    captured = {}
    async def resolve(stream, **kwargs):
        from cinematch.models import Stream
        captured.update(kwargs)
        return Stream("Video", url="https://cdn.example.org/video.mp4")
    async def play(stream, **kwargs):
        pass
    service.debrid.resolve = resolve
    service.player.play = play
    from cinematch.models import Stream
    media = Media("kitsu:747", "series", "Anime", videos=[{"id": "kitsu:747:8", "season": 1, "episode": 8}])
    try:
        await service.play(Stream("Torrent", kind="torrent", info_hash="a"*40), media, "kitsu:747:8")
        assert captured == {"season": 1, "episode": 8}
    finally:
        await service.close()
