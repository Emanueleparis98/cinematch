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
    if season is None:
        return 1
    try:
        return max(0, int(season))
    except (TypeError, ValueError):
        return 1


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
    if not isinstance(data, dict) or not isinstance(data.get("id"), (str, int)) or isinstance(data.get("id"), bool):
        return None
    if not isinstance(data.get("name"), str) or not plain(data["name"], 250).strip():
        return None
    kind = data.get("type", "movie")
    if kind not in {"movie", "series", "tv", "channel", "anime"}:
        return None
    mid = str(data["id"])
    def strings(value, limit):
        return [plain(s, 100) for s in value if isinstance(s, str)][:limit] if isinstance(value, list) else []
    def scalar(value, limit):
        if not value or isinstance(value, bool) or not isinstance(value, (str, int, float)):
            return ""
        return plain(str(value), limit)
    videos = []
    for video in (data.get("videos") if isinstance(data.get("videos"), list) else [])[:2000]:
        if not isinstance(video, dict) or not isinstance(video.get("id"), str):
            continue
        item = dict(video)
        for key in ("season", "episode"):
            if item.get(key) is None:
                item.pop(key, None)
                continue
            try:
                number = int(item[key])
            except (TypeError, ValueError):
                item.pop(key, None)
            else:
                item[key] = max(0, number)
        videos.append(item)
    return Media(mid, kind, plain(data["name"], 250), year=scalar(data.get("releaseInfo") or data.get("year"), 30),
                 overview=plain(data.get("description"), 4000), source=source, addon_url=addon_url,
                 imdb=mid if re.fullmatch(r"tt\d+", mid) else "",
                 genres=strings(data.get("genres") or data.get("genre"), 20),
                 cast=strings(data.get("cast"), 12), directors=strings(data.get("director"), 6),
                 videos=videos,
                 rating=scalar(data.get("imdbRating"), 10))
