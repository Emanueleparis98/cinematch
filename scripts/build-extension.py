"""Build a self-contained unpacked extension and a ZIP, with standard Python only."""
from pathlib import Path
import json
import shutil
import zipfile

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "extensions" / "cinematch"
DEST = ROOT / "dist" / "CineMatch-Chrome"
DEST.mkdir(parents=True, exist_ok=True)
files = []

def copy(source, relative):
    target = DEST / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    files.append(relative)

for source in sorted(SOURCE.rglob("*")):
    if source.is_file():
        copy(source, source.relative_to(SOURCE))
copy(ROOT / "plugins/cinema-currents/assets/movements.json", Path("data/movements.json"))
copy(ROOT / "assets/cinematch.png", Path("assets/cinematch.png"))
for name in ["Fraunces-SemiBold.ttf", "InstrumentSans-Regular.ttf", "InstrumentSans-SemiBold.ttf", "OFL-Fraunces.txt", "OFL-InstrumentSans.txt"]:
    copy(ROOT / "assets/fonts" / name, Path("assets") / name)
manifest = json.loads((DEST / "manifest.json").read_text(encoding="utf-8"))
archive = ROOT / "dist" / f"CineMatch-Chrome-{manifest['version']}.zip"
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as package:
    for relative in files:
        package.write(DEST / relative, str(relative))
print(f"Unpacked: {DEST}")
print(f"ZIP: {archive} ({archive.stat().st_size:,} bytes)")
