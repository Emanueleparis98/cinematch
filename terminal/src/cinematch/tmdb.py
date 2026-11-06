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
        if kind not in {"movie", "tv"} or not isinstance(item, dict):
            return None
        mid = item.get("id")
        if type(mid) is not int or mid <= 0:
            return None
        series = kind == "tv"
        external = item.get("external_ids") if isinstance(item.get("external_ids"), dict) else {}
        credits = item.get("credits") if isinstance(item.get("credits"), dict) else {}
        genres = item.get("genres") if isinstance(item.get("genres"), list) else []
        seasons = item.get("seasons") if isinstance(item.get("seasons"), list) else []
        created_by = item.get("created_by") if isinstance(item.get("created_by"), list) else []
        cast = credits.get("cast") if isinstance(credits.get("cast"), list) else []
        crew = credits.get("crew") if isinstance(credits.get("crew"), list) else []
        vote = item.get("vote_average")
        vote = round(vote, 1) if isinstance(vote, (int, float)) else 0
        return Media(f"tmdb:{mid}", "series" if series else "movie", plain(item.get("name") if series else item.get("title"), 250),
                     year=plain(item.get("first_air_date") if series else item.get("release_date"), 10)[:4],
                     overview=plain(item.get("overview"), 4000), source="TMDB", tmdb=mid, rating=str(vote),
                     imdb=external.get("imdb_id", "") if series else item.get("imdb_id", ""),
                     genres=[g["name"] for g in genres if isinstance(g, dict) and isinstance(g.get("name"), str)],
                     cast=[p["name"] for p in cast[:12] if isinstance(p, dict) and isinstance(p.get("name"), str)],
                     directors=[p["name"] for p in crew if isinstance(p, dict) and p.get("job") == "Director" and isinstance(p.get("name"), str)] if not series else [p["name"] for p in created_by if isinstance(p, dict) and isinstance(p.get("name"), str)],
                     seasons=[s["season_number"] for s in seasons if isinstance(s, dict) and type(s.get("season_number")) is int],
                     url=f"https://www.themoviedb.org/{kind}/{mid}")

    async def search(self, query, section="all", skip=0):
        if section in {"tv", "anime"}:
            return []
        kind = "tv" if section == "series" else "movie" if section == "movie" else "multi"
        data = await self.request(f"/search/{kind}" if query else f"/trending/{'all' if kind == 'multi' else kind}/week", **({"query": query, "include_adult": "false"} if query else {}), page=skip+1)
        results = data.get("results", []) if isinstance(data, dict) else []
        return [m for item in results if isinstance(item, dict) and (m := self.parse(item, item.get("media_type", kind)))]

    async def details(self, media):
        kind = "tv" if media.type == "series" else "movie"
        data = await self.request(f"/{kind}/{media.tmdb}", append_to_response="credits,external_ids")
        return self.parse(data, kind) or media

    async def episodes(self, media, season):
        data = await self.request(f"/tv/{media.tmdb}/season/{season}")
        episodes = data.get("episodes", []) if isinstance(data, dict) else []
        return [{"id": f"{media.imdb or media.id}:{season}:{ep['episode_number']}", "season": season, "episode": ep["episode_number"],
                 "title": plain(ep.get("name"), 250), "overview": plain(ep.get("overview"), 1500)} for ep in episodes if isinstance(ep, dict) and type(ep.get("episode_number")) is int]
