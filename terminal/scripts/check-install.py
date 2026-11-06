"""Test the actual release installer in a disposable uv environment on each OS."""
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent


def main():
    public = "--public" in sys.argv
    wheel = None if public else next((ROOT/"terminal/dist/release").glob("cinematch-*-py3-none-any.whl"))
    uv = shutil.which("uv") or str(Path(sys.executable).parent / ("uv.exe" if os.name == "nt" else "uv"))
    with tempfile.TemporaryDirectory(prefix="cinematch install ") as directory:
        base = Path(directory)
        installer = ROOT/"terminal"/("install.ps1" if os.name == "nt" else "install.sh")
        env = {**os.environ, "CINEMATCH_UV": uv, "UV_TOOL_DIR": str(base/"tools"),
               "UV_TOOL_BIN_DIR": str(base/"bin"), "UV_CACHE_DIR": str(base/"cache")}
        if public:
            manifest = json.load(urllib.request.urlopen("https://raw.githubusercontent.com/Emanueleparis98/cinematch/main/terminal/latest.json", timeout=30))
            version = manifest["version"]
            assert re.fullmatch(r"\d+\.\d+\.\d+", version)
            wheel = base/f"cinematch-{version}-py3-none-any.whl"
            url = f"https://github.com/Emanueleparis98/cinematch/releases/download/terminal-v{version}/"
            wheel.write_bytes(urllib.request.urlopen(url+wheel.name, timeout=60).read())
            installer = base/installer.name
            installer.write_bytes(urllib.request.urlopen(url+("install-terminal.ps1" if os.name == "nt" else "install-terminal.sh"), timeout=30).read())
        if os.name == "nt":
            command = ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(installer),
                       "-Python", sys.executable, "-NoPath"]
            wheel_flag = "-Wheel"
        else:
            command = ["sh", str(installer), "--python", sys.executable, "--no-path"]
            wheel_flag = "--wheel"
        subprocess.run(command + ([] if public else [wheel_flag, str(wheel)]), env=env, check=True, timeout=240)
        executable = base/"bin"/("cinematch.exe" if os.name == "nt" else "cinematch")
        subprocess.run([str(executable), "--version"], env=env, check=True, timeout=30)
        data = json.loads(subprocess.check_output([str(executable), "currents", "--json"], env=env, timeout=30))
        assert len(data) == 74
        prompt = subprocess.check_output([str(executable), "prompt", "Consigliami un film"], env=env, timeout=30).decode("utf-8")
        assert "Consigliami un film" in prompt
        installed_python = base/"tools/cinematch"/("Scripts/python.exe" if os.name == "nt" else "bin/python")
        code = (ROOT/"terminal/scripts/check-installed-ui.py").read_text(encoding="utf-8")
        subprocess.run([str(installed_python), "-c", code], env=env, cwd=base, check=True, timeout=30)
        # Reject a damaged release before invoking any installer/dependency download.
        corrupted = base/"bad"/wheel.name
        corrupted.parent.mkdir()
        corrupted.write_bytes(wheel.read_bytes()+b"corrupted")
        (corrupted.parent/"SHA256SUMS-terminal.txt").write_text(hashlib.sha256(wheel.read_bytes()).hexdigest()+"  "+wheel.name+"\n")
        bad = command + [wheel_flag, str(corrupted)]
        result = subprocess.run(bad, check=False, env=env, capture_output=True, text=True, timeout=30)
        assert result.returncode != 0 and "Checksum" in result.stdout+result.stderr
        print("PASS: release installed, cinematch command, catalog and installed TUI work; corrupted wheel rejected.")


if __name__ == "__main__":
    main()
