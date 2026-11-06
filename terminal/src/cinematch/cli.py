from __future__ import annotations

import argparse
import asyncio
import getpass
import json
import sys
from dataclasses import asdict

from . import __version__
from .models import CineError
from .net import Network
from .prompts import generate, search_currents
from .service import Cinema
from .store import Store


def parser():
    p = argparse.ArgumentParser(prog="cinematch", description="Cinema, serie, anime, TV e prompt dal terminale.")
    p.add_argument("--version", action="version", version=f"CineMatch Terminal {__version__}")
    commands = p.add_subparsers(dest="command")
    search = commands.add_parser("search", help="Cerca nei cataloghi")
    search.add_argument("query", nargs="?", default="")
    search.add_argument("--type", choices=["all", "movie", "series", "anime", "tv"], default="all")
    search.add_argument("--source", choices=["all", "addons", "tmdb"], default="all")
    search.add_argument("--json", action="store_true")
    currents = commands.add_parser("currents", help="Le 74 correnti, disponibili offline")
    currents.add_argument("query", nargs="?", default="")
    currents.add_argument("--json", action="store_true")
    prompt = commands.add_parser("prompt", help="Genera un prompt locale")
    prompt.add_argument("question", nargs="?", default="")
    prompt.add_argument("--mode", choices=["free", "recommendations", "introduction", "discoveries", "compare"], default="free")
    prompt.add_argument("--current", action="append", default=[])
    prompt.add_argument("--count", type=int)
    prompt.add_argument("--detailed", action="store_true")
    commands.add_parser("doctor", help="Controlla player, percorsi e servizi senza mostrare chiavi")
    addons = commands.add_parser("addons", help="Gestisci addon; usa l’interfaccia per link personali")
    addons.add_argument("action", choices=["list", "add", "remove"])
    addons.add_argument("value", nargs="?", help="Indice da rimuovere; per aggiungere il link viene richiesto in modo nascosto")
    commands.add_parser("configure", help="Inserisci chiavi senza lasciarle nella cronologia della shell")
    return p


async def run(args):
    if args.command == "currents":
        items = search_currents(args.query)
        if args.json:
            print(json.dumps(items, ensure_ascii=False, indent=2))
        else:
            for m in items:
                print(f"{m['id']:28} {m['label']} ({m['start']}–{m['end'] or 'oggi'})")
        return
    if args.command == "prompt":
        print(generate(mode=args.mode, movements=args.current, question=args.question, count=args.count, depth="detailed" if args.detailed else "concise"))
        return
    store = Store()
    service = Cinema(Network(), store)
    try:
        if args.command == "doctor":
            print(f"CineMatch Terminal {__version__}\nConfigurazione: {store.config_path}\nLibreria: {store.data_path}\nPlayer: {service.player.available() or 'non trovato'}")
            print(f"TMDB: {'configurato' if store.config['tmdb_key'] else 'facoltativo, non configurato'}\nDebrid: {store.config['debrid_provider'] or 'non configurato'}\nAddon personali: {len(store.config['addons'])}\nCorrenti: {len(search_currents())}")
        elif args.command == "configure":
            print("Invio conserva il valore attuale. '-' rimuove la credenziale.")
            for field, label in [("tmdb_key", "TMDB"), ("debrid_key", "Chiave debrid")]:
                value = getpass.getpass(label+": ")
                if value:
                    store.config[field] = "" if value == "-" else value.strip()
            provider = input("Servizio [rd / torbox / nessuno; Invio conserva]: ").strip()
            if provider:
                if provider not in {"rd", "torbox", "nessuno"}:
                    raise CineError("Scegli rd, torbox oppure nessuno.")
                store.config["debrid_provider"] = "" if provider == "nessuno" else provider
            store.save_config()
            print("Impostazioni salvate.")
        elif args.command == "addons":
            if args.action == "add":
                if args.value:
                    raise CineError("Per evitare chiavi nella cronologia, usa cinematch addons add senza il link: lo chiederò in modo nascosto.")
                addon = await service.addons.add(getpass.getpass("Manifest URL: "))
                print("Aggiunto: "+addon["name"])
            elif args.action == "remove":
                try:
                    idx = int(args.value)-1
                    if idx < 0:
                        raise ValueError()
                    store.config["addons"].pop(idx)
                except (ValueError, TypeError, IndexError):
                    raise CineError("Indica l’indice mostrato da cinematch addons list.") from None
                store.save_config()
            else:
                for i, addon in enumerate(store.config["addons"], 1):
                    print(f"{i}. {addon['name']} · {'attivo' if addon.get('enabled', True) else 'spento'}")
        elif args.command == "search":
            await service.addons.initialize()
            items = await service.search(args.query, args.type, args.source)
            if args.json:
                # Do not export personalized manifests or signed stream URLs.
                print(json.dumps([{k: v for k, v in asdict(m).items() if k not in {"addon_url", "url"}} for m in items], ensure_ascii=False, indent=2))
            else:
                for m in items:
                    print(f"{m.title} ({m.year}) · {m.type} · {m.source}")
                for error in service.errors:
                    print(error, file=sys.stderr)
        else:
            from .app import CinemaApp
            await CinemaApp(service).run_async()
    finally:
        await service.close()


def main():
    for handle in (sys.stdout, sys.stderr):
        if hasattr(handle, "reconfigure"):
            handle.reconfigure(encoding="utf-8")
    args = parser().parse_args()
    try:
        asyncio.run(run(args))
    except (CineError, OSError) as e:
        print(str(e) if isinstance(e, CineError) else "Operazione non riuscita: verifica l’accesso ai file locali e il player.", file=sys.stderr)
        raise SystemExit(1)
    except KeyboardInterrupt:
        raise SystemExit(130)


if __name__ == "__main__":
    main()
