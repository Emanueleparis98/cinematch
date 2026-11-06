from urllib.parse import parse_qs

import httpx
import pytest
from cinematch.cb01 import BROWSER_UA, Cb01, Cb01Error, sources
from cinematch.models import Stream
from cinematch.net import Network


async def test_stayonline_uses_public_page_session_and_normal_first_click_request():
    requests = []
    async def api(request):
        requests.append(request)
        assert request.headers["user-agent"] == BROWSER_UA
        if request.url.path == "/e/EXAMPLE/":
            return httpx.Response(200, text="<html>Continue</html>", headers={"Set-Cookie": "public_session=demo; Path=/"})
        if request.url.path.endswith("linkEmbedView.php"):
            assert request.headers["referer"] == "https://stayonline.pro/e/EXAMPLE/"
            assert request.headers["origin"] == "https://stayonline.pro"
            assert request.headers["x-requested-with"] == "XMLHttpRequest"
            assert "public_session=demo" in request.headers["cookie"]
            assert parse_qs(request.content.decode(), keep_blank_values=True) == {"id": ["EXAMPLE"], "ref": [""], "reCaptchaResponse": [""]}
            return httpx.Response(200, json={"status": "success", "data": {"value": "https://m1xdrop.net/e/VIDEO"}})
        # Mixdrop includes the library even on ordinary video pages.
        return httpx.Response(200, text='''<html><script src="https://www.google.com/recaptcha/api.js"></script>
            <script>MDCore.wurl="//media.mxcontent.net/movie.mp4?s=demo";</script></html>''')
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        stream = await Cb01(net).resolve(Stream("Mixdrop HD", url="https://stayonline.pro/e/EXAMPLE/", kind="page"))
        assert stream.url == "https://media.mxcontent.net/movie.mp4?s=demo"
        assert stream.headers == {"User-Agent": BROWSER_UA, "Referer": "https://m1xdrop.net/e/VIDEO"}
        assert len(requests) == 3
    finally:
        await net.close()


@pytest.mark.parametrize("reply", [{"status": "error", "code": 429}, {"status": "error", "code": "429"}])
async def test_stayonline_challenge_stops_without_retrying_or_forging_a_token(reply):
    calls = []
    async def api(request):
        calls.append(request)
        return httpx.Response(200, json=reply) if request.method == "POST" else httpx.Response(200, text="Continue")
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(Cb01Error) as raised:
            await Cb01(net).resolve(Stream("Mixdrop", url="https://stayonline.pro/l/EXAMPLE/", kind="page"))
        assert raised.value.code == "captcha"
        assert raised.value.manual_url == "https://stayonline.pro/l/EXAMPLE/"
        assert len(calls) == 2
    finally:
        await net.close()


async def test_stayonline_service_error_is_not_reported_as_unsupported_player():
    async def api(request):
        return httpx.Response(200, json={"status": "error", "message": "Unknown Error 6xrZ"}) if request.method == "POST" else httpx.Response(200, text="Continue")
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(Cb01Error, match="errore del servizio"):
            await Cb01(net).resolve(Stream("Mixdrop", url="https://stayonline.pro/e/EXAMPLE/", kind="page"))
    finally:
        await net.close()


async def test_deleted_mixdrop_video_is_not_confused_with_recaptcha():
    async def api(request):
        return httpx.Response(200, text='''<html><script src="https://www.google.com/recaptcha/api.js"></script>
             <h2>WE ARE SORRY</h2><p>We can't find the video you are looking for.</p></html>''')
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(Cb01Error) as raised:
            await Cb01(net).resolve(Stream("Mixdrop", url="https://m1xdrop.net/e/REMOVED", kind="page"))
        assert raised.value.code == "unavailable" and "rimosso" in str(raised.value)
        assert raised.value.manual_url == "https://m1xdrop.net/e/REMOVED"
    finally:
        await net.close()


async def test_maxstream_manual_link_points_to_actual_verification_page():
    calls = []
    async def api(request):
        calls.append(request)
        if request.url.host == "stayonline.pro":
            if request.method == "POST":
                return httpx.Response(200, json={"status": "success", "data": {"value": "https://uprot.net/msei/VERIFICATION"}})
            return httpx.Response(200, text="Continue")
        return httpx.Response(200, text='<form id="upcaptcha-form">UPCaptcha Protection <button>VERIFICA</button></form>')
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(Cb01Error) as raised:
            await Cb01(net).resolve(Stream("Maxstream HD", url="https://stayonline.pro/e/EXAMPLE/", kind="page"))
        assert raised.value.code == "captcha"
        assert raised.value.manual_url == "https://uprot.net/msei/VERIFICATION"
        assert len(calls) == 3
    finally:
        await net.close()


def test_cb01_tab_labels_and_duplicate_wrappers_keep_distinct_providers():
    text = '''<a href="https://stayonline.pro/l/MAX/">Maxstream</a>
      <a href="https://stayonline.pro/l/MIX/">Mixdrop</a>
      <div id="fancy-tabs"><a class="link-catch-all" data-counter="1">Maxstream HD</a>
      <a class="link-catch-all" data-counter="2">Mixdrop HD</a></div>
      <div class="tabs-catch-all" data-counter="1" data-src="https://stayonline.pro/e/MAX/"></div>
      <div class="tabs-catch-all" data-counter="2" data-src="https://stayonline.pro/e/MIX/"></div>'''
    streams = sources(text)
    assert [(s.name, s.url) for s in streams] == [("Mixdrop HD", "https://stayonline.pro/e/MIX/"), ("Maxstream HD", "https://stayonline.pro/e/MAX/")]


@pytest.mark.parametrize("reply", [None, [], {"status": "success", "data": None}, {"status": "success", "data": {"value": "javascript:alert(1)"}}])
async def test_malformed_stayonline_response_stays_a_safe_diagnostic(reply):
    async def api(request):
        return httpx.Response(200, json=reply) if request.method == "POST" else httpx.Response(200, text="Continue")
    net = Network(httpx.AsyncClient(transport=httpx.MockTransport(api)))
    try:
        with pytest.raises(Cb01Error):
            await Cb01(net).resolve(Stream("Video", url="https://stayonline.pro/e/EXAMPLE/", kind="page"))
    finally:
        await net.close()
