import pytest
from cinematch.addons import parse_streams
from cinematch.models import Stream
from cinematch.stream_filters import matches_stream, source_info


@pytest.mark.parametrize(("name", "quality"), [
    ("Movie.2160p.ITA", "4K"), ("4K UHD", "4K"), ("Film 1080p ENG", "1080p"),
    ("Full HD", "1080p"), ("720p JPN", "720p"), ("HD", "HD"),
    ("576p ITA", "SD"), ("SD", "SD"), ("Video", "unknown"),
])
def test_declared_quality_and_missing_resolution(name, quality):
    assert source_info(Stream(name)).quality == quality


def test_generic_hd_is_not_misrepresented_as_1080p_or_720p():
    stream = Stream("Mixdrop HD")
    assert matches_stream(stream, quality="HD")
    assert not matches_stream(stream, quality="1080p")
    assert not matches_stream(stream, quality="720p")


@pytest.mark.parametrize("label", ["JPN SUB-ITA 1080p", "JPN SUBITA 1080p", "JPN · Italian subtitles · 1080p"])
def test_italian_subtitles_are_not_italian_audio(label):
    stream = Stream(label)
    assert matches_stream(stream, language="SUB-ITA")
    assert matches_stream(stream, language="JPN")
    assert not matches_stream(stream, language="ITA")


def test_unknown_multilingual_stream_does_not_invent_italian_audio():
    stream = Stream("MULTI 2160p")
    assert matches_stream(stream, language="MULTI", quality="4K")
    assert not matches_stream(stream, language="ITA")
    assert not matches_stream(stream, language="unknown")


def test_addon_filename_and_subtitle_metadata_are_used():
    stream = parse_streams({"streams": [{"name": "Addon", "url": "https://cdn.example.org/video.mp4",
        "behaviorHints": {"filename": "Movie_1080p_JPN.mkv"},
        "subtitles": [{"url": "https://cdn.example.org/sub.vtt", "lang": "it-IT"}]}]})[0]
    assert matches_stream(stream, language="JPN", quality="1080p")
    assert matches_stream(stream, language="SUB-ITA", quality="1080p")
    assert not matches_stream(stream, language="ITA")


def test_language_does_not_match_fragments_in_other_words():
    stream = Stream("Digital Legend Original")
    assert matches_stream(stream, language="unknown", quality="unknown")
    assert not matches_stream(stream, language="ITA")
    assert not matches_stream(stream, language="ENG")


def test_filters_intersect_with_text_and_leave_unknowns_available_when_reset():
    streams = [Stream("1080p ITA", addon="Uno"), Stream("720p ITA", addon="Uno"),
               Stream("1080p ENG", addon="Due"), Stream("Video", addon="Due")]
    assert [s.name for s in streams if matches_stream(s, language="ITA", quality="1080p", query="uno")] == ["1080p ITA"]
    assert len([s for s in streams if matches_stream(s)]) == 4
