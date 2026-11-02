Synthetic five-second test pattern and 440 Hz sine wave generated locally with
FFmpeg; no external footage, faces, copyrighted movies, or network media.

Source command: `ffmpeg -f lavfi -i testsrc2=size=320x180:rate=12 -f lavfi -i sine=frequency=440:sample_rate=44100 -t 5 -c:v libx264 -pix_fmt yuv420p -preset ultrafast -b:v 120k -g 12 -c:a aac -b:a 32k -movflags +faststart pattern.mp4`

HLS: `ffmpeg -i pattern.mp4 -c copy -hls_time 1 -hls_list_size 0 -hls_segment_filename segment%d.ts pattern.m3u8`

Test-only assets, excluded from the extension distribution.
