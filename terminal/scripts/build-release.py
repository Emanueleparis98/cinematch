"""Create universal release artifacts. No platform-specific executable required."""
import hashlib
import json
import shutil
import subprocess
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
DIST = ROOT / "terminal" / "dist" / "release"


def main():
    canonical = (ROOT/"plugins/cinema-currents/assets/movements.json").read_bytes()
    if canonical != (ROOT/"terminal/src/termovies/movements.json").read_bytes():
        raise SystemExit("Il catalogo terminale non coincide con le correnti canoniche del progetto.")
    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True)
    subprocess.run([sys.executable, "-m", "build", "--wheel", "--sdist", "--outdir", str(DIST)], cwd=ROOT, check=True)
    for name in ["install.sh", "install.ps1"]:
        shutil.copy2(ROOT/"terminal"/name, DIST/name)
        # Keep the previous artifact names so older installers keep working.
        shutil.copy2(ROOT/"terminal"/name, DIST/("install-terminal"+Path(name).suffix))
    version = tomllib.loads((ROOT/"pyproject.toml").read_text(encoding="utf-8"))["project"]["version"]
    latest = json.dumps({"version": version}, indent=2)+"\n"
    (DIST/"latest.json").write_text(latest, encoding="utf-8")
    (DIST/"latest-terminal.json").write_text(latest, encoding="utf-8")
    (ROOT/"terminal/latest.json").write_text(latest, encoding="utf-8")
    wheel = next(DIST.glob("*.whl"))
    scoop_zip = DIST/f"Termovies-{version}-scoop.zip"
    with zipfile.ZipFile(scoop_zip, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        info = zipfile.ZipInfo(wheel.name, (2020,1,1,0,0,0))
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        archive.writestr(info, wheel.read_bytes())
    scoop = {
        "version": version,
        "description": "Film, series, anime, TV and cinema prompts in your terminal",
        "homepage": "https://github.com/Emanueleparis98/termovies",
        "license": "MIT", "depends": "main/uv",
        "url": f"https://github.com/Emanueleparis98/termovies/releases/download/terminal-v{version}/{scoop_zip.name}",
        "hash": hashlib.sha256(scoop_zip.read_bytes()).hexdigest(),
        "installer": {"script": [
            '& uv venv --python 3.13 "$dir\\venv"',
            "if ($LASTEXITCODE -ne 0) { throw 'Cannot create Termovies environment' }",
            f'& uv pip install --python "$dir\\venv\\Scripts\\python.exe" "$dir\\{wheel.name}"',
            "if ($LASTEXITCODE -ne 0) { throw 'Cannot install Termovies' }",
        ]},
        "bin": "venv\\Scripts\\termovies.exe",
        "notes": "Install mpv or VLC for video playback. Run: termovies",
        "checkver": {"github": "https://github.com/Emanueleparis98/termovies", "regex": "terminal-v([\\d.]+)"},
    }
    bucket = ROOT/"bucket"
    bucket.mkdir(exist_ok=True)
    (bucket/"termovies.json").write_text(json.dumps(scoop, indent=2)+"\n", encoding="utf-8")
    source = DIST/f"termovies-{version}.tar.gz"
    recipe = (ROOT/"terminal/packaging/arch/PKGBUILD.in").read_text(encoding="utf-8").replace("@VERSION@", version).replace("@SOURCE_SHA256@", hashlib.sha256(source.read_bytes()).hexdigest())
    arch_dir = ROOT/"terminal/build/arch"
    arch_dir.mkdir(parents=True, exist_ok=True)
    (arch_dir/"PKGBUILD").write_text(recipe, encoding="utf-8")
    with tarfile.open(DIST/f"termovies-{version}-arch-build.tar.gz", "w:gz") as archive:
        archive.add(arch_dir/"PKGBUILD", arcname="termovies/PKGBUILD")
    shutil.copy2(arch_dir/"PKGBUILD", DIST/"PKGBUILD")
    lines = [hashlib.sha256(p.read_bytes()).hexdigest()+"  "+p.name for p in sorted(DIST.iterdir()) if p.is_file()]
    checksums = "\n".join(lines)+"\n"
    (DIST/"SHA256SUMS").write_text(checksums, encoding="utf-8")
    (DIST/"SHA256SUMS-terminal.txt").write_text(checksums, encoding="utf-8")
    print(f"Release: {DIST}")


if __name__ == "__main__":
    main()
