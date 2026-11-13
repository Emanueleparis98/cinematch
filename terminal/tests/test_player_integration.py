"""Optional real playback check, using a generated clip instead of a third-party film."""
import asyncio
import functools
import http.server
import shutil
import subprocess
import sys
import threading

import pytest
from termovies.models import Media, Stream
from termovies.net import Network
from termovies.service import Cinema
from termovies.store import Store


@pytest.mark.skipif(sys.platform != "linux" or not shutil.which("mpv") or not shutil.which("ffmpeg"), reason="Requires local Linux mpv and ffmpeg")
async def test_real_mpv_decodes_http_clip_and_saves_resume(tmp_path, monkeypatch):
    clip = tmp_path/"sample.mp4"
    await asyncio.to_thread(subprocess.run, ["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc=size=160x90:rate=12", "-f", "lavfi", "-i", "sine=frequency=440", "-t", "8", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "ultrafast", "-c:a", "aac", str(clip)], check=True, timeout=30)
    class Handler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *_):
            pass
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(tmp_path)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    service = Cinema(Network(), Store(tmp_path/"config"))
    original = service.player.command
    def headless(*args, **kwargs):
        command = original(*args, **kwargs)
        return [command[0], "--vo=null", "--ao=null", *command[1:]]
    monkeypatch.setattr(service.player, "command", headless)
    media = Media("test-clip", "movie", "Clip generata per il collaudo")
    try:
        await service.play(Stream("Test HTTP", url=f"http://127.0.0.1:{server.server_port}/sample.mp4"), media)
        await asyncio.sleep(0.5)
        await service.save_progress()
        assert service.player.position > 0.5
        assert service.player.duration == pytest.approx(8, abs=0.5)
        assert Store(tmp_path/"config").data["progress"][media.key+":"] > 0.5
    finally:
        await service.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)
