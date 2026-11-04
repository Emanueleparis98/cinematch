/* Pure parsers for the CB01 pages. No network, no DOM, no remote code.
   The site exposes ordinary links; this module only reads them and the
   standard Mixdrop packer without ever executing fetched JavaScript. */

export const CB01_HOSTS = ['cb01one.org', 'www.cb01one.org'];
export const PLAYER_HOSTS = ['stayonline.pro', 'uprot.net'];
export const MIXDROP_HOSTS = ['m1xdrop.net', 'mxdrop.click', 'mixdrop.co', 'mixdrop.ag', 'mxdrop.to', 'mixdrop.to'];
export const MEDIA_HOSTS = ['mxcontent.net'];

const decodeEntities = value => String(value)
  .replace(/&#0*38;|&amp;/gi, '&')
  .replace(/&#0*39;|&#x27;|&apos;/gi, "'")
  .replace(/&quot;|&#0*34;/gi, '"')
  .replace(/&lt;/gi, '<')
  .replace(/&gt;/gi, '>')
  .replace(/&nbsp;/gi, ' ')
  .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(Number(code)))
  .replace(/&#x([0-9a-f]+);/gi, (_, code) => String.fromCharCode(parseInt(code, 16)));

export const textOf = value => decodeEntities(String(value ?? '').replace(/<[^>]*>/g, ' ')).replace(/\s+/g, ' ').trim();

export function safeUrl(value, base = 'https://cb01one.org/') {
  if (typeof value !== 'string' || value.length > 16000) return null;
  try {
    const url = new URL(decodeEntities(value.trim()), base);
    if (!['https:', 'http:'].includes(url.protocol) || url.username || url.password) return null;
    url.hash = '';
    return url;
  } catch { return null; }
}

export function isCb01ArticleUrl(value) {
  const url = safeUrl(value);
  if (!url || url.protocol !== 'https:' || !CB01_HOSTS.includes(url.hostname)) return false;
  const path = url.pathname;
  if (!path || path === '/' || path.length > 320) return false;
  if (/^\/(?:wp-admin|wp-content|wp-includes|login|register|lostpassword|feed|category|tag|page|author)(?:\/|$)/i.test(path)) return false;
  if (/^\/(?:serietv\/)?(?:category|top-rated|page|feed)(?:\/|$)/i.test(path)) return false;
  return /^\/[a-z0-9][a-z0-9/_\-\.]*$/i.test(path);
}

export function isPlayerUrl(value) {
  const url = safeUrl(value);
  return Boolean(url && url.protocol === 'https:' && PLAYER_HOSTS.includes(url.hostname));
}

export function isMixdropUrl(value) {
  const url = safeUrl(value);
  return Boolean(url && url.protocol === 'https:' && (MIXDROP_HOSTS.includes(url.hostname) || MIXDROP_HOSTS.some(host => url.hostname.endsWith(`.${host}`))));
}

export function isAllowedMediaUrl(value) {
  const url = safeUrl(value);
  if (!url || url.protocol !== 'https:') return false;
  if (!MEDIA_HOSTS.some(host => url.hostname === host || url.hostname.endsWith(`.${host}`))) return false;
  return /\.(?:mp4|m3u8|webm)(?:$|\?)/i.test(url.pathname + url.search);
}

function absoluteImage(value, articleUrl) {
  const url = safeUrl(value, articleUrl);
  if (!url || url.protocol !== 'https:') return null;
  if (!/^\/[a-z0-9][a-z0-9/_\-\.]*\.(?:jpg|jpeg|png|webp)$/i.test(url.pathname)) return null;
  return url.href;
}

function metaContent(html, property) {
  const escaped = property.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const direct = html.match(new RegExp(`<meta[^>]+(?:property|name)=["']${escaped}["'][^>]+content=["']([^"']+)["']`, 'i'));
  if (direct) return decodeEntities(direct[1]);
  const reversed = html.match(new RegExp(`<meta[^>]+content=["']([^"']+)["'][^>]+(?:property|name)=["']${escaped}["']`, 'i'));
  return reversed ? decodeEntities(reversed[1]) : null;
}

function pageTitle(html) {
  const title = metaContent(html, 'og:title')
    ?? html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/i)?.[1]
    ?? html.match(/<title[^>]*>([\s\S]*?)<\/title>/i)?.[1]
    ?? '';
  return textOf(title)
    .replace(/\s+Streaming\s*[-–—].*$/i, '')
    .replace(/\s*[-–—]\s*(?:FILM|SERIE TV)\s+GRATIS.*$/i, '')
    .trim();
}

function splitYear(title) {
  const match = title.match(/\((\d{4})\)/);
  return {title: title.replace(/\s*\[(?:HD|4K|UHD)\]\s*/gi, ' ').replace(/\s*\((\d{4})\)\s*$/, '').replace(/\s+/g, ' ').trim(), year: match ? match[1] : ''};
}

function seriesTitle(title) {
  return title
    .replace(/\s*[-–—]\s*Stagione\s+\d+.*$/i, '')
    .replace(/\s*[-–—]\s*\d+\s*[×x]\s*\d+.*$/i, '')
    .replace(/\s*[-–—]\s*COMPLETA.*$/i, '')
    .replace(/\s*[-–—]\s*ITA.*$/i, '')
    .replace(/\s*\[(?:HD|4K|UHD)\]\s*/gi, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function posterFrom(html, articleUrl) {
  const direct = html.match(/<img[^>]+class=["'][^"']*responsive-locandina[^"']*["'][^>]*>/i)?.[0]
    ?? html.match(/<img[^>]+src=["']([^"']+)["'][^>]*class=["'][^"']*responsive-locandina[^"']*["'][^>]*>/i)?.[0];
  const src = direct?.match(/\bsrc=["']([^"']+)["']/i)?.[1] ?? direct?.match(/\bdata-src=["']([^"']+)["']/i)?.[1];
  return absoluteImage(src ?? metaContent(html, 'og:image'), articleUrl);
}

function providerFor(url) {
  try {
    const host = new URL(url).hostname;
    if (host === 'stayonline.pro' || host.endsWith('.stayonline.pro')) return 'stayonline';
    if (host === 'uprot.net' || host.endsWith('.uprot.net')) return 'uprot';
  } catch {}
  return 'unknown';
}

function qualityFor(url, fallback = '') {
  try {
    const path = new URL(url).pathname.toLowerCase();
    if (/\/mse\/|\/e\//.test(path)) return 'HD';
    if (/\/msf\/|\/l\//.test(path)) return 'SD';
  } catch {}
  return fallback;
}

function labelFor(provider, text = '') {
  const value = textOf(text);
  if (/mixdrop/i.test(value)) return 'Mixdrop';
  if (/maxstream/i.test(value)) return 'Maxstream';
  return provider === 'stayonline' ? 'Mixdrop' : provider === 'uprot' ? 'Maxstream' : value || 'Sorgente';
}

function rankSource(source) {
  const provider = source.provider;
  const label = source.label;
  const quality = source.quality;
  return (quality === 'HD' ? 40 : quality === 'SD' ? 10 : 0)
    + (/mixdrop/i.test(label) ? 30 : /maxstream/i.test(label) ? 20 : 0)
    + (provider === 'stayonline' ? 8 : 0)
    + (source.kind === 'data' ? 2 : 0);
}

function sourceFrom(url, label, kind, fallbackQuality) {
  const normalized = safeUrl(url);
  if (!normalized) return null;
  // Some episode links are published over HTTP; the HTTPS endpoint is the
  // same and is required by the extension.
  if (normalized.protocol === 'http:' && PLAYER_HOSTS.includes(normalized.hostname)) normalized.protocol = 'https:';
  if (normalized.protocol !== 'https:') return null;
  const href = normalized.href;
  if (!isPlayerUrl(href)) return null;
  const provider = providerFor(href);
  return {
    url: href,
    provider,
    label: labelFor(provider, label),
    quality: qualityFor(href, fallbackQuality),
    kind
  };
}

export function parseMovieSources(html) {
  const sources = [];
  for (const match of html.matchAll(/data-src=["'](https?:\/\/[^"']+)["']/gi)) {
    const source = sourceFrom(match[1], '', 'data', 'HD');
    if (source) sources.push(source);
  }
  for (const match of html.matchAll(/<a\b[^>]*href=["'](https?:\/\/[^"']+)["'][^>]*>([\s\S]*?)<\/a>/gi)) {
    const href = match[1];
    if (!isPlayerUrl(href)) continue;
    const source = sourceFrom(href, match[2], 'link', qualityFor(href, 'SD'));
    if (source) sources.push(source);
  }
  const seen = new Set();
  return sources
    .map(source => ({...source, rank: rankSource(source)}))
    .sort((a, b) => b.rank - a.rank)
    .filter(source => {
      if (seen.has(source.url)) return false;
      seen.add(source.url);
      return true;
    });
}

export function parseTvSeasons(html) {
  const heads = [...html.matchAll(/<div[^>]+class=["'][^"']*\bsp-head\b[^"']*["'][^>]*>([\s\S]*?)<\/div>/gi)];
  if (!heads.length) return [];
  const seasons = [];
  for (let index = 0; index < heads.length; index++) {
    const head = heads[index];
    const label = textOf(head[1]);
    const seasonNumber = Number(label.match(/STAGIONE\s+(\d+)/i)?.[1]);
    if (!Number.isSafeInteger(seasonNumber) || seasonNumber < 0) continue;
    const start = head.index ?? 0;
    const end = heads[index + 1]?.index ?? html.length;
    const body = html.slice(start, end);
    const seasonQuality = /HD/i.test(label) ? 'HD' : /SD/i.test(label) ? 'SD' : 'HD';
    const episodes = [];
    for (const row of body.matchAll(/<p\b[^>]*>([\s\S]*?)<\/p>/gi)) {
      const rowText = textOf(row[1]);
      const number = rowText.match(/(\d+)\s*[×x]\s*(\d+)/);
      if (!number) continue;
      const episodeNumber = Number(number[2]);
      if (!Number.isSafeInteger(episodeNumber) || episodeNumber < 1) continue;
      const links = [...row[1].matchAll(/<a\b[^>]*href=["'](https?:\/\/[^"']+)["'][^>]*>([\s\S]*?)<\/a>/gi)]
        .map(link => sourceFrom(link[1], link[2], 'link', seasonQuality))
        .filter(Boolean)
        .map(source => ({...source, quality: seasonQuality, rank: rankSource({...source, quality: seasonQuality})}))
        .sort((a, b) => b.rank - a.rank);
      const seen = new Set();
      const unique = links.filter(source => {
        if (seen.has(source.url)) return false;
        seen.add(source.url);
        return true;
      });
      if (!unique.length) continue;
      episodes.push({episode: episodeNumber, title: '', sources: unique});
    }
    if (episodes.length) {
      episodes.sort((a, b) => a.episode - b.episode);
      seasons.push({season: seasonNumber, name: label || `Stagione ${seasonNumber}`, episodes});
    }
  }
  return seasons.sort((a, b) => a.season - b.season);
}

export function parseCb01Article(html, articleUrl) {
  if (typeof html !== 'string' || html.length < 100) throw new Error('La pagina CB01 non contiene dati leggibili.');
  const url = safeUrl(articleUrl);
  if (!url || !CB01_HOSTS.includes(url.hostname)) throw new Error('Indirizzo CB01 non valido.');
  const isTv = /^\/serietv\//i.test(url.pathname) || /class=["'][^"']*\bsp-head\b/i.test(html);
  const rawTitle = pageTitle(html);
  const poster = posterFrom(html, url.href);
  if (isTv) {
    const {year} = splitYear(rawTitle);
    const title = seriesTitle(splitYear(rawTitle).title) || 'Serie TV';
    const seasons = parseTvSeasons(html);
    return {
      kind: 'tv',
      articleUrl: url.href,
      title,
      originalTitle: rawTitle,
      year,
      poster,
      seasons,
      sourceCount: seasons.reduce((total, season) => total + season.episodes.reduce((sum, episode) => sum + episode.sources.length, 0), 0)
    };
  }
  const {title, year} = splitYear(rawTitle);
  const sources = parseMovieSources(html);
  return {
    kind: 'movie',
    articleUrl: url.href,
    title: title || 'Film',
    originalTitle: rawTitle,
    year,
    poster,
    sources,
    sourceCount: sources.length
  };
}

export function publicMedia(parsed) {
  const base = {
    kind: parsed.kind,
    title: parsed.title,
    originalTitle: parsed.originalTitle,
    year: parsed.year,
    poster: parsed.poster,
    key: parsed.articleUrl
  };
  if (parsed.kind !== 'tv') return base;
  return {
    ...base,
    seasons: parsed.seasons.map(season => ({
      season: season.season,
      name: season.name,
      episodes: season.episodes.map(episode => ({episode: episode.episode, title: episode.title}))
    }))
  };
}

export function defaultEpisode(parsed) {
  if (parsed.kind !== 'tv' || !parsed.seasons.length) return null;
  const season = [...parsed.seasons].sort((a, b) => b.season - a.season).find(item => item.episodes.length);
  if (!season) return null;
  const episode = season.episodes[0];
  return {season: season.season, episode: episode.episode};
}

export function episodeFor(parsed, seasonNumber, episodeNumber) {
  const season = parsed.seasons?.find(item => item.season === seasonNumber);
  return season?.episodes.find(item => item.episode === episodeNumber) ?? null;
}

export function parseStayOnlineResponse(data) {
  if (!data || typeof data !== 'object') throw new Error('StayOnline ha restituito una risposta non valida.');
  if (data.status === 'error') throw new Error(textOf(data.message) || 'StayOnline non ha restituito il flusso.');
  const value = data.data?.value ?? data.value;
  const url = safeUrl(value);
  if (!url || url.protocol !== 'https:') throw new Error('StayOnline non ha restituito un indirizzo valido.');
  return url.href;
}

/* Decode the standard Dean Edwards packer used by Mixdrop. This is a pure
   string transformation: the fetched code is never evaluated. */
export function unpackPacker(source) {
  if (typeof source !== 'string' || !source.includes('eval(function(p,a,c,k,e,d)')) return source;
  const blockPattern = /eval\(function\(p,a,c,k,e,d\)\{[\s\S]*?\}\('([\s\S]*?)',(\d+),(\d+),'([\s\S]*?)'\.split\('\|'\)/g;
  return source.replace(blockPattern, (_, payload, baseText, countText, wordsText) => {
    const base = Number(baseText), count = Number(countText);
    if (!Number.isSafeInteger(base) || base < 2 || !Number.isSafeInteger(count) || count < 1 || count > 2000) return '';
    const words = wordsText.split('|');
    const name = value => (value < base ? '' : name(Math.floor(value / base))) + ((value %= base) > 35 ? String.fromCharCode(value + 29) : value.toString(36));
    let decoded = payload.replace(/\\'/g, "'").replace(/\\\\/g, '\\');
    for (let index = count - 1; index >= 0; index--) {
      if (!words[index]) continue;
      const token = name(index);
      decoded = decoded.replace(new RegExp(`\\b${token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`, 'g'), () => words[index]);
    }
    return decoded;
  });
}

export function parseMixdropPage(html, pageUrl) {
  if (typeof html !== 'string' || html.length < 100) throw new Error('La pagina Mixdrop non contiene dati leggibili.');
  const decoded = unpackPacker(html);
  const wurl = decoded.match(/MDCore\.wurl\s*=\s*["']([^"']+)["']/i)?.[1]
    ?? decoded.match(/MDCore\.furl\s*=\s*["']([^"']+)["']/i)?.[1]
    ?? decoded.match(/["']file["']\s*:\s*["']([^"']+\.(?:mp4|m3u8)[^"']*)["']/i)?.[1]
    ?? decoded.match(/<source[^>]+src=["']([^"']+)["']/i)?.[1];
  if (!wurl) throw new Error('Mixdrop non ha esposto un flusso diretto.');
  const url = safeUrl(wurl.startsWith('//') ? `https:${wurl}` : wurl, pageUrl);
  if (!url || !isAllowedMediaUrl(url.href)) throw new Error('Mixdrop ha restituito un indirizzo video non supportato.');
  const filename = decoded.match(/MDCore\.vfile\s*=\s*["']([^"']+)["']/i)?.[1] ?? '';
  const poster = decoded.match(/MDCore\.poster\s*=\s*["']([^"']+)["']/i)?.[1] ?? '';
  const posterUrl = poster ? safeUrl(poster.startsWith('//') ? `https:${poster}` : poster, pageUrl) : null;
  return {
    url: url.href,
    kind: /\.m3u8(?:$|\?)/i.test(url.pathname) ? 'hls' : 'video',
    filename: textOf(filename).slice(0, 300),
    poster: posterUrl?.protocol === 'https:' ? posterUrl.href : null
  };
}

export function streamFrom(parsed, resolved, source) {
  const label = `${source.label}${source.quality ? ` ${source.quality}` : ''}`.trim();
  const filename = /^[a-f0-9]{16,}\.(?:mp4|mkv|m3u8)$/i.test(resolved.filename ?? '')
    ? `${parsed.title}${source.quality ? ` · ${source.quality}` : ''}`
    : resolved.filename ?? '';
  return {
    name: label || 'CB01',
    description: `CB01 · ${label || 'Sorgente'}`,
    filename,
    url: resolved.url,
    kind: resolved.kind || 'video',
    playable: true,
    groupName: 'CB01',
    subtitles: [],
    direct: true,
    crossOrigin: false,
    mediaKey: parsed.articleUrl
  };
}
