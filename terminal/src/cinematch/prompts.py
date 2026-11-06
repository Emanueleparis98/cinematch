"""Local prompt composer, using the same canonical cinema catalog as the extension."""
from __future__ import annotations

import json
from importlib.resources import files

from .models import CineError, normalized

KINDS = {"movement": "Movimento storico", "tendency": "Tendenza / scena", "cycle": "Genere / ciclo", "tradition": "Tradizione", "technique": "Tecnica / ambito produttivo"}
MODES = {"free": "Richiesta libera", "recommendations": "Consigli di visione", "introduction": "Introduzione e percorso", "discoveries": "Opere meno conosciute", "compare": "Confronta correnti"}
PROVIDERS = {"DeepSeek": "https://chat.deepseek.com/", "ChatGPT": "https://chatgpt.com/", "Claude": "https://claude.ai/", "Gemini": "https://gemini.google.com/"}


def catalog():
    return json.loads(files("cinematch").joinpath("movements.json").read_text(encoding="utf-8"))


def search_currents(query="", kind="all"):
    words = normalized(query).split()
    return [m for m in catalog() if (kind == "all" or m["kind"] == kind) and all(w in normalized(" ".join([
        m["label"], m["original_name"], m["summary"], *m["aliases"], *m["regions"], *m["authors"], *[v["title"] for v in m["works"]]
    ])) for w in words)]


def period(m):
    return f"circa {m['start']}–{m['end']}" if m["end"] else f"dal {m['start']} circa · sviluppi e continuità"


def generate(*, mode="free", movements=(), question="", count=None, depth="concise", relation="historical", preferences="", exclusions="", watched=()):
    if mode not in MODES or depth not in {"concise", "detailed"} or relation not in {"historical", "legacy"}:
        raise CineError("Impostazioni del prompt non valide.")
    if count is not None and (type(count) is not int or not 1 <= count <= 4294967295):
        raise CineError("Il numero di titoli deve essere un intero positivo oppure automatico.")
    free = mode == "free"
    if free and not question.strip():
        raise CineError("Scrivi la richiesta libera.")
    if not free and not 1 <= len(movements) <= 4:
        raise CineError("Seleziona da una a quattro correnti.")
    lookup = {m["id"]: m for m in catalog()}
    try:
        selected = [] if free else [lookup[mid] for mid in dict.fromkeys(movements)]
    except KeyError:
        raise CineError("Corrente non riconosciuta.") from None
    if mode == "compare" and len(selected) < 2:
        raise CineError("Per il confronto scegli almeno due correnti.")
    out = ["Aiutami a esplorare il cinema con attenzione storica. Rispondi in italiano.", "", f"OBIETTIVO: {MODES[mode]}."]
    if count:
        out.append(f"Quantità richiesta: {count} titoli complessivi, senza duplicati. Se non trovi abbastanza opere pertinenti, spiega il limite invece di aggiungere titoli estranei.")
    elif free:
        out.append("Rispondi alla richiesta libera rispettandone il formato; proponi film, serie o anime soltanto se richiesti.")
    else:
        out.append("Quantità: proponi un numero motivato e adeguato alla richiesta, senza un limite predefinito di cinque film.")
    if selected:
        out.extend(["", "CONTESTO DELLE VOCI SELEZIONATE"])
        for m in selected:
            out.append(f"- {m['label']} / {m['original_name']} — {KINDS[m['kind']]}; {period(m)}; {', '.join(m['regions'])}.\n  {m['summary']}\n  Confini: {m['scope_note']}")
            if depth == "detailed":
                out.append(f"  Caratteristiche: {'; '.join(m['traits'])}. Figure di riferimento: {', '.join(m['authors'])}.")
            if m["sources"]:
                out.append(f"  Riferimento da consultare se puoi navigare: {m['sources'][0]['title']} — {m['sources'][0]['url']}")
        out.extend(["", "Privilegia opere riconosciute come appartenenti alla corrente o al ciclo richiesto. Una coincidenza di anno, paese o genere non basta. Se includi un precursore o un caso discusso, dichiaralo." if relation == "historical" else "Includi anche eredità e influenze successive, distinguendole chiaramente dalle appartenenze storiche. Motiva il collegamento concreto."])
    extra = {"introduction": "Costruisci un percorso accessibile dal contesto storico alle opere; spiega come guardarle e in quale ordine.", "discoveries": "Privilegia opere meno note rispetto ai titoli canonici; non dichiararle rare o disponibili in streaming senza verifica.", "compare": "Confronta le voci senza fonderle: differenze, punti di contatto ed esempi distinti; ripartisci la quantità complessiva in modo esplicito."}
    if mode in extra:
        out.append(extra[mode])
    for title, text in [("PREFERENZE PERSONALI", preferences), ("CRITERI DA ESCLUDERE", exclusions), ("GIÀ VISTI / GIÀ PROPOSTI — non riproporre questi titoli", "; ".join(watched)), ("LA MIA RICHIESTA", question)]:
        if text.strip():
            out.extend(["", title, text.strip()])
    out.extend(["", "COME RISPONDERE", "Per ogni titolo usa un’intestazione chiara: Titolo originale (anno) — Regia: Nome; aggiungi il titolo italiano se noto. Per serie e anime indica il formato. Spiega l’attinenza specifica. Se la richiesta non riguarda titoli, adatta la struttura.", "Usa spiegazioni brevi e concrete." if depth == "concise" else "Aggiungi un’analisi storica e formale più approfondita.", "Distingui fatti verificati, attribuzioni discusse e tue inferenze. Se puoi navigare, controlla i riferimenti e cita pagine pertinenti; altrimenti dichiara che non hai verificato online. Non inventare fonti, URL, film, dati o disponibilità sulle piattaforme. Evita spoiler importanti. Se alcuni criteri sono incompatibili, spiegalo prima di proporre alternative."])
    return "\n".join(out)+"\n"
