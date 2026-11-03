Synthetic five-second test pattern and 440 Hz sine wave generated locally with
FFmpeg; no external footage, faces, copyrighted movies, or network media.

Source command: `ffmpeg -f lavfi -i testsrc2=size=320x180:rate=12 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 5 -c:v libx264 -pix_fmt yuv420p -preset ultrafast -b:v 120k -g 12 -c:a aac -b:a 32k -movflags +faststart pattern.mp4`

HLS: `ffmpeg -i pattern.mp4 -c copy -hls_time 1 -hls_list_size 0 -hls_segment_filename segment%d.ts pattern.m3u8`

Test-only assets, excluded from the extension distribution.

Decoder regression variants, generated from the same five-second synthetic pattern:

- `pattern-hevc-ac3.mkv`: HEVC and AC3 5.1; audio expanded with `pan=5.1|FL=c0|FR=c0|FC=c0|LFE=c0|BL=c0|BR=c0`, `libx265 -preset ultrafast -x265-params log-level=error:pools=1:frame-threads=1`, `ac3 -b:a 192k`.
- `pattern-h264-eac3.mkv`: video copy, `eac3 -b:a 128k`.
- `pattern-h264-dts.mkv`: video copy, `dca -strict -2 -b:a 768k`.
- `pattern-mpeg4-mp3.avi`: `mpeg4 -q:v 5`, `libmp3lame -b:a 64k`.
- `pattern-tracks.mkv`: HEVC/AC3 plus the original AAC track and captions.srt, with Italian/English track metadata.
- `pattern.mpd` and `pattern-stream*.mp4`: DASH output with `-c copy -single_file 1 -use_template 0 -use_timeline 0 -seg_duration 1 -f dash`.

These are small test-only videos, not downloadable product content or commercial films.
