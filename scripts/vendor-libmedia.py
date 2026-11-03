"""Verify pinned decoder assets; --restore downloads the exact recorded build."""
from pathlib import Path
import argparse, ast, base64, hashlib, io, json, re, tarfile, urllib.request

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / 'extensions/cinematch/vendor/libmedia'
PACKAGE = 'https://registry.npmjs.org/@libmedia/avplayer/-/avplayer-1.3.1.tgz'
SRI = 'wRELj99DbfS0t9iKkts5p1v/PSlyGkQ7A5Vy/GNTFg4ZiNo+OWTkkW/ioQuoT8seeljqizN0lWYJjJ9Ra32fcA=='

def patch(data):
    for before, after in [(b'thread:w,', b'thread:!1,'), (b'audioWorklet:p,', b'audioWorklet:!1,')]:
        if data.count(before) != 1:
            raise ValueError('Unexpected upstream build: refusing an ambiguous compatibility patch')
        data = data.replace(before, after)
    return data

def verify():
    expected = json.loads((VENDOR / 'integrity.json').read_text(encoding='utf-8-sig'))
    actual = {str(p.relative_to(VENDOR)).replace('\\', '/') for p in VENDOR.rglob('*') if p.suffix in ('.js', '.wasm', '.css')}
    if actual != set(expected):
        raise ValueError('Decoder file list differs from the pinned build')
    for name, digest in expected.items():
        target = (VENDOR / name).resolve()
        if not target.is_relative_to(VENDOR.resolve()) or hashlib.sha256(target.read_bytes()).hexdigest() != digest:
            raise ValueError(f'Decoder integrity mismatch: {name}')
    print(f'libmedia 1.3.1: {len(expected)} pinned resources verified')

def restore():
    data = urllib.request.urlopen(PACKAGE, timeout=60).read()
    if base64.b64encode(hashlib.sha512(data).digest()).decode() != SRI:
        raise ValueError('npm package integrity mismatch')
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive:
            if member.name.startswith('package/dist/esm/') and member.name.endswith('.js'):
                name = Path(member.name).name
                data = archive.extractfile(member).read()
                (VENDOR / name).write_bytes(patch(data) if name == 'avplayer.js' else data)
                if name == '630.avplayer.js':
                    css = ast.literal_eval(re.search(r"const g=('(?:[^'\\]|\\.)*')", data.decode('utf-8')).group(1))
                    (VENDOR / 'ass.css').write_bytes((css+'\n').encode('utf-8'))
    for resource in json.loads((VENDOR / 'wasm-provenance.json').read_text(encoding='utf-8-sig')):
        target = (VENDOR / resource['file']).resolve()
        if not target.is_relative_to(VENDOR.resolve()) or not resource['url'].startswith('https://zhaohappy.github.io/libmedia/dist/'):
            raise ValueError('Invalid decoder provenance')
        data = urllib.request.urlopen(resource['url'], timeout=60).read()
        if not data.startswith(b'\0asm') or hashlib.sha256(data).hexdigest() != resource['sha256']:
            raise ValueError('Published decoder differs from its pinned hash')
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restore', action='store_true')
    if parser.parse_args().restore:
        restore()
    verify()
