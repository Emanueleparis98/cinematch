from __future__ import annotations

import re

from .models import CineError, Media, plain

BASE = "https://api.themoviedb.org/3"


class Tmdb:
    def __init__(self, net, store):
        self.net, self.store = net, store

    async def request(self, path, **params):
        key = self.store.config["tmdb_key"].strip().removeprefix("Bearer ")
        if not key:
            raise CineError("Collega TMDB nelle impostazioni.")
        headers = {"Accept": "application/json"}
        if re.fullmatch(r"[a-fA-F0-9]{32}", key):
            params["api_key"] = key
        elif key.startswith("eyJ") and key.count(".") == 2 and not re.search(r"\s", key):
            headers["Authorization"] = "Bearer "+key
        else:
            raise CineError("Inserisci una chiave API v3 o un Read Access Token TMDB valido.")
        try:
            return await self.net.json(BASE+path, params={"language": "it-IT", **params}, headers=headers)
        except CineError as e:
            if getattr(e, "status", 0) == 401:
                raise CineError("TMDB: credenziale non valida.") from None
            raise

    async def verify(self):
        await self.request("/configuration")

    @staticmethod
    def parse(item, kind):
        if kind not in {"movie", "tv"}:
            return None
        mid = item.get("id")
        if type(mid) is not int or mid <= 0:
            return None
        series = kind == "tv"
        return Media(f"tmdb:{mid}", "series" if series else "movie", plain(item.get("name") if series else item.get("title"), 250),
                     year=plain(item.get("first_air_date") if series else item.get("release_date"), 10)[:4],
                     overview=plain(item.get("overview"), 4000), source="TMDB", tmdb=mid, rating=str(round(item.get("vote_average") or 0, 1)),
                     imdb=item.get("external_ids", {}).get("imdb_id", "") if series else item.get("imdb_id", ""),
                     genres=[g["name"] for g in item.get("genres", [])],
                     cast=[p["name"] for p in item.get("credits", {}).get("cast", [])[:12]],
                     directors=[p["name"] for p in item.get("credits", {}).get("crew", []) if p.get("job") == "Director"] if not series else [p["name"] for p in item.get("created_by", [])],
                     seasons=[s["season_number"] for s in item.get("seasons", []) if type(s.get("season_number")) is int],
                     url=f"https://www.themoviedb.org/{kind}/{mid}")

    async def search(self, query, section="all", skip=0):
        if section in {"tv", "anime"}:
            return []
        kind = "tv" if section == "series" else "movie" if section == "movie" else "multi"
        data = await self.request(f"/search/{kind}" if query else f"/trending/{'all' if kind == 'multi' else kind}/week", **({"query": query, "include_adult": "false"} if query else {}), page=skip//20+1)
        return [m for item in data.get("results", []) if (m := self.parse(item, item.get("media_type", kind)))]

    async def details(self, media):
        kind = "tv" if media.type == "series" else "movie"
        data = await self.request(f"/{kind}/{media.tmdb}", append_to_response="credits,external_ids")
        return self.parse(data, kind) or media

    async def episodes(self, media, season):
        data = await self.request(f"/tv/{media.tmdb}/season/{season}")
        return [{"id": f"{media.imdb or media.id}:{season}:{ep['episode_number']}", "season": season, "episode": ep["episode_number"],
                 "title": plain(ep.get("name"), 250), "overview": plain(ep.get("overview"), 1500)} for ep in data.get("episodes", []) if type(ep.get("episode_number")) is int]
