from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from urllib.parse import urlsplit, urlunsplit


class CineError(Exception):
    """A diagnostic that is safe to show without response bodies or secret URLs."""


def plain(value, limit=1000):
    value = value if isinstance(value, str) else ""
    return "".join(c for c in value[:limit] if c in "\n\t" or unicodedata.category(c)[0] != "C")


def normalized(value):
    return "".join(c for c in unicodedata.normalize("NFD", str(value)).casefold() if not unicodedata.combining(c))


def web_url(value):
    if not isinstance(value, str) or len(value) > 16000 or re.search(r"[\x00-\x20\x7f]", value):
        raise CineError("Indirizzo HTTP/HTTPS non valido.")
    try:
        url = urlsplit(value)
        if url.scheme not in {"https", "http"} or not url.hostname or url.username or url.password:
            raise ValueError()
        _ = url.port
    except ValueError:
        raise CineError("Indirizzo HTTP/HTTPS non valido.") from None
    return urlunsplit(url._replace(fragment=""))


@dataclass
class Media:
    id: str
    type: str
    title: str
    year: str = ""
    overview: str = ""
    source: str = ""
    addon_url: str = ""
    imdb: str = ""
    tmdb: int | None = None
    genres: list[str] = field(default_factory=list)
    cast: list[str] = field(default_factory=list)
    directors: list[str] = field(default_factory=list)
    videos: list[dict] = field(default_factory=list)
    seasons: list[int] = field(default_factory=list)
    rating: str = ""
    url: str = ""
    resume_video_id: str = ""

    @property
    def key(self):
        return f"{self.type}:{self.id}"

    @property
    def available_seasons(self):
        return self.seasons or sorted({episode_season(v) for v in self.videos}) or [1]

    def season_for_video(self, video_id):
        video = next((v for v in self.videos if v["id"] == video_id), None)
        if video is not None:
            return episode_season(video)
        match = re.fullmatch(r"(?:tt\d+|tmdb:\d+):(\d+):(\d+)", video_id)
        return int(match[1]) if match else None


def episode_season(video):
    season = video.get("season")
    return int(season) if season is not None else 1


@dataclass
class Stream:
    name: str
    description: str = ""
    addon: str = ""
    url: str = field(default="", repr=False)
    kind: str = "video"
    info_hash: str = ""
    file_idx: int | None = None
    headers: dict[str, str] = field(default_factory=dict, repr=False)
    subtitles: list[str] = field(default_factory=list, repr=False)
    filename: str = ""
    subtitle_languages: list[str] = field(default_factory=list)


def parse_media(data, *, source="", addon_url=""):
    if not isinstance(data, dict) or not isinstance(data.get("id"), (str, int)) or not data.get("name"):
        return None
    kind = data.get("type", "movie")
    if kind not in {"movie", "series", "tv", "channel", "anime"}:
        return None
    mid = str(data["id"])
    def strings(value, limit):
        return [plain(s, 100) for s in value if isinstance(s, str)][:limit] if isinstance(value, list) else []
    return Media(mid, kind, plain(data["name"], 250), year=plain(str(data.get("releaseInfo") or data.get("year") or ""), 30),
                 overview=plain(data.get("description"), 4000), source=source, addon_url=addon_url,
                 imdb=mid if re.fullmatch(r"tt\d+", mid) else "",
                 genres=strings(data.get("genres") or data.get("genre"), 20),
                 cast=strings(data.get("cast"), 12), directors=strings(data.get("director"), 6),
                 videos=[v for v in (data.get("videos") or []) if isinstance(v, dict) and isinstance(v.get("id"), str)][:2000],
                 rating=plain(str(data.get("imdbRating") or ""), 10))
