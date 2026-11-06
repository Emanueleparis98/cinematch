import json
import os
from pathlib import Path

import httpx
import pytest
from cinematch.addons import (
    endpoint,
    manifest_url,
    parse_manifest,
    parse_streams,
    supports,
)
from cinematch.debrid import Debrid, choose_file
from cinematch.models import CineError, Media, Stream, parse_media, web_url
from cinematch.net import Network
from cinematch.player import Player
from cinematch.prompts import catalog, generate, search_currents
from cinematch.store import Store

ROOT = Path(__file__).resolve().parents[2]


def test_catalog_is_the_canonical_catalog():
    assert len(catalog()) == 74
    assert catalog() == json.loads((ROOT/"plugins/cinema-currents/assets/movements.json").read_text(encoding="utf-8"))
    assert search_currents("néoréalisme") or search_currents("Rossellini")
    assert search_currents("Méliès")[0]["id"] == "cinema_attrazioni"


def test_prompts_free_does_not_inject_preserved_currents():
    prompt = generate(question="Consigliami anime brevi", movements=["id_inesistente"], watched=["Akira"], exclusions="spoiler")
    assert "Consigliami anime brevi" in prompt and "Akira" in prompt
    assert "CONTESTO DELLE VOCI" not in prompt
    assert "Non inventare" in prompt


def test_prompt_comparison_and_historical_boundaries():
    choices = [m["id"] for m in catalog()[:2]]
    with pytest.raises(CineError, match="due correnti"):
        generate(mode="compare", movements=choices[:1])
    prompt = generate(mode="compare", movements=choices, count=8, depth="detailed", relation="legacy")
    assert "Confini:" in prompt and "Caratteristiche:" in prompt and "8 titoli complessivi" in prompt
    assert "influenze successive" in prompt and "senza fonderle" in prompt


@pytest.mark.parametrize("value", ["javascript:alert(1)", "file:///etc/passwd", "https://user:secret@example.org", "https://example.org/\r\nX:1"])
def test_urls_reject_non_web_and_injection(value):
    with pytest.raises(CineError):
        web_url(value)


def test_manifest_config_and_personalized_paths():
    with pytest.raises(CineError, match="configuratore"):
        manifest_url("https://example.org/configure")
    url = manifest_url("stremio://example.org/personal/manifest.json?key=abc")
    addon = parse_manifest({"id": "demo", "name": "Demo", "types": ["series"], "resources": ["stream"], "idPrefixes": ["kitsu:"]}, url)
    assert supports(addon, "stream", "series", "kitsu:1:2")
    assert not supports(addon, "stream", "series", "tt12345:1:2")
    assert endpoint(addon, "stream", "series", "kitsu:1:2") == "https://example.org/personal/stream/series/kitsu%3A1%3A2.json?key=abc"


def test_streams_preserve_file_selection_headers_and_subtitles():
    items = parse_streams({"streams": [
        {"infoHash": "A"*40, "fileIdx": 2, "name": "1080p ITA"},
        {"url": "https://cdn.example.org/v.m3u8", "behaviorHints": {"proxyHeaders": {"request": {"Referer": "https://example.org/", "Host": "evil", "Cookie": "bad\r\nX:1"}}}, "subtitles": [{"url": "https://cdn.example.org/sub.vtt"}]},
        {"externalUrl": "https://example.org/watch"}, {"url": "https://cdn.example.org/v.m3u8"}, {"infoHash": "invalid"}
    ]}, "Demo")
    assert len(items) == 3
    assert items[0].file_idx == 2 and items[0].info_hash == "a"*40
    assert items[1].headers == {"Referer": "https://example.org/"}
    assert len(items[1].subtitles) == 1 and items[2].kind == "external"


def test_nullable_cinemeta_fields_keep_all_episodes():
    m = parse_media({"id": "tt0903747", "type": "series", "name": "Breaking Bad", "director": None, "cast": None,
                     "genres": None, "videos": [{"id": "tt0903747:1:1", "season": 1, "episode": 1}]})
    assert len(m.videos) == 1 and m.directors == []


def test_episode_never_selects_different_episode_or_sample():
    files = [{"id": 1, "path": "S01E01.mkv", "bytes": 100}, {"id": 2, "path": "S01E02.mkv", "bytes": 1000}, {"id": 3, "path": "sample.mkv", "bytes": 5000}]
    assert choose_file(files, season=1, episode=1)["id"] == 1
    assert choose_file(files)["id"] == 2
    with pytest.raises(CineError):
        choose_file(files, file_idx=2)
    with pytest.raises(CineError, match="certezza"):
        choose_file(files, season=2, episode=1)


def test_storage_atomic_private_and_favorites(tmp_path):
    store = Store(tmp_path)
    store.config["debrid_key"] = "secret"
    store.save_config()
    if os.name != "nt":
        assert store.config_path.stat().st_mode & 0o777 == 0o600
    media = Media("tt12345", "movie", "Un film")
    assert store.favorite(media)
    assert Store(tmp_path).library()[0].title == "Un film"
    assert not store.favorite(media)
    assert not store.library()


async def test_network_redacts_secret_urls_and_provider_bodies():
    async def fail(request):
        raise httpx.ConnectError("https://example.org/TOKEN/manifest.json", request=request)
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(fail)))
    try:
        with pytest.raises(CineError) as error:
            await net.json("https://example.org/TOKEN/manifest.json")
        assert "TOKEN" not in str(error.value)
    finally:
        await net.close()


async def test_public_catalog_redirect_does_not_forward_authorization():
    requests = []
    async def api(request):
        requests.append(request)
        if request.url.host == "catalog.example.org":
            return httpx.Response(307, headers={"Location": "https://cdn.example.org/catalog.json"})
        assert "authorization" not in request.headers
        return httpx.Response(200, json={"metas": []})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        assert await net.json("https://catalog.example.org/", redirects=True, headers={"Authorization": "Bearer personal-key"}) == {"metas": []}
        assert len(requests) == 2
    finally:
        await net.close()


async def test_redirect_to_http_is_refused():
    async def api(request):
        return httpx.Response(302, headers={"Location": "http://example.org/"})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(CineError, match="non sicuro"):
            await net.json("https://example.org/", redirects=True)
    finally:
        await net.close()


async def test_redirect_keeps_only_public_client_headers():
    async def api(request):
        if request.url.host == "before.example.org":
            return httpx.Response(302, headers={"Location": "https://after.example.org/player"})
        assert request.headers["user-agent"] == "CineMatch-compatibility-client"
        assert not any(key in request.headers for key in ["authorization", "cookie", "referer", "origin"])
        return httpx.Response(200, text="Player")
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        assert await net.request("https://before.example.org/player", redirects=True,
            headers={"User-Agent": "CineMatch-compatibility-client", "Authorization": "Bearer private", "Cookie": "private=value", "Referer": "https://private.example/", "Origin": "https://private.example"}) == b"Player"
    finally:
        await net.close()


async def test_rd_35_stops_and_does_not_misdiagnose_key(tmp_path):
    requests = []
    async def refused(request):
        requests.append(request)
        return httpx.Response(451, json={"error_code": 35, "error": "private service body"})
    store = Store(tmp_path)
    store.config.update(debrid_provider="rd", debrid_key="account-secret")
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(refused)))
    try:
        with pytest.raises(CineError, match="codice 35") as error:
            await Debrid(net, store).resolve(Stream("Torrent", kind="torrent", info_hash="a"*40))
        assert len(requests) == 1
        assert "chiave" not in str(error.value) and "private service body" not in str(error.value)
    finally:
        await net.close()


async def test_rd_selects_correct_file_and_isolates_accounts(tmp_path):
    store = Store(tmp_path)
    store.config.update(debrid_provider="rd", debrid_key="account-one")
    calls = []
    async def api(request):
        calls.append((request.url.path, request.headers.get("authorization"), request.content))
        path = request.url.path
        if path.endswith("addMagnet"):
            return httpx.Response(200, json={"id": "TASK1" if request.headers["authorization"].endswith("one") else "TASK2"})
        if "/info/" in path:
            return httpx.Response(200, json={"status": "downloaded", "files": [{"id": 1, "path": "S01E01.mkv", "selected": 1}, {"id": 2, "path": "S01E02.mkv", "selected": 1}], "links": ["https://rd.example/one", "https://rd.example/two"]})
        return httpx.Response(200, json={"download": "https://cdn.example.org/video.mkv"})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        client = Debrid(net, store)
        stream = Stream("Torrent", kind="torrent", info_hash="a"*40, file_idx=1)
        assert (await client.resolve(stream)).url == "https://cdn.example.org/video.mkv"
        assert b"two" in calls[-1][2]
        store.config["debrid_key"] = "account-two"
        await client.resolve(stream)
        assert len([c for c in calls if c[0].endswith("addMagnet")]) == 2
        assert len(store.data["tasks"]) == 2
        assert "account-one" not in json.dumps(store.data)
    finally:
        await net.close()


async def test_torbox_multipart_cached_file_and_api_key_stays_on_service(tmp_path):
    store = Store(tmp_path)
    store.config.update(debrid_provider="torbox", debrid_key="torbox-secret")
    calls = []
    async def api(request):
        calls.append(request)
        if request.url.path.endswith("createtorrent"):
            assert "multipart/form-data" in request.headers["content-type"]
            assert b'add_only_if_cached' in request.content
            data = {"torrent_id": 8}
        elif request.url.path.endswith("mylist"):
            data = [{"id": 8, "download_finished": True, "files": [{"id": 10, "name": "Movie.mkv", "size": 100}]}]
        else:
            assert request.url.params["file_id"] == "10"
            data = "https://cdn.example.org/ready.mkv"
        return httpx.Response(200, json={"success": True, "data": data})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        result = await Debrid(net, store).resolve(Stream("Torrent", kind="torrent", info_hash="a"*40))
        assert result.url == "https://cdn.example.org/ready.mkv"
        assert all(r.url.host == "api.torbox.app" for r in calls)
    finally:
        await net.close()


def test_player_uses_argv_and_never_shell(monkeypatch, tmp_path):
    player = Player(Store(tmp_path))
    monkeypatch.setattr(player, "available", lambda: "/usr/bin/mpv")
    command = player.command(Stream("Movie", url="https://cdn.example.org/film.mkv?x=$(touch-pwned)"), resume=90)
    assert command[-2] == "--" and "--start=90" in command
    assert command[-1].endswith("$(touch-pwned)")
