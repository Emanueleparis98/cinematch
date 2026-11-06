from __future__ import annotations

import asyncio
import json
import ntpath
import os
import shutil
import sys
import tempfile
from pathlib import Path

from .models import CineError, web_url


class Player:
    def __init__(self, store):
        self.store = store
        self.process = None
        self.temporary = None
        self.socket = None
        self.position = 0.0
        self.duration = 0.0

    def available(self):
        choice = self.store.config.get("player", "auto")
        override = os.environ.get("CINEMATCH_PLAYER", "")
        if override and Path(override).is_file():
            return override
        for name in ([choice] if choice != "auto" else ["mpv", "vlc", "ffplay"]):
            path = shutil.which(name)
            if path:
                return path
        if sys.platform == "win32" and choice in {"auto", "vlc"}:
            for key in ["ProgramFiles", "ProgramFiles(x86)"]:
                path = Path(os.environ.get(key, "C:/Program Files"))/"VideoLAN/VLC/vlc.exe"
                if path.is_file():
                    return str(path)
        return None

    def command(self, stream, *, resume=0, title="CineMatch"):
        executable = self.available()
        if not executable:
            raise CineError("Installa mpv o VLC e riapri CineMatch. Arch: sudo pacman -S mpv · Ubuntu: sudo apt install mpv")
        url = web_url(stream.url)
        name = ntpath.basename(executable).lower()
        headers = stream.headers
        if name.startswith("mpv"):
            args = [executable, "--no-config", "--force-window=yes", "--cache=yes", "--ytdl=no", "--title="+title]
            if self.socket:
                args += ["--input-ipc-server="+self.socket]
            # Only whitelisted request headers are forwarded by the stream parser.
            options = {"start": str(max(0, resume))}
            for key, value in headers.items():
                args.append("--http-header-fields-append="+key+": "+value)
            for sub in stream.subtitles:
                args.append("--sub-file="+web_url(sub))
            for key, value in options.items():
                args.append("--"+key+"="+value)
            return args+["--", url]
        if name.startswith("vlc"):
            unsupported = {k.lower() for k in headers} - {"referer", "user-agent"}
            if unsupported:
                raise CineError("Questa sorgente richiede header personalizzati: seleziona mpv nelle impostazioni.")
            args = [executable, "--play-and-exit", "--no-video-title-show", "--start-time="+str(max(0, resume))]
            for key, value in headers.items():
                args.append(("--http-referrer=" if key.lower() == "referer" else "--http-user-agent=")+value)
            return args+["--", url]
        args = [executable, "-loglevel", "error", "-autoexit", "-window_title", title]
        if headers:
            args += ["-headers", "".join(k+": "+v+"\r\n" for k, v in headers.items())]
        return args+["-ss", str(max(0, resume)), "-i", url]

    async def play(self, stream, *, resume=0, title="CineMatch"):
        if stream.kind in {"external", "torrent"}:
            raise CineError("Questa sorgente deve essere risolta prima di avviare il player.")
        await self.stop()
        self.position, self.duration = float(resume), 0.0
        self.temporary = tempfile.TemporaryDirectory(prefix="cm-")
        self.socket = str(Path(self.temporary.name)/"mpv.sock") if os.name != "nt" else None
        try:
            args = self.command(stream, resume=resume, title=title)
            self.process = await asyncio.create_subprocess_exec(*args, stdin=asyncio.subprocess.DEVNULL,
                                                               stdout=asyncio.subprocess.DEVNULL, stderr=asyncio.subprocess.DEVNULL)
            await asyncio.sleep(0.8)
            if self.process.returncode is not None:
                raise CineError("Il player si è chiuso: verifica la disponibilità della sorgente o scegli un altro flusso.")
        except BaseException:
            await self.stop()
            raise

    async def progress(self):
        if not self.socket or not self.process or self.process.returncode is not None:
            return self.position, self.duration
        try:
            reader, writer = await asyncio.wait_for(asyncio.open_unix_connection(self.socket), 1)
            try:
                writer.write(b'{"command":["get_property","time-pos"],"request_id":1}\n{"command":["get_property","duration"],"request_id":2}\n')
                await writer.drain()
                for _ in range(10):
                    line = await asyncio.wait_for(reader.readline(), 1)
                    if not line:
                        break
                    data = json.loads(line)
                    value = data.get("data")
                    if type(value) in {int, float}:
                        if data.get("request_id") == 1:
                            self.position = max(0, value)
                        elif data.get("request_id") == 2:
                            self.duration = max(0, value)
                            break
            finally:
                writer.close()
                await writer.wait_closed()
        except (OSError, ValueError, TimeoutError):
            pass
        return self.position, self.duration

    async def stop(self):
        if self.process and self.process.returncode is None:
            await self.progress()
            try:
                self.process.terminate()
                await asyncio.wait_for(self.process.wait(), 3)
            except ProcessLookupError:
                pass
            except TimeoutError:
                self.process.kill()
                await self.process.wait()
        self.process = None
        if self.temporary:
            self.temporary.cleanup()
            self.temporary = None
        self.socket = None
