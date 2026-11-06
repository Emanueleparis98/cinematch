from __future__ import annotations

import json
import os
import tempfile
from dataclasses import asdict
from pathlib import Path

from platformdirs import user_config_path, user_data_path

from .models import CineError, Media


class Store:
    def __init__(self, root: Path | None = None):
        self.config_path = (root or user_config_path("cinematch", appauthor=False))/"settings.json"
        self.data_path = (root or user_data_path("cinematch", appauthor=False))/"library.json"
        self.config = self.read(self.config_path, {"addons": [], "tmdb_key": "", "debrid_provider": "", "debrid_key": "", "player": "auto"})
        self.data = self.read(self.data_path, {"favorites": [], "history": [], "tasks": {}})

    @staticmethod
    def read(path, default):
        if not path.exists():
            return default
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(value, dict):
                raise TypeError()
            return {**default, **value}
        except (OSError, ValueError, TypeError):
            raise CineError("Un file locale di CineMatch è illeggibile. Ripristinalo prima di salvare le impostazioni.") from None

    @staticmethod
    def write(path, data):
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        fd, temporary = tempfile.mkstemp(prefix=".cinematch-", dir=path.parent)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(data, handle, ensure_ascii=False, indent=2)
                handle.write("\n")
            os.replace(temporary, path)
            if os.name != "nt":
                path.chmod(0o600)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)

    def save_config(self):
        self.write(self.config_path, self.config)

    def save_data(self):
        self.write(self.data_path, self.data)

    def favorite(self, media):
        items = self.data["favorites"]
        found = any(item["id"] == media.id and item["type"] == media.type for item in items)
        self.data["favorites"] = [item for item in items if (item["id"], item["type"]) != (media.id, media.type)]
        if not found:
            self.data["favorites"].insert(0, asdict(media))
        self.save_data()
        return not found

    def remember(self, media, video_id=""):
        items = [h for h in self.data["history"] if (h["media"]["id"], h["media"]["type"], h["video_id"]) != (media.id, media.type, video_id)]
        self.data["history"] = [{"media": asdict(media), "video_id": video_id}, *items][:100]
        self.save_data()

    def library(self, history=False):
        if not history:
            return [Media(**item) for item in self.data["favorites"]]
        items, seen = [], set()
        for item in self.data["history"]:
            media = Media(**item["media"])
            if media.key not in seen:
                seen.add(media.key)
                media.resume_video_id = item["video_id"]
                items.append(media)
        return items
