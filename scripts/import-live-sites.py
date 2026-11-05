"""Import the two FMHY live sections as data, never executable provider code.

Usage: python scripts/import-live-sites.py path/to/video.md
Review the resulting diff before releasing an updated host list.
"""
import json
import hashlib
import re
import sys
from datetime import date
from pathlib import Path
from urllib.parse import urlsplit

root = Path(__file__).resolve().parent.parent
text = Path(sys.argv[1]).read_text(encoding='utf-8')
section = None
sites = {}
for line in text.splitlines():
    if line.startswith('## '):
        section = 'tv' if line.endswith('Live TV') else 'sports' if line.endswith('Live Sports') else None
    if not section or not line.startswith('* ') or '**Note**' in line:
        continue
    # Only service names/numbered aliases. Social, status and mirror indexes are
    # retained as metadata; they must not grant scripts access to unrelated sites.
    links = re.findall(r'\[([^]]+)\]\((https?:/{1,2}[^)]+)\)', line)
    primary = None
    for label, url in links:
        label = label.replace('\u2060', '').strip()
        url = re.sub(r'^https:/([^/])', r'https://\1', url)
        if label in ['Mirrors', 'Status', 'Discord', 'Telegram', 'TG', 'X', 'Bypass Blocks', 'Sports Calendars / Resources']:
            continue
        host = urlsplit(url).hostname
        if not host:
            continue
        primary = primary or label
        # Shared platforms are limited to the path cited by FMHY.
        path = urlsplit(url).path if host in ['www.reddit.com', 'redd.it', 'zhangboheng.github.io'] else '/'
        key = host + path
        site = sites.setdefault(key, {'name': primary if label.isdigit() else label, 'url': url, 'host': host, 'path': path, 'sections': []})
        if section not in site['sections']:
            site['sections'].append(section)
document = {'source': 'https://fmhy.net/video#live-tv-sports', 'sourceFile': 'https://github.com/fmhy/edit/blob/main/docs/video.md', 'sourceSha256': hashlib.sha256(text.encode('utf-8')).hexdigest(), 'importedAt': date.today().isoformat(), 'sites': list(sites.values())}
target = root / 'extensions/cinematch/data/live-sites.json'
target.write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'{len(sites)} service hosts imported; no availability or playback claim.')
