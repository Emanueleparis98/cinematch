import {
  isCb01ArticleUrl,
  isMixdropUrl,
  isPlayerUrl,
  parseCb01Article,
  parseMixdropPage,
  parseStayOnlineResponse,
  safeUrl,
  streamFrom,
  unpackPacker
} from './cb01-resolver.js';

const ARTICLE_TTL = 5 * 60 * 1000;
const MAX_ARTICLE = 2 * 1024 * 1024;
const MAX_PLAYER = 1024 * 1024;
const cache = new Map();

export class Cb01Error extends Error {
  constructor(message, code = 'unavailable', manualUrl = null) {
    super(message);
    this.name = 'Cb01Error';
    this.code = code;
    this.manualUrl = manualUrl;
  }
}

function timeoutError() {
  return new Cb01Error('Il sito sta impiegando troppo tempo a rispondere. Riprova.', 'timeout');
}

async function readLimited(response, maxBytes) {
  if (Number(response.headers.get('content-length')) > maxBytes) throw new Cb01Error('La pagina è troppo grande per essere analizzata.', 'too-large');
  const reader = response.body?.getReader();
  if (!reader) return '';
  const chunks = [];
  let size = 0;
  try {
    for (;;) {
      const {done, value} = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > maxBytes) throw new Cb01Error('La pagina è troppo grande per essere analizzata.', 'too-large');
      chunks.push(value);
    }
  } catch (error) {
    try { await reader.cancel(); } catch {}
    throw error;
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

async function fetchText(url, {maxBytes = MAX_PLAYER, timeout = 12000, allowedHost} = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout);
  try {
    const response = await fetch(url, {
      signal: controller.signal,
      credentials: 'omit',
      cache: 'no-store',
      redirect: 'follow',
      referrerPolicy: 'no-referrer',
      headers: {accept: 'text/html,application/xhtml+xml,application/json;q=0.8'}
    });
    if (!response.ok) throw new Cb01Error(`Il server ha risposto con errore ${response.status}.`, 'http');
    const finalUrl = safeUrl(response.url);
    if (!finalUrl || (allowedHost && !allowedHost(finalUrl))) throw new Cb01Error('Il server ha reindirizzato verso un sito non autorizzato.', 'redirect');
    return await readLimited(response, maxBytes);
  } catch (error) {
    if (error instanceof Cb01Error) throw error;
    if (controller.signal.aborted) throw timeoutError();
    throw new Cb01Error('Il sito non è raggiungibile. Controlla la connessione e riprova.', 'network');
  } finally {
    clearTimeout(timer);
  }
}

async function fetchJson(url, {body, timeout = 12000} = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout);
  try {
    const response = await fetch(url, {
      method: body ? 'POST' : 'GET',
      body,
      signal: controller.signal,
      credentials: 'omit',
      cache: 'no-store',
      redirect: 'follow',
      referrerPolicy: 'no-referrer',
      headers: {accept: 'application/json', ...(body ? {'content-type': 'application/x-www-form-urlencoded'} : {})}
    });
    if (!response.ok) throw new Cb01Error(`Il server ha risposto con errore ${response.status}.`, 'http');
    const text = await readLimited(response, 256 * 1024);
    try { return JSON.parse(text); }
    catch { throw new Cb01Error('Il server non ha restituito JSON valido.', 'json'); }
  } catch (error) {
    if (error instanceof Cb01Error) throw error;
    if (controller.signal.aborted) throw timeoutError();
    throw new Cb01Error('Il server non è raggiungibile. Controlla la connessione e riprova.', 'network');
  } finally {
    clearTimeout(timer);
  }
}

export async function inspectArticle(articleUrl, {refresh = false} = {}) {
  if (!isCb01ArticleUrl(articleUrl)) throw new Cb01Error('La scheda CB01 non è valida.', 'invalid-url');
  const cached = cache.get(articleUrl);
  if (!refresh && cached && Date.now() - cached.at < ARTICLE_TTL) return cached.parsed;
  const html = await fetchText(articleUrl, {
    maxBytes: MAX_ARTICLE,
    timeout: 15000,
    allowedHost: url => url.hostname === 'cb01one.org' || url.hostname === 'www.cb01one.org'
  });
  const parsed = parseCb01Article(html, articleUrl);
  cache.set(articleUrl, {parsed, at: Date.now()});
  return parsed;
}

async function resolveStayOnline(value) {
  const url = safeUrl(value);
  const id = url?.pathname.match(/^\/(?:e|l)\/([A-Za-z0-9]+)\/?$/)?.[1];
  if (!url || !id || url.hostname !== 'stayonline.pro') throw new Cb01Error('Il collegamento StayOnline non è valido.', 'provider');
  const data = await fetchJson('https://stayonline.pro/ajax/linkEmbedView.php', {body: new URLSearchParams({id})});
  try { return parseStayOnlineResponse(data); }
  catch (error) {
    if (data?.code === 429) throw new Cb01Error('StayOnline richiede una verifica prima di continuare. Riprova tra poco.', 'captcha');
    throw new Cb01Error(error.message, 'provider');
  }
}

async function resolveMixdrop(value) {
  const pageUrl = safeUrl(value);
  if (!pageUrl || !isMixdropUrl(pageUrl.href)) throw new Cb01Error('Il collegamento Mixdrop non è valido.', 'provider');
  const html = await fetchText(pageUrl.href, {
    maxBytes: MAX_PLAYER,
    timeout: 12000,
    allowedHost: url => isMixdropUrl(url.href)
  });
  try { return parseMixdropPage(html, pageUrl.href); }
  catch (error) {
    if (/captcha|cf-challenge|just a moment/i.test(html)) throw new Cb01Error('Mixdrop richiede una verifica manuale. Riprova più tardi.', 'captcha');
    throw new Cb01Error(error.message, 'provider');
  }
}

async function resolveUprot(value) {
  const url = safeUrl(value);
  if (!url || !isPlayerUrl(url.href) || url.hostname !== 'uprot.net') throw new Cb01Error('Il collegamento Maxstream non è valido.', 'provider');
  const html = await fetchText(url.href, {
    maxBytes: MAX_PLAYER,
    timeout: 12000,
    allowedHost: target => target.hostname === 'uprot.net'
  });
  if (/UPCaptcha|upcaptcha|captcha/i.test(html)) {
    throw new Cb01Error('Maxstream richiede una verifica manuale. Scegli un’altra sorgente o apri la scheda CB01.', 'captcha', url.href);
  }
  const decoded = unpackPacker(html);
  const candidate = decoded.match(/(?:file|source|src)\s*[:=]\s*["']([^"']+\.(?:mp4|m3u8)[^"']*)["']/i)?.[1]
    ?? decoded.match(/["'](https?:\/\/[^"']+\.(?:mp4|m3u8)[^"']*)["']/i)?.[1];
  if (!candidate) throw new Cb01Error('Maxstream non ha esposto un flusso diretto.', 'provider');
  const media = safeUrl(candidate.startsWith('//') ? `https:${candidate}` : candidate, url.href);
  if (!media || media.protocol !== 'https:' || !isPlayerUrl(media.href)) throw new Cb01Error('Maxstream ha restituito un indirizzo non supportato.', 'provider');
  return {url: media.href, kind: /\.m3u8(?:$|\?)/i.test(media.pathname) ? 'hls' : 'video', filename: '', poster: null};
}

async function resolveSource(source) {
  if (source.provider === 'stayonline') {
    const target = await resolveStayOnline(source.url);
    const targetUrl = safeUrl(target);
    if (!targetUrl) throw new Cb01Error('StayOnline ha restituito un indirizzo non valido.', 'provider');
    if (isMixdropUrl(targetUrl.href)) return resolveMixdrop(targetUrl.href);
    if (targetUrl.hostname === 'uprot.net') return resolveUprot(targetUrl.href);
    throw new Cb01Error('StayOnline ha restituito un provider non supportato.', 'provider');
  }
  if (source.provider === 'uprot') return resolveUprot(source.url);
  throw new Cb01Error('Questo provider non è ancora supportato.', 'provider');
}

export async function resolveSources(parsed, sources, {manualUrl = parsed.articleUrl} = {}) {
  const failures = [];
  for (const source of sources) {
    try {
      const resolved = await resolveSource(source);
      return {stream: streamFrom(parsed, resolved, source), source, failures};
    } catch (error) {
      failures.push({label: source.label, quality: source.quality, code: error.code ?? 'unavailable', message: error.message});
    }
  }
  const captcha = failures.some(item => item.code === 'captcha');
  throw Object.assign(
    new Cb01Error(captcha
      ? 'Nessuna sorgente automatica disponibile: il provider richiede una verifica manuale.'
      : 'Nessuna sorgente CB01 è disponibile in questo momento.', captcha ? 'captcha' : 'unavailable', manualUrl),
    {failures}
  );
}

export async function resolveMovie(parsed) {
  if (!parsed.sources?.length) throw new Cb01Error('Questa scheda non contiene sorgenti streaming.', 'no-sources', parsed.articleUrl);
  return resolveSources(parsed, parsed.sources);
}

export async function resolveEpisode(parsed, seasonNumber, episodeNumber, {manualUrl = parsed.articleUrl} = {}) {
  const season = parsed.seasons?.find(item => item.season === seasonNumber);
  const episode = season?.episodes.find(item => item.episode === episodeNumber);
  if (!episode?.sources?.length) throw new Cb01Error('Questo episodio non contiene sorgenti streaming.', 'no-sources', manualUrl);
  return resolveSources(parsed, episode.sources, {manualUrl});
}

export function clearCb01Cache() {
  cache.clear();
}
