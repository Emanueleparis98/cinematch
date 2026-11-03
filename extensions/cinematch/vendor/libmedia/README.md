# libmedia / AVPlayer 1.3.1

Copyright (c) 2024–present Gaoxing Zhao and the libmedia contributors.
LGPL-3.0-or-later. See COPYING.LGPLv3 and COPYING.GPLv3. MIT notices for the
embedded ass-compiler and assjs dependencies are retained alongside them.

AVPlayer JavaScript comes from the official npm package, integrity checked:
https://registry.npmjs.org/@libmedia/avplayer/-/avplayer-1.3.1.tgz
SHA-512: `wRELj99DbfS0t9iKkts5p1v/PSlyGkQ7A5Vy/GNTFg4ZiNo+OWTkkW/ioQuoT8seeljqizN0lWYJjJ9Ra32fcA==`

Corresponding upstream source, codec build scripts and LGPL notices:
https://github.com/zhaohappy/libmedia/tree/152f629d3021fd8013efa464fcb7b55f9fbe7753
https://github.com/zhaohappy/libmedia/archive/152f629d3021fd8013efa464fcb7b55f9fbe7753.zip
Initialize upstream submodules when rebuilding. The upstream README documents
FFmpeg and decoder dependencies and their individual licenses. Only decoders,
resampling and audio playback resources are distributed here, no x264/x265 encoders.
FFmpeg source: https://ffmpeg.org/download.html

The individually compiled WASM decoders come from the upstream project's official
published build, fetched 2026-11-03. Exact origin and SHA-256 for each resource are
in wasm-provenance.json. integrity.json pins every shipped JS/WASM byte sequence.
No runtime code is loaded from a CDN. Assets and this separate library remain
replaceable in the unpacked extension; there are no anti-modification checks at runtime.

## Chrome extension compatibility changes

Two capabilities in the upstream feature object are disabled in avplayer.js:
`thread:w,` becomes `thread:!1,`; `audioWorklet:p,` becomes `audioWorklet:!1,`.
Chrome exposes SharedArrayBuffer to extension pages without cross-origin isolation;
the automatic thread path and generated worklet URLs otherwise fail under MV3 CSP.
The library's single-thread, AudioBufferSourceNode fallback is used instead.
Native WebCodecs/hardware acceleration remains enabled by the CineMatch adapter;
local WASM decoders handle codecs missing from the browser. No JavaScript unsafe-eval.

These changes are reproducible in scripts/vendor-libmedia.py. Run it without
arguments to verify the existing files offline, or with --restore to reconstruct
the pinned library files. The two changes are not an upstream endorsement.
CineMatch's integration source is in codec-player.js and range-reader.js in the
parent extension directory; player controls are in watch.js/watch.html/watch.css.

The bundled assjs stylesheet is extracted verbatim from 630.avplayer.js into
ass.css. watch.html links it with id ASS-global-style so assjs reuses it rather
than attempting inline style injection blocked by extension CSP. The CSS is
covered by the retained assjs MIT notice and the resource integrity manifest.
