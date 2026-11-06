from __future__ import annotations

import asyncio
import re
from urllib.parse import parse_qs, quote, urlsplit, urlunsplit

from .cache import AsyncTTLCache
from .models import CineError, Stream, parse_media, plain, web_url

DEFAULTS = ["https://v3-cinemeta.strem.io/manifest.json", "https://anime-kitsu.strem.fun/manifest.json"]
REQUEST_TIMEOUT = 12.0
SEARCH_TIMEOUT = 25.0
STREAMS_TIMEOUT = 18.0
JSON_TTL = 60.0
STREAMS_TTL = 60.0


def manifest_url(value):
    url = web_url(re.sub(r"^stremio://", "https://", value.strip(), flags=re.IGNORECASE))
    parts = urlsplit(url)
    if re.search(r"/(?:app/)?configure/?$", parts.path):
        raise CineError("Questo è il configuratore: copia il link che termina con manifest.json.")
    if not parts.path.endswith("/manifest.json"):
        if parts.path.endswith(".json"):
            raise CineError("Per l’addon serve manifest.json; per una lista usa Importa JSON.")
        parts = parts._replace(path=parts.path.rstrip("/")+"/manifest.json")
    return urlunsplit(parts)


def parse_manifest(data, url):
    if not isinstance(data, dict) or not isinstance(data.get("id"), str) or not isinstance(data.get("name"), str) or not isinstance(data.get("resources"), list) or not isinstance(data.get("types"), list):
        raise CineError("Manifest non valido: mancano id, nome, risorse o tipi.")
    hints = data.get("behaviorHints") or {}
    if not isinstance(hints, dict):
        raise CineError("Manifest non valido: behaviorHints deve essere un oggetto.")
    if hints.get("configurationRequired"):
        raise CineError("Configura l’addon sul suo sito e copia il manifest personalizzato.")
    if not all(isinstance(t, str) for t in data["types"]) or not isinstance(data.get("catalogs", []), list):
        raise CineError("Manifest non valido: tipi e cataloghi devono essere elenchi.")
    return {"url": manifest_url(url), "id": plain(data["id"], 160), "name": plain(data["name"], 100),
            "types": data["types"], "resources": data["resources"], "idPrefixes": [p for p in (data.get("idPrefixes") or []) if isinstance(p, str)],
            "catalogs": data.get("catalogs", []), "enabled": True}


def supports(addon, resource, kind, mid=""):
    for item in addon.get("resources", []):
        entry = {"name": item, "types": addon.get("types", [])} if isinstance(item, str) else item
        if not isinstance(entry, dict) or entry.get("name") != resource or kind not in entry.get("types", addon.get("types", [])):
            continue
        prefixes = entry.get("idPrefixes", addon.get("idPrefixes", [])) or []
        if not mid or not prefixes or any(mid.startswith(p) for p in prefixes if isinstance(p, str)):
            return True
    return False


def endpoint(addon, resource, kind, mid, extra=""):
    parts = urlsplit(addon["url"])
    path = parts.path[:-len("manifest.json")]+"/".join(quote(str(p), safe="") for p in (resource, kind, mid))
    if extra:
        path += "/"+extra
    return urlunsplit(parts._replace(path=path+".json"))


def parse_streams(data, addon=""):
    items = data if isinstance(data, list) else data.get("streams") if isinstance(data, dict) else None
    if not isinstance(items, list) or len(items) > 500:
        raise CineError("Il JSON deve contenere una lista streams con massimo 500 elementi.")
    streams, seen = [], set()
    for item in items:
        if not isinstance(item, dict):
            continue
        url, kind = "", "video"
        info_hash = item.get("infoHash", "")
        if isinstance(item.get("url"), str) and item["url"].startswith("magnet:"):
            info_hash = parse_qs(urlsplit(item["url"]).query).get("xt", [""])[0].removeprefix("urn:btih:")
        for field in ("url", "externalUrl"):
            try:
                url = web_url(item.get(field))
                kind = "external" if field == "externalUrl" else "hls" if ".m3u8" in urlsplit(url).path.lower() else "dash" if ".mpd" in urlsplit(url).path.lower() else "video"
                break
            except CineError:
                pass
        if not url:
            if not isinstance(info_hash, str) or not re.fullmatch(r"[a-fA-F0-9]{40}", info_hash):
                continue
            kind = "torrent"
        info_hash = info_hash if isinstance(info_hash, str) else ""
        idx = item.get("fileIdx")
        idx = idx if type(idx) is int and idx >= 0 else None
        key = (url, info_hash if not url else "", idx)
        if key in seen:
            continue
        seen.add(key)
        hints = item.get("behaviorHints") if isinstance(item.get("behaviorHints"), dict) else {}
        proxy = hints.get("proxyHeaders") if isinstance(hints.get("proxyHeaders"), dict) else {}
        raw_headers = proxy.get("request") if isinstance(proxy.get("request"), dict) else {}
        headers = {k: v for k, v in raw_headers.items() if isinstance(k, str) and isinstance(v, str)
                   and k.lower() in {"referer", "user-agent", "origin", "cookie", "authorization"} and not re.search(r"[\r\n\x00]", k+v)}
        subtitles, subtitle_languages = [], []
        for sub in (item.get("subtitles") if isinstance(item.get("subtitles"), list) else [])[:30]:
            try:
                subtitles.append(web_url(sub.get("url")))
                subtitle_languages.append(plain(sub.get("lang"), 30))
            except (CineError, AttributeError):
                pass
        streams.append(Stream(plain(item.get("name"), 100) or "Video", plain(item.get("description") or item.get("title"), 1000),
                              addon, url, kind, info_hash.lower(), idx, headers, subtitles,
                              filename=plain(hints.get("filename"), 500), subtitle_languages=subtitle_languages))
    return streams


class Addons:
    def __init__(self, net, store):
        self.net, self.store = net, store
        self.semaphore = asyncio.Semaphore(5)
        self.defaults = []
        self.errors = []
        self.offsets = {}
        self._json_cache = AsyncTTLCache(ttl=JSON_TTL, maxsize=256)
        self._streams_cache = AsyncTTLCache(ttl=STREAMS_TTL, maxsize=128)

    @property
    def all(self):
        return [a for a in [*self.defaults, *self.store.config["addons"]] if a.get("enabled", True)]

    @property
    def signature(self):
        return tuple((a["url"], bool(a.get("enabled", True))) for a in self.all)

    async def add(self, url, *, default=False):
        url = manifest_url(url)
        addon = parse_manifest(await self.json(url), url)
        # idPrefixes can be specified globally as well as on individual resources.
        if default:
            self.defaults = [a for a in self.defaults if a["url"] != addon["url"]]
            self.defaults.append(addon)
        else:
            items = self.store.config["addons"]
            self.store.config["addons"] = [a for a in items if a["url"] != url]+[addon]
            self.store.save_config()
        return addon

    async def initialize(self):
        async def load(url):
            try:
                await self.add(url, default=True)
            except (CineError, ValueError, TypeError):
                self.errors.append("Un catalogo integrato non è raggiungibile.")
        await asyncio.gather(*(load(url) for url in DEFAULTS))

    async def _load_json(self, url):
        async with self.semaphore:
            try:
                async with asyncio.timeout(REQUEST_TIMEOUT):
                    return await self.net.json(url, redirects=True)
            except TimeoutError:
                raise CineError("Il server dell’addon non ha risposto in tempo.") from None

    async def json(self, url):
        return await self._json_cache.get(url, lambda: self._load_json(url))

    async def search(self, query, section="all", skip=0):
        self.errors = []
        if not skip:
            self.offsets = {}
        async def one(addon, cat):
            kind, cid = cat.get("type"), cat.get("id")
            if not isinstance(kind, str) or not isinstance(cid, str):
                return []
            extras = cat.get("extra", [])
            names = {e.get("name") for e in extras if isinstance(e, dict)}
            searchable = "search" in names or "search" in cat.get("extraSupported", [])
            if query and not searchable and kind not in {"tv", "channel"}:
                return []
            required = {e.get("name") for e in extras if isinstance(e, dict) and e.get("isRequired")}
            if required - ({"search"} if query else set()) - {"skip"}:
                return []
            parts = []
            if query and searchable:
                parts.append("search="+quote(query, safe=""))
            key = (addon["url"], kind, cid, query, section)
            offset = self.offsets.get(key, 0)
            if skip and ("skip" in names or "skip" in cat.get("extraSupported", [])):
                if not offset:
                    return []
                parts.append(f"skip={offset}")
            elif skip:
                return []
            try:
                data = await self.json(endpoint(addon, "catalog", kind, cid, "&".join(parts)))
                raw = data.get("metas", [])[:200]
                self.offsets[key] = offset+len(raw) if raw else 0
                items = [m for item in raw if isinstance(item, dict) and (m := parse_media({"type": kind, **item}, source=addon["name"], addon_url=addon["url"]))]
                return [m for m in items if query.casefold() in m.title.casefold()] if query and not searchable else items
            except (CineError, ValueError, TypeError, AttributeError, KeyError, TimeoutError):
                self.errors.append(addon["name"]+": catalogo non disponibile.")
                return []
        tasks = []
        for addon in self.all:
            anime = "kitsu" in addon["id"].lower() or "anime" in addon["name"].lower()
            if section == "anime" and not anime:
                continue
            if section != "anime" and addon in self.defaults and anime:
                continue
            for cat in addon.get("catalogs", [])[:30]:
                if not isinstance(cat, dict):
                    continue
                kind = cat.get("type")
                if section == "movie" and kind != "movie" or section == "series" and kind != "series" or section == "tv" and kind not in {"tv", "channel"}:
                    continue
                tasks.append(asyncio.create_task(one(addon, cat)))
        if not tasks:
            return []
        try:
            done, pending = await asyncio.wait(tasks, timeout=SEARCH_TIMEOUT)
        except BaseException:
            for task in tasks:
                if not task.done():
                    task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            raise
        if pending:
            self.errors.append("Alcune fonti non hanno risposto in tempo.")
            for task in pending:
                task.cancel()
            await asyncio.gather(*pending, return_exceptions=True)
        seen, items = set(), []
        for task in tasks:
            if task not in done or task.cancelled():
                continue
            try:
                group = task.result()
            except Exception:
                self.errors.append("Una fonte non ha risposto.")
                continue
            for media in group:
                if media.key not in seen:
                    seen.add(media.key)
                    items.append(media)
        return items

    async def details(self, media):
        addons = sorted(self.all, key=lambda a: a["url"] != media.addon_url)
        for addon in addons:
            if supports(addon, "meta", media.type, media.id):
                try:
                    data = await self.json(endpoint(addon, "meta", media.type, media.id))
                    detail = parse_media(data.get("meta"), source=addon["name"], addon_url=addon["url"])
                    if detail:
                        return detail
                except (CineError, AttributeError, ValueError, TypeError, KeyError, TimeoutError):
                    continue
        return media

    async def streams(self, media, video_id=""):
        self.errors = []
        addons = self.all
        signature = tuple((a["url"], bool(a.get("enabled", True))) for a in addons)
        key = (media.key, media.imdb, media.tmdb, media.addon_url, video_id, signature)
        return await self._streams_cache.get(key, lambda: self._load_streams(media, video_id, addons))

    async def _load_streams(self, media, video_id, addons):
        mid = video_id or media.id
        # Prefer native IDs; TMDB results obtain IMDb in the detail request.
        if media.tmdb:
            mid = (media.imdb or media.id)+((":"+video_id.split(":")[-2]+":"+video_id.split(":")[-1]) if video_id and video_id.count(":") >= 2 else "")
        async def one(addon):
            candidate = mid
            if media.tmdb and not supports(addon, "stream", media.type, candidate):
                candidate = media.id+(":"+":".join(video_id.split(":")[-2:]) if video_id else "")
            if not supports(addon, "stream", media.type, candidate):
                return []
            try:
                return parse_streams(await self.json(endpoint(addon, "stream", media.type, candidate)), addon["name"])
            except (CineError, AttributeError, TypeError, ValueError, KeyError, TimeoutError):
                self.errors.append(addon["name"]+": flussi non disponibili.")
                return []
        tasks = [asyncio.create_task(one(addon)) for addon in addons]
        if not tasks:
            return []
        try:
            done, pending = await asyncio.wait(tasks, timeout=STREAMS_TIMEOUT)
        except BaseException:
            for task in tasks:
                if not task.done():
                    task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            raise
        if pending:
            self.errors.append("Alcune fonti non hanno risposto in tempo.")
            for task in pending:
                task.cancel()
            await asyncio.gather(*pending, return_exceptions=True)
        streams, seen = [], set()
        for task in tasks:
            if task not in done or task.cancelled():
                continue
            try:
                group = task.result()
            except Exception:
                self.errors.append("Una fonte non ha risposto.")
                continue
            for stream in group:
                identity = (stream.info_hash, stream.file_idx, stream.url) if stream.info_hash else (stream.url, stream.kind)
                if identity in seen:
                    continue
                seen.add(identity)
                streams.append(stream)
        return streams

    def clear_cache(self):
        self._json_cache.clear()
        self._streams_cache.clear()
