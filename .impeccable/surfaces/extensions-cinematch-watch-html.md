---
version: 1
slug: "extensions-cinematch-watch-html"
primary_target: "extensions/cinematch/watch.html"
related_targets: ["extensions/cinematch/watch.css","extensions/cinematch/watch.js","extensions/cinematch/enrich.js","extensions/cinematch/panel.html"]
---

# CineMatch — player e addon, 3.0

Mode: Operate. Extension of the accepted dark visual world, not a replacement.
Inherit DESIGN.md, Manrope, navy/cobalt tokens, custom checks and accessible lists.
No new visual system or approved-comp claim. Preserve existing design artifacts.

Task: click a movie poster in an AI answer, choose an addon stream, watch it in
the extension without a native app. A header entry makes Addon management discoverable.
Player first: film identity, 16:9 video, playback feedback, source rows and import.
Addon section: manifest input, installed toggles/removal, contextual help/configurators.

The video itself leads. Use flat source rows, not nested cards; leave the AI text
and unboxed photographic strip intact. Film posters launch the player; TMDB
attribution and person portraits retain their source links. No forms inside answers.
At 320px, actions wrap and the video stays inside the viewport. Owned pages remain dark.

Required states: empty addon/player, manifest validation, loading, network error,
playback, pause, unsupported source, imported JSON, disabled and removed addon.
Honest compatibility: HTTP/HLS in browser; no torrent engine, DRM bypass or codec
conversion. External addon availability and account setup are not design evidence.

Review evidence: controlled DeepSeek response, synthetic MP4/HLS playback,
addon and player captures at 320/600/1180; existing panel regression suite.

3.1 extension: automatic integrated decoder for MKV/legacy formats, with a flat
transport strip below the video, timeline, volume, fullscreen and track selectors.
Preserve the inherited palette and controls. Frame-led playback is primary; no
codec configuration form is added. Regular native playback keeps its native controls.
Evidence includes synthetic HEVC/AC3 5.1, audio/video rendering and decoded waveform,
E-AC3/DTS/AVI, audio languages, SRT, DASH, fullscreen, seek and source switching.
