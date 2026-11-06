"""Read the existing CB01/Mixdrop format without executing fetched JavaScript."""
from __future__ import annotations

import html
import re
from urllib.parse import urljoin, urlsplit

from bs4 import BeautifulSoup

from .models import CineError, Media, Stream, plain, web_url

BASE = "https://cb01one.org/"
PLAYERS = {"stayonline.pro", "uprot.net", "m1xdrop.net", "mxdrop.click", "mixdrop.co", "mixdrop.ag", "mxdrop.to", "mixdrop.to"}


def player_url(value):
    try:
        url = web_url(value)
        if urlsplit(url).hostname in PLAYERS:
            return url.replace("http:", "https:", 1)
    except CineError:
        pass
    return None


def unpack(source):
    pattern = r"eval\(function\(p,a,c,k,e,d\)\{[\s\S]*?\}\('([\s\S]*?)',(\d+),(\d+),'([\s\S]*?)'\.split\('\|'\)"
    def decode(m):
        payload, radix, count, vocabulary = m.groups()
        radix, count = int(radix), int(count)
        if not 2 <= radix <= 62 or not 1 <= count <= 2000:
            return ""
        words = vocabulary.split("|")
        alphabet = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        def token(n):
            return (token(n//radix) if n >= radix else "")+alphabet[n % radix]
        payload = payload.replace("\\'", "'").replace("\\\\", "\\")
        mapping = {token(i): words[i] for i in range(min(count, len(words))) if words[i]}
        return re.sub(r"\b\w+\b", lambda t: mapping.get(t[0], t[0]), payload)
    return re.sub(pattern, decode, source)


def sources(fragment):
    soup = BeautifulSoup(fragment, "html.parser")
    found, seen = [], set()
    for tag in soup.select("[data-src], a[href], iframe[src]"):
        url = player_url(html.unescape(str(tag.get("data-src") or tag.get("href") or tag.get("src") or "")))
        if not url or url in seen:
            continue
        seen.add(url)
        label = plain(tag.get_text(" ", strip=True), 100)
        name = "Maxstream" if "uprot.net" in url else "Mixdrop"
        quality = "HD" if re.search(r"/(e|mse)/", url) else "SD"
        found.append(Stream(name+" "+quality, label, "CB01", url, "page"))
    return sorted(found, key=lambda s: ("HD" not in s.name, "Mixdrop" not in s.name))


def parse_article(text, url):
    soup = BeautifulSoup(text, "html.parser")
    meta = soup.select_one('meta[property="og:title"]')
    heading = soup.select_one("h1") or soup.title
    title = meta.get("content", "") if meta else heading.get_text(" ", strip=True) if heading else "CB01"
    title = re.split(r"\s+Streaming\s*[-–—]", title, flags=re.IGNORECASE)[0]
    year = re.search(r"\((\d{4})\)", title)
    title = re.sub(r"\s*\[(HD|UHD|4K)\]|\s*\(\d{4}\)\s*$", "", title, flags=re.IGNORECASE).strip()
    series = "/serietv/" in url
    title = re.split(r"\s*[-–—]\s*(?:Stagione|\d+\s*[×x]|COMPLETA|ITA)", title, flags=re.IGNORECASE)[0].strip()
    media = Media(url, "series" if series else "movie", plain(title, 250), year[1] if year else "", source="CB01", url=url)
    video_sources = {}
    if series:
        heads = list(re.finditer(r'<div[^>]+class=["\'][^"\']*\bsp-head\b[^"\']*["\'][^>]*>([\s\S]*?)</div>', text, re.IGNORECASE))
        for i, head in enumerate(heads):
            number = re.search(r"STAGIONE\s+(\d+)", BeautifulSoup(head[1], "html.parser").get_text(), re.IGNORECASE)
            if not number:
                continue
            season = int(number[1])
            body = text[head.end():heads[i+1].start() if i+1 < len(heads) else len(text)]
            for row in BeautifulSoup(body, "html.parser").select("p"):
                ep = re.search(r"(\d+)\s*[×x]\s*(\d+)", row.get_text())
                if not ep:
                    continue
                episode = int(ep[2])
                mid = f"cb01:{season}:{episode}"
                choices = sources(str(row))
                if choices:
                    video_sources.setdefault(mid, []).extend(choices)
                    if not any(v["id"] == mid for v in media.videos):
                        media.videos.append({"id": mid, "season": season, "episode": episode, "title": f"Episodio {episode}"})
        media.videos.sort(key=lambda v: (v["season"], v["episode"]))
    else:
        video_sources[""] = sources(text)
    return media, video_sources


class Cb01:
    def __init__(self, net):
        self.net = net
        self.articles = {}

    async def search(self, query, section="all", skip=0):
        if not query or section in {"anime", "tv"} or skip:
            return []
        base = BASE+"serietv/" if section == "series" else BASE
        text = await self.net.text(base, params={"s": query}, redirects=True, limit=2*1024*1024)
        soup = BeautifulSoup(text, "html.parser")
        seen, items = set(), []
        for anchor in soup.select("article h2 a[href], article h3 a[href], .entry-title a[href], h2 a[href]"):
            url = urljoin(base, str(anchor.get("href", "")))
            if urlsplit(url).hostname not in {"cb01one.org", "www.cb01one.org"} or url in seen:
                continue
            seen.add(url)
            title = anchor.get_text(" ", strip=True)
            year = re.search(r"\((\d{4})\)", title)
            title = re.sub(r"\s*\[(HD|UHD|4K)\]|\s*\(\d{4}\)\s*$", "", title, flags=re.IGNORECASE).strip()
            items.append(Media(url, "series" if "/serietv/" in url else "movie", plain(title, 250), year[1] if year else "", source="CB01", url=url))
        return items[:50]

    async def details(self, media):
        if urlsplit(media.url).hostname not in {"cb01one.org", "www.cb01one.org"}:
            raise CineError("Indirizzo CB01 non valido.")
        text = await self.net.text(media.url, redirects=True, limit=2*1024*1024)
        result, choices = parse_article(text, media.url)
        self.articles[media.url] = choices
        return result

    async def streams(self, media, video_id=""):
        if media.url not in self.articles:
            await self.details(media)
        return self.articles[media.url].get(video_id, [])

    async def resolve(self, stream):
        url = player_url(stream.url)
        if not url:
            raise CineError("Player CB01 non supportato.")
        if urlsplit(url).hostname == "stayonline.pro":
            match = re.search(r"/(?:e|l)/([A-Za-z0-9]+)/?", urlsplit(url).path)
            if not match:
                raise CineError("Collegamento StayOnline non valido.")
            data = await self.net.json("https://stayonline.pro/ajax/linkEmbedView.php", method="POST", data={"id": match[1]})
            url = player_url((data.get("data") or {}).get("value") or data.get("value") or "")
            if not url or urlsplit(url).hostname == "stayonline.pro":
                raise CineError("StayOnline non ha restituito un player supportato. Può essere necessaria una verifica nel browser.")
        text = await self.net.text(url, redirects=True, limit=1024*1024)
        if re.search(r"captcha|cf-challenge|just a moment", text, re.IGNORECASE):
            raise CineError("La fonte richiede una verifica nel browser. Scegli un’altra sorgente o apri il sito.")
        text = unpack(text)
        match = re.search(r'''(?:MDCore\.(?:wurl|furl)|["']?(?:file|source)["']?)\s*[:=]\s*["']([^"']+)["']''', text, re.IGNORECASE)
        if not match:
            raise CineError("Il player non ha esposto un flusso diretto. Apri la fonte nel browser.")
        direct = web_url(urljoin(url, html.unescape(match[1].replace("\\/", "/"))))
        host = urlsplit(direct).hostname or ""
        if not (host == "mxcontent.net" or host.endswith(".mxcontent.net") or host == "uprot.net"):
            raise CineError("Il player ha restituito un host video non supportato.")
        return Stream(stream.name, stream.description, "CB01", direct, "hls" if ".m3u8" in direct else "video", headers={"Referer": url})
