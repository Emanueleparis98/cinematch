"""Filter declared source information without guessing missing audio tracks or pixels."""
from __future__ import annotations

import re
from dataclasses import dataclass

from .models import normalized

LANGUAGES = [("Tutte le lingue", "all"), ("Italiano", "ITA"), ("Inglese", "ENG"),
             ("Giapponese", "JPN"), ("Francese", "FRA"), ("Spagnolo", "SPA"),
             ("Tedesco", "GER"), ("Coreano", "KOR"), ("Cinese", "CHI"),
             ("Portoghese", "POR"), ("Sottotitoli italiani", "SUB-ITA"),
             ("Multilingua", "MULTI"), ("Lingua non indicata", "unknown")]
QUALITIES = [("Tutte le definizioni", "all"), ("4K / 2160p", "4K"), ("1080p / Full HD", "1080p"),
             ("720p", "720p"), ("HD · risoluzione non indicata", "HD"),
             ("SD / 480–576p", "SD"), ("Definizione non indicata", "unknown")]
ALIASES = {"ITA": r"ita|italian[oa]?|italiano|italiana", "ENG": r"eng|english|inglese",
           "JPN": r"jpn|jap|japanese|giapponese", "FRA": r"fra|fre|french|francese",
           "SPA": r"spa|esp|spanish|spagnolo", "GER": r"ger|deu|german|tedesco",
           "KOR": r"kor|korean|coreano", "CHI": r"chi|zho|chinese|cinese",
           "POR": r"por|portuguese|portoghese"}
FLAGS = {"🇮🇹": "ITA", "🇬🇧": "ENG", "🇺🇸": "ENG", "🇯🇵": "JPN", "🇫🇷": "FRA",
         "🇪🇸": "SPA", "🇩🇪": "GER", "🇰🇷": "KOR", "🇨🇳": "CHI", "🇵🇹": "POR", "🇧🇷": "POR"}


def token(pattern, text):
    return bool(re.search(r"(?<![a-z0-9])(?:"+pattern+r")(?![a-z0-9])", text))


@dataclass(frozen=True)
class SourceInfo:
    quality: str
    audio: frozenset[str]
    subtitles: frozenset[str]
    multi: bool

    @property
    def language_label(self):
        values = [code for _, code in LANGUAGES if code in self.audio]
        if self.multi:
            values.append("MULTI")
        if "ITA" in self.subtitles:
            values.append("SUB-ITA")
        return " / ".join(values) or "—"


def source_info(stream):
    text = normalized(f"{stream.name} {stream.description} {stream.filename}")
    subtitles = set()
    # Subtitle markers describe subtitles, never proof of an Italian audio track.
    for code, pattern in ALIASES.items():
        marker = r"(?<![a-z0-9])(?:sub(?:s|titles|bed|tit[a-z]*)?|sottotitoli)[ ._:\-]*(?:"+pattern+r")(?![a-z0-9])"
        if re.search(marker, text):
            subtitles.add(code)
            text = re.sub(marker, " ", text)
        suffix = r"(?<![a-z0-9])(?:"+pattern+r")[ ._:\-]+(?:subs?|subtitles|sottotitoli)(?![a-z0-9])"
        if re.search(suffix, text):
            subtitles.add(code)
            text = re.sub(suffix, " ", text)
    for lang in stream.subtitle_languages:
        if not isinstance(lang, str):
            continue
        code = lang.upper().split("-")[0]
        if code in {"IT", "ITA", "ITALIAN", "ITALIANO"}:
            subtitles.add("ITA")
    audio = {code for code, pattern in ALIASES.items() if token(pattern, text)}
    audio.update(code for flag, code in FLAGS.items() if flag in text)
    quality = "unknown"
    for label, pattern in [("4K", r"2160[pi]?|4k|uhd"), ("1080p", r"1080[pi]?|fhd|full[ ._-]*hd"),
                           ("720p", r"720[pi]?"), ("SD", r"480[pi]?|576[pi]?|sd"), ("HD", r"hd")]:
        if token(pattern, text):
            quality = label
            break
    return SourceInfo(quality, frozenset(audio), frozenset(subtitles), token(r"multi(?:lingua|lingual)?|dual[ ._-]*audio", text) or len(audio) > 1)


def matches_stream(stream, *, language="all", quality="all", query=""):
    info = source_info(stream)
    if quality != "all" and quality != info.quality:
        return False
    if language == "unknown" and (info.audio or info.subtitles or info.multi):
        return False
    if language == "SUB-ITA" and "ITA" not in info.subtitles:
        return False
    if language == "MULTI" and not info.multi:
        return False
    if language not in {"all", "unknown", "SUB-ITA", "MULTI"} and language not in info.audio:
        return False
    haystack = normalized(f"{stream.name} {stream.description} {stream.addon} {stream.filename}")
    return all(word in haystack for word in normalized(query).split())
