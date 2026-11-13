import httpx
import pytest
from termovies.addons import Addons, parse_manifest
from termovies.models import Media
from termovies.net import Network
from termovies.store import Store
from termovies.tmdb import Tmdb


async def test_mixed_catalog_pagination_tracks_each_source_offset(tmp_path):
    store = Store(tmp_path)
    for name in ["a", "b"]:
        store.config["addons"].append(parse_manifest({"id": name, "name": name, "types": ["movie"], "resources": ["catalog"],
            "catalogs": [{"type": "movie", "id": "top", "extra": [{"name": "search"}, {"name": "skip"}]}]}, f"https://{name}.example.org/manifest.json"))
    calls = []
    async def api(request):
        calls.append(str(request.url))
        count = 3 if request.url.host.startswith("a.") else 2
        if "skip=" in str(request.url):
            assert f"skip={count}" in str(request.url)
            return httpx.Response(200, json={"metas": []})
        return httpx.Response(200, json={"metas": [{"id": request.url.host+str(i), "name": f"Film {i}", "type": "movie"} for i in range(count)]})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        addons = Addons(net, store)
        assert len(await addons.search("prova", "movie")) == 5
        assert await addons.search("prova", "movie", skip=1) == []
        assert len(calls) == 4
    finally:
        await net.close()


async def test_tv_without_search_extra_can_still_find_channels(tmp_path):
    store = Store(tmp_path)
    store.config["addons"] = [parse_manifest({"id": "tv", "name": "Canali", "types": ["tv"], "resources": ["catalog", "stream"], "catalogs": [{"type": "tv", "id": "channels"}]}, "https://tv.example.org/manifest.json")]
    async def api(request):
        return httpx.Response(200, json={"metas": [{"id": "channel-1", "name": "Canale Cultura", "type": "tv"}, {"id": "channel-2", "name": "Altro canale", "type": "tv"}]})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        result = await Addons(net, store).search("Cultura", "tv")
        assert len(result) == 1 and result[0].id == "channel-1"
    finally:
        await net.close()


@pytest.mark.parametrize("key", ["a"*32, "eyJdemo.signature.token"])
async def test_tmdb_italian_search_details_and_episode_ids(tmp_path, key):
    store = Store(tmp_path)
    store.config["tmdb_key"] = key
    calls = []
    async def api(request):
        calls.append(request)
        assert request.url.params["language"] == "it-IT"
        if key.startswith("eyJ"):
            assert request.headers["authorization"] == "Bearer "+key and "api_key" not in request.url.params
        else:
            assert request.url.params["api_key"] == key
        if "/search/" in request.url.path:
            assert request.url.params["page"] == "2"
            data = {"results": [{"id": 10, "name": "Serie italiana", "first_air_date": "2020-01-01"}]}
        elif "/season/" in request.url.path:
            data = {"episodes": [{"episode_number": 3, "name": "Terzo episodio"}]}
        else:
            data = {"id": 10, "name": "Serie italiana", "first_air_date": "2020-01-01", "external_ids": {"imdb_id": "tt12345"}, "seasons": [{"season_number": 2}]}
        return httpx.Response(200, json=data)
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        tmdb = Tmdb(net, store)
        items = await tmdb.search("Serie", "series", skip=1)
        media = await tmdb.details(items[0])
        assert media.imdb == "tt12345" and media.seasons == [2]
        episodes = await tmdb.episodes(media, 2)
        assert episodes[0]["id"] == "tt12345:2:3"
    finally:
        await net.close()


async def test_tmdb_bearer_prefix_is_case_insensitive(tmp_path):
    store = Store(tmp_path)
    store.config["tmdb_key"] = "bEaReR   "+"a"*32
    seen = []
    async def api(request):
        seen.append(request.url.params["api_key"])
        return httpx.Response(200, json={"images": {}})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        await Tmdb(net, store).request("/configuration")
        assert seen == ["a"*32]
    finally:
        await net.close()


def test_tmdb_parse_does_not_invent_ratings_or_imdb_ids():
    media = Tmdb.parse({"id": 10, "title": "Film", "vote_average": 0, "imdb_id": 123}, "movie")
    assert media.rating == "" and media.imdb == ""
    assert Tmdb.parse({"id": 11, "title": "   "}, "movie") is None


async def test_debrid_key_is_not_injected_into_addon_requests(tmp_path):
    store = Store(tmp_path)
    store.config.update(debrid_provider="rd", debrid_key="globally-private")
    store.config["addons"] = [parse_manifest({"id": "demo", "name": "Demo", "types": ["series"], "resources": ["stream"], "idPrefixes": ["kitsu:"]}, "https://addon.example.org/manifest.json")]
    async def api(request):
        assert "globally-private" not in str(request.url) and "authorization" not in request.headers
        assert request.url.path.endswith("kitsu:747:8.json")
        return httpx.Response(200, json={"streams": [{"infoHash": "a"*40, "fileIdx": 8}]})
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        streams = await Addons(net, store).streams(Media("kitsu:747", "series", "Anime"), "kitsu:747:8")
        assert streams[0].file_idx == 8
    finally:
        await net.close()
