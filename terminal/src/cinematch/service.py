from __future__ import annotations

import asyncio
import re

from .addons import Addons
from .cache import AsyncTTLCache
from .debrid import Debrid
from .models import CineError
from .player import Player
from .tmdb import Tmdb


class Cinema:
    def __init__(self, net, store):
        self.net, self.store = net, store
        self.addons = Addons(net, store)
        self.tmdb = Tmdb(net, store)
        self.debrid = Debrid(net, store)
        self.player = Player(store)
        self._details_cache = AsyncTTLCache(ttl=300, maxsize=128)
        self._episodes_cache = AsyncTTLCache(ttl=300, maxsize=128)
        self.errors = []
        self.current = None
        self.video_id = ""

    async def search(self, query="", section="all", source="all", skip=0):
        self.errors = []
        if not query.strip():
            return []
        tasks = []
        if source in {"all", "addons"}:
            tasks.append(self.addons.search(query, section, skip))
        if source in {"all", "tmdb"} and self.store.config["tmdb_key"]:
            tasks.append(self.tmdb.search(query, section, skip))
        results = await asyncio.gather(*tasks, return_exceptions=True)
        items, seen = [], set()
        for group in results:
            if isinstance(group, BaseException):
                self.errors.append(str(group) if isinstance(group, CineError) else "Una fonte non ha risposto.")
                continue
            for m in group:
                if m.key not in seen:
                    seen.add(m.key)
                    items.append(m)
        self.errors += self.addons.errors
        return items

    async def details(self, media):
        key = (media.key, media.tmdb, media.imdb, media.addon_url, self.addons.signature)
        return await self._details_cache.get(key, lambda: self._load_details(media))

    async def _load_details(self, media):
        if media.tmdb:
            return await self.tmdb.details(media)
        return await self.addons.details(media)

    async def episodes(self, media, season):
        if media.tmdb:
            key = (media.key, media.tmdb, season)
            return await self._episodes_cache.get(key, lambda: self._load_episodes(media, season))
        return [v for v in media.videos if int(v.get("season") or 1) == season]

    async def _load_episodes(self, media, season):
        return await self.tmdb.episodes(media, season)

    async def streams(self, media, video_id=""):
        if media.type in {"series", "anime"} and media.videos and not video_id:
            raise CineError("Seleziona prima un episodio.")
        return await self.addons.streams(media, video_id)

    async def play(self, stream, media, video_id=""):
        if stream.kind == "torrent":
            ep = next((v for v in media.videos if v["id"] == video_id), {})
            # TMDB episode list may be loaded separately from the media object.
            season = ep.get("season")
            episode = ep.get("episode")
            match = re.fullmatch(r"(?:tt\d+|tmdb:\d+):(\d+):(\d+)", video_id)
            if match:
                season, episode = int(match[1]), int(match[2])
            elif video_id.startswith("kitsu:"):
                season = ep.get("season") or 1
            stream = await self.debrid.resolve(stream, season=season, episode=episode)
        if stream.kind == "external":
            raise CineError("Questo collegamento si apre nel browser: premi Apri fonte.")
        await self.save_progress()
        resume = self.store.data.get("progress", {}).get(media.key+":"+video_id, 0)
        await self.player.play(stream, resume=resume, title=media.title)
        self.current, self.video_id = media, video_id
        self.store.remember(media, video_id)

    async def save_progress(self):
        if not self.current:
            return
        position, duration = await self.player.progress()
        # Mark near-complete titles as watched rather than resuming at end credits.
        self.store.data.setdefault("progress", {})[self.current.key+":"+self.video_id] = 0 if duration and position > duration*0.95 else position
        self.store.save_data()

    def clear_cache(self):
        self._details_cache.clear()
        self._episodes_cache.clear()
        self.addons.clear_cache()

    async def close(self):
        await self.save_progress()
        await self.player.stop()
        await self.net.close()
