import asyncio
import json

import httpx
import pytest
from cinematch.addons import parse_manifest
from cinematch.app import CinemaApp
from cinematch.models import CineError, Media, Stream
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
from textual.widgets import Button, DataTable, Input, Select, Static, TextArea

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


async def wait_until(pilot, condition):
    # A screen can be idle before its queued Select messages start a worker,
    # especially with the Windows event loop. Wait for the observable result.
    async with asyncio.timeout(5):
        while not condition():
            await pilot.pause(0.01)
    await pilot.pause()


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
            await wait_until(pilot, lambda: isinstance(app.screen, StreamsScreen) and bool(app.screen.items))
            assert isinstance(app.screen, StreamsScreen)
            assert app.screen.video_id == "tt12345:2:1"
            assert app.screen.query_one(DataTable).row_count == 2
            app.screen.query_one(Input).value = "ITA"
            await pilot.pause()
            assert app.screen.query_one(DataTable).row_count == 1
    finally:
        await service.close()


async def test_startup_and_empty_search_never_load_popular_catalogs(tmp_path):
    calls = []
    async def forbidden(request):
        calls.append(request)
        return httpx.Response(500)
    service = cinema(tmp_path, forbidden)
    app = CinemaApp(service)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            await pilot.pause()
            app.query_one("#section", Select).value = "anime"
            app.search()
            await pilot.pause()
            await app.workers.wait_for_complete()
            assert not calls and not app.items
            assert app.query_one("#streams", Button).disabled
            assert not await service.search("")
    finally:
        await service.close()


async def test_explore_filters_apply_to_episode_picker_and_persist(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.query_one("#language", Select).value = "ITA"
            app.query_one("#quality", Select).value = "1080p"
            app.query_one("#section", Select).value = "series"
            app.query_one("#search", Input).value = "prova"
            app.search()
            await pilot.pause()
            await app.workers.wait_for_complete()
            await pilot.pause()
            app.streams()
            await pilot.pause()
            await app.workers.wait_for_complete()
            screen = app.screen
            assert isinstance(screen, StreamsScreen)
            assert len(screen.visible_items) == 1 and screen.visible_items[0].name == "1080p ITA"
            screen.query_one("#stream-quality", Select).value = "4K"
            await pilot.pause()
            assert not screen.visible_items and screen.query_one("#play", Button).disabled
            screen.reset_filters()
            await pilot.pause()
            assert len(screen.visible_items) == 2
            await pilot.press("escape")
            await pilot.pause()
            assert app.query_one("#quality", Select).value == "all"
            assert app.query_one("#language", Select).value == "all"
            saved = Store(tmp_path).config
            assert saved["stream_language"] == "all" and saved["stream_quality"] == "all"
    finally:
        await service.close()


async def test_empty_search_cannot_play_previous_selection(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            app.query_one("#search", Input).value = "prova"
            app.search()
            await pilot.pause()
            await app.workers.wait_for_complete()
            await pilot.pause()
            assert app.current
            app.query_one("#search", Input).value = ""
            app.search()
            await pilot.pause()
            assert app.current is None and app.query_one("#streams", Button).disabled
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
            series = Media("tt12345", "series", "Una serie", videos=[{"id": "tt12345:1:1", "season": 1, "episode": 1}])
            for screen in [SettingsScreen(), AddonsScreen(), CurrentsScreen(), PromptScreen(), ImportScreen(), StreamsScreen(Media("demo", "movie", "Un film"), imported=[]), StreamsScreen(series)]:
                app.push_screen(screen)
                await pilot.pause()
                close = screen.query_one("#close", Button)
                assert close.region.bottom <= size[1], (type(screen).__name__, close.region, size)
                if isinstance(screen, StreamsScreen) and screen.has_episode_picker:
                    assert screen.query_one("#stream-episode", Select).region.bottom <= size[1]
                    assert screen.query_one(DataTable).size.height >= 1
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
            assert not app.screen.query_one("#stream-episode-row").display
    finally:
        await service.close()


@pytest.mark.parametrize("kind", ["series", "anime"])
async def test_stream_picker_changes_episode_request_and_playback(tmp_path, kind):
    requested, played = [], []

    async def api(request):
        requested.append(request.url.path)
        return httpx.Response(200, json={"streams": [{"name": "1080p ITA", "url": "https://cdn.example.org/video.mp4"}]})

    service = cinema(tmp_path, api)
    # Anime catalogues commonly expose the Stremio series resource.
    service.store.config["addons"][0]["types"].append("anime")

    async def play(stream, media, video_id):
        played.append(video_id)

    service.play = play
    app = CinemaApp(service, autoload=False)
    media = Media("tt12345", kind, "Serie", videos=[
        {"id": "tt12345:0:1", "season": 0, "episode": 1},
        {"id": "tt12345:1:1", "season": 1, "episode": 1},
        {"id": "tt12345:1:2", "season": 1, "episode": 2},
        {"id": "tt12345:2:1", "season": 2, "episode": 1}])
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            screen = StreamsScreen(media, "tt12345:1:2")
            app.push_screen(screen)
            await wait_until(pilot, lambda: bool(screen.items))
            assert screen.query_one("#stream-season", Select).value == 1
            assert screen.query_one("#stream-episode", Select).value == "tt12345:1:2"
            screen.query_one("#stream-language", Select).value = "ITA"
            screen.query_one("#stream-filter", Input).value = "1080"
            screen.query_one("#stream-episode", Select).value = "tt12345:1:1"
            await wait_until(pilot, lambda: screen.video_id == "tt12345:1:1" and bool(screen.items))
            assert requested[-1].endswith("/tt12345:1:1.json")
            screen.query_one("#stream-season", Select).value = 2
            await wait_until(pilot, lambda: screen.video_id == "tt12345:2:1" and bool(screen.items))
            assert screen.video_id == "tt12345:2:1"
            screen.query_one("#stream-season", Select).value = 0
            await wait_until(pilot, lambda: screen.video_id == "tt12345:0:1" and bool(screen.items))
            assert requested[-1].endswith("/tt12345:0:1.json")
            assert screen.query_one("#stream-language", Select).value == "ITA"
            assert screen.query_one("#stream-filter", Input).value == "1080"
            assert len(screen.visible_items) == 1
            screen.play()
            await app.workers.wait_for_complete()
            assert played == ["tt12345:0:1"]
    finally:
        await service.close()


async def test_tmdb_picker_loads_seasons_and_handles_missing_episodes(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    media = Media("tmdb:42", "series", "Serie TMDB", tmdb=42, imdb="tt12345", seasons=[0, 1, 2, 3])
    requested = []

    async def tmdb_request(path, **params):
        requested.append(path)
        if path.endswith("/2"):
            return {"episodes": []}
        if path.endswith("/3"):
            raise CineError("Fonte non disponibile")
        return {"episodes": [{"episode_number": 1, "name": "Primo"}, {"episode_number": 2, "name": "Secondo"}]}

    service.tmdb.request = tmdb_request
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            screen = StreamsScreen(media, "tt12345:1:2")
            app.push_screen(screen)
            await wait_until(pilot, lambda: bool(screen.items))
            assert requested == ["/tv/42/season/1"]
            assert screen.video_id == "tt12345:1:2"
            for season in (2, 3):
                screen.query_one("#stream-season", Select).value = season
                message = "Nessun episodio" if season == 2 else "Non riesco"
                await wait_until(pilot, lambda: message in str(screen.query_one("#streams-status", Static).render()))
                assert not screen.video_id and not screen.items
                assert screen.query_one("#play", Button).disabled
                assert screen.query_one("#stream-episode", Select).disabled
            screen.query_one("#stream-season", Select).value = 0
            await wait_until(pilot, lambda: screen.video_id == "tt12345:0:1" and bool(screen.items))
            assert screen.video_id == "tt12345:0:1" and screen.items
    finally:
        await service.close()


async def test_changing_episode_clears_old_streams_and_ignores_late_response(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    started, release = asyncio.Event(), asyncio.Event()

    async def streams(media, video_id):
        if video_id.endswith(":2"):
            started.set()
            try:
                await release.wait()
            except asyncio.CancelledError:
                # Simulate a source that finishes despite cancellation.
                await release.wait()
        return [Stream(video_id, url="https://cdn.example.org/video.mp4")]

    service.streams = streams
    media = Media("tt12345", "series", "Serie", videos=[
        {"id": f"tt12345:1:{ep}", "season": 1, "episode": ep} for ep in (1, 2, 3)])
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            screen = StreamsScreen(media, "tt12345:1:1")
            app.push_screen(screen)
            await wait_until(pilot, lambda: bool(screen.items))
            screen.query_one("#stream-episode", Select).value = "tt12345:1:2"
            await wait_until(pilot, started.is_set)
            assert started.is_set()
            assert not screen.items and screen.query_one("#play", Button).disabled
            screen.query_one("#stream-episode", Select).value = "tt12345:1:3"
            await wait_until(pilot, lambda: screen.video_id == "tt12345:1:3" and bool(screen.items))
            release.set()
            await app.workers.wait_for_complete()
            assert [s.name for s in screen.items] == ["tt12345:1:3"]
    finally:
        release.set()
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


async def test_stream_picker_can_close_while_source_returns(tmp_path):
    service = cinema(tmp_path)
    app = CinemaApp(service, autoload=False)
    started, release, finished = asyncio.Event(), asyncio.Event(), asyncio.Event()

    async def streams(media, video_id):
        started.set()
        try:
            await release.wait()
        except asyncio.CancelledError:
            await release.wait()
        finished.set()
        return [Stream("Video", url="https://cdn.example.org/video.mp4")]

    service.streams = streams
    try:
        async with app.run_test(size=(120, 44)) as pilot:
            screen = StreamsScreen(Media("tt12345", "movie", "Film"))
            app.push_screen(screen)
            await wait_until(pilot, started.is_set)
            screen.dismiss()
            await pilot.pause()
            release.set()
            await wait_until(pilot, finished.is_set)
            assert not isinstance(app.screen, StreamsScreen)
    finally:
        release.set()
        await service.close()
