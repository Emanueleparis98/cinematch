from __future__ import annotations

import json
from urllib.parse import urljoin

import httpx

from .models import CineError, web_url


class Network:
    def __init__(self, client=None):
        self.client = client or httpx.AsyncClient(timeout=httpx.Timeout(20, connect=8), follow_redirects=False,
                                                headers={"User-Agent": "CineMatch-Terminal/0.1 (+https://github.com/Emanueleparis98/cinematch)"})

    async def close(self):
        await self.client.aclose()

    async def _fetch(self, url, *, method="GET", limit=4*1024*1024, redirects=False, **kwargs):
        url = web_url(url)
        depth = kwargs.pop("_redirect_depth", 0)
        try:
            async with self.client.stream(method, url, follow_redirects=False, **kwargs) as response:
                if response.is_redirect:
                    if not redirects or depth >= 5 or method != "GET":
                        raise CineError("Il server richiede un reindirizzamento non supportato. Verifica l’indirizzo del manifest.")
                    target = web_url(urljoin(str(response.url), response.headers.get("location", "")))
                    if url.startswith("https:") and not target.startswith("https:"):
                        raise CineError("Il server ha reindirizzato verso un collegamento non sicuro.")
                    # Keep the public client identity consistent for signed video URLs;
                    # never forward authentication, cookies or referrers across hosts.
                    headers = kwargs.pop("headers", None) or {}
                    public_headers = {k: v for k, v in headers.items() if k.lower() in {"user-agent", "accept"}}
                    if public_headers:
                        kwargs["headers"] = public_headers
                    kwargs.pop("params", None)
                    await response.aclose()
                    return await self._fetch(target, method=method, limit=limit, redirects=True, _redirect_depth=depth+1, **kwargs)
                chunks = bytearray()
                async for chunk in response.aiter_bytes():
                    chunks.extend(chunk)
                    if len(chunks) > limit:
                        raise CineError("Risposta troppo grande per essere letta.")
                if response.status_code >= 400:
                    code = None
                    try:
                        code = json.loads(chunks).get("error_code")
                    except (ValueError, AttributeError):
                        pass
                    error = CineError(f"Il server ha risposto con errore {response.status_code}.")
                    error.status = response.status_code
                    error.code = code
                    raise error
                return bytes(chunks), str(response.url)
        except httpx.HTTPError:
            # httpx exception strings include complete URLs, sometimes containing tokens.
            raise CineError("Server non raggiungibile o troppo lento. Controlla la connessione.") from None

    async def request(self, url, **kwargs):
        body, _ = await self._fetch(url, **kwargs)
        return body

    async def json(self, url, **kwargs):
        raw = await self.request(url, **kwargs)
        if not raw:
            return None
        try:
            return json.loads(raw)
        except (ValueError, UnicodeDecodeError):
            raise CineError("Il server non ha restituito JSON valido. Usa il manifest, non il configuratore.") from None

    async def text(self, url, **kwargs):
        return (await self.request(url, **kwargs)).decode("utf-8", errors="replace")
