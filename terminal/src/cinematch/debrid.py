from __future__ import annotations

import hashlib
import re

from .models import CineError, Stream, web_url

SERVICES = {"rd": ("Real-Debrid", "https://api.real-debrid.com/rest/1.0/"), "torbox": ("TorBox", "https://api.torbox.app/v1/api/")}


def choose_file(files, *, file_idx=None, season=None, episode=None):
    if not isinstance(files, list) or not all(isinstance(f, dict) for f in files):
        raise CineError("Il servizio ha restituito un elenco di file non valido.")
    def video(f):
        name = f.get("path") or f.get("name") or ""
        return re.search(r"\.(mp4|mkv|avi|webm|m4v|mov|ts)$", name, re.IGNORECASE) and not re.search(r"\b(sample|trailer)\b", name, re.IGNORECASE)
    videos = [f for f in files if video(f)]
    if file_idx is not None:
        if type(file_idx) is not int or not 0 <= file_idx < len(files) or files[file_idx] not in videos:
            raise CineError("Il file indicato dall’addon non è un video disponibile.")
        return files[file_idx]
    if season is not None and episode is not None:
        pattern = rf"(?:s0*{season}e0*{episode}(?!\d)|(?:^|\D)0*{season}x0*{episode}(?!\d))"
        matches = [f for f in videos if re.search(pattern, f.get("path") or f.get("name") or "", re.IGNORECASE)]
        if len(matches) == 1:
            return matches[0]
        if len(videos) == 1 and not re.search(r"s\d+e\d+|\d+x\d+", videos[0].get("path") or videos[0].get("name") or "", re.IGNORECASE):
            return videos[0]
        raise CineError("Non posso identificare con certezza l’episodio. Scegli un risultato con file indicizzato.")
    if not videos:
        raise CineError("Il torrent non contiene video supportati.")
    def size(file):
        value = file.get("bytes") or file.get("size") or 0
        return value if isinstance(value, (int, float)) else 0
    return max(videos, key=size)


class Debrid:
    def __init__(self, net, store):
        self.net, self.store = net, store

    def credentials(self):
        provider, key = self.store.config["debrid_provider"], self.store.config["debrid_key"].strip()
        if provider not in SERVICES or not key or len(key) > 4096 or re.search(r"\s", key):
            raise CineError("Collega Real-Debrid o TorBox nelle impostazioni. La chiave verrà usata per tutti gli addon.")
        return provider, key

    async def api(self, provider, key, path, **kwargs):
        name, base = SERVICES[provider]
        try:
            data = await self.net.json(base+path, headers={"Authorization": "Bearer "+key}, **kwargs)
        except CineError as e:
            status, code = getattr(e, "status", 0), getattr(e, "code", None)
            if provider == "rd" and code == 35:
                raise CineError("Real-Debrid: file bloccato dal servizio per violazione del copyright (codice 35). Scegli un’altra sorgente; CineMatch non può sbloccarlo.") from None
            if status == 451:
                raise CineError(name+": accesso rifiutato per una restrizione legale (451).") from None
            if provider == "rd" and code == 22:
                raise CineError("Real-Debrid: indirizzo IP non autorizzato (codice 22).") from None
            if provider == "rd" and code in {23, 36}:
                raise CineError("Real-Debrid: quota o limite di utilizzo raggiunto.") from None
            if provider == "rd" and code in {24, 28}:
                raise CineError("Real-Debrid: file non disponibile o non consentito dal servizio.") from None
            if provider == "rd" and code == 37:
                raise CineError("Real-Debrid: operazione API disabilitata dal servizio.") from None
            if status in {401, 403}:
                raise CineError(name+": verifica chiave e abilitazione dell’account.") from None
            if status == 429:
                raise CineError(name+": troppe richieste. Attendi prima di riprovare.") from None
            raise
        if provider == "torbox":
            if not isinstance(data, dict) or data.get("success") is not True:
                raise CineError("TorBox: operazione non disponibile. Controlla cache e limiti dell’account.")
            return data.get("data")
        if data is not None and not isinstance(data, dict):
            raise CineError("Real-Debrid: risposta non valida.")
        if data is None and not path.startswith("torrents/selectFiles/"):
            raise CineError("Real-Debrid: risposta vuota.")
        return data

    async def verify(self):
        provider, key = self.credentials()
        await self.api(provider, key, "user" if provider == "rd" else "user/me")

    async def resolve(self, stream, *, season=None, episode=None):
        provider, key = self.credentials()  # Snapshot: credentials never rotate during an operation.
        if not re.fullmatch(r"[a-f0-9]{40}", stream.info_hash):
            raise CineError("Hash BitTorrent v1 non valido.")
        magnet = "magnet:?xt=urn:btih:"+stream.info_hash
        scope = hashlib.sha256((provider+":"+key).encode()).hexdigest()
        task_key = scope+":"+stream.info_hash
        if provider == "rd":
            tid = self.store.data["tasks"].get(task_key)
            if not tid:
                added = await self.api(provider, key, "torrents/addMagnet", method="POST", data={"magnet": magnet})
                tid = str(added.get("id", ""))
                if not re.fullmatch(r"[A-Za-z0-9_-]+", tid):
                    raise CineError("Real-Debrid: torrent non disponibile.")
                self.store.data["tasks"][task_key] = tid
                self.store.save_data()
            try:
                info = await self.api(provider, key, "torrents/info/"+tid)
            except CineError as e:
                if getattr(e, "status", 0) == 404:
                    self.store.data["tasks"].pop(task_key, None)
                    self.store.save_data()
                    raise CineError("Torrent rimosso da Real-Debrid. Scegli di nuovo la sorgente per aggiungerlo.") from None
                raise
            if not info.get("files"):
                raise CineError("Real-Debrid sta acquisendo i dati del torrent. Riprova tra poco.")
            file = choose_file(info["files"], file_idx=stream.file_idx, season=season, episode=episode)
            if info.get("status") == "waiting_files_selection":
                await self.api(provider, key, "torrents/selectFiles/"+tid, method="POST", data={"files": str(file["id"])})
                info = await self.api(provider, key, "torrents/info/"+tid)
            if info.get("status") != "downloaded":
                raise CineError("Torrent aggiunto a Real-Debrid, ma non ancora pronto. Attendi il completamento sul servizio e riprova.")
            selected = [f for f in info["files"] if f.get("selected") == 1]
            index = next((i for i, f in enumerate(selected) if f["id"] == file["id"]), -1)
            if index < 0 or index >= len(info.get("links", [])):
                raise CineError("Il file richiesto non è selezionato su Real-Debrid.")
            result = await self.api(provider, key, "unrestrict/link", method="POST", data={"link": info["links"][index]})
            url = result.get("download")
        else:
            # TorBox accepts multipart/form-data for createtorrent.
            added = await self.api(provider, key, "torrents/createtorrent", method="POST", files={
                "magnet": (None, magnet), "add_only_if_cached": (None, "true"), "allow_zip": (None, "false")})
            tid = added.get("torrent_id") if isinstance(added, dict) else None
            if type(tid) is not int:
                raise CineError("TorBox: torrent non disponibile in cache.")
            info = await self.api(provider, key, "torrents/mylist", params={"id": tid, "bypass_cache": "true"})
            if isinstance(info, list):
                info = next((i for i in info if i.get("id") == tid), {})
            if not info or not info.get("download_finished") or not info.get("files"):
                raise CineError("TorBox: torrent non ancora pronto. Riprova più tardi.")
            file = choose_file(info["files"], file_idx=stream.file_idx, season=season, episode=episode)
            url = await self.api(provider, key, "torrents/requestdl", params={"token": key, "torrent_id": tid, "file_id": file["id"], "redirect": "false"})
        url = web_url(url)
        if not url.startswith("https:") or key in url:
            raise CineError("Il servizio non ha restituito un collegamento video utilizzabile.")
        return Stream(stream.name, stream.description, stream.addon, url, "video", subtitles=stream.subtitles)
