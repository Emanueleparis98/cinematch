/* Shared, testable live-source parsing. Treat every page and URL as untrusted. */
export const LIVE_TTL = 20 * 60 * 1000;
export const MAX_LIVE_SOURCES = 40;
export const AD_DOMAINS = ['adsco.re','cdn4ads.com','rapidadexchange.com','acscdn.com','doubleclick.net','googlesyndication.com','googleadservices.com','popads.net','popcash.net','propellerads.com','onclickads.net','adcash.com','exoclick.com','adsterra.com','histats.com','google-analytics.com'];
export function publicLiveUrl(value, base) {
  try {
    if (typeof value !== 'string' || value.length > 8192) return null;
    const url = new URL(value.trim(), base);
    const host = url.hostname.toLowerCase();
    if (!['https:','http:'].includes(url.protocol) || url.username || url.password) return null;
    if (!host.includes('.') || host.endsWith('.local') || host.endsWith('.localhost') || host.endsWith('.internal') || host.includes(':') || /^\d+(?:\.\d+){3}$/.test(host)) return null;
    if (AD_DOMAINS.some(domain => host === domain || host.endsWith('.' + domain))) return null;
    url.hash = '';
    return url.href;
  } catch { return null; }
}
export function liveKind(value, mime = '') {
  const url = publicLiveUrl(value); if (!url) return null;
  const path = new URL(url).pathname;
  if (/\.(?:ts|m4s|aac|mp3|vtt|srt|jpg|png|gif|webp)$/i.test(path)) return null;
  if (/\.m3u8?$/i.test(path) || /(?:application|audio)\/(?:vnd\.apple\.mpegurl|x-mpegurl)/i.test(mime)) return 'hls';
  if (/\.mpd$/i.test(path) || /application\/dash\+xml/i.test(mime)) return 'dash';
  if (/\.(?:mp4|webm|mkv|mov)$/i.test(path) || /^video\/(?:mp4|webm|quicktime|x-matroska)(?:;|$)/i.test(mime)) return 'video';
  return null;
}
export function matchLiveSite(value, sites) {
  const safe = publicLiveUrl(value); if (!safe) return null;
  const url = new URL(safe);
  return sites.find(site => {
    const host = site.host.replace(/^www\./, '');
    const prefix = site.host === 'zhangboheng.github.io' ? site.path.replace(/\/[^/]*$/, '/') : site.path.replace(/\/$/,'')+'/';
    return (url.hostname === host || url.hostname === 'www.' + host) && (site.path === '/' || url.pathname === site.path || url.pathname.startsWith(prefix));
  }) ?? null;
}
export function liveOrigins(sites) {
  return [...new Set(sites.flatMap(site => {
    const host = site.host.replace(/^www\./, '');
    return [`https://${host}/*`, `https://www.${host}/*`, `http://${host}/*`, `http://www.${host}/*`];
  }))];
}
export function sourceFromUrl(value, mime = '', base) {
  const url = publicLiveUrl(value, base), kind = url && liveKind(url, mime);
  if (!kind) return null;
  return {url, kind, mime, name: new URL(url).hostname, description: `${kind.toUpperCase()} · rilevato nella pagina, disponibilità da verificare`, subtitles: [], playable: true};
}
function unpackLiveData(text){
  // Decode only the standard literal packer shape, in one bounded pass. Page
  // code is never executed and replacement words cannot expand one another.
  return text.replace(/eval\(function\(p,a,c,k,e,[dr]\)\{[\s\S]*?\}\('([\s\S]*?)',(\d+),(\d+),'([\s\S]*?)'\.split\('\|'\)/g,(original,payload,baseText,countText,dictionary)=>{
    const base=Number(baseText),count=Number(countText);
    if(base<2||base>62||count<1||count>2000||payload.length>128000||dictionary.length>128000)return '';
    const words=dictionary.split('|'),lookup=new Map();
    const token=value=>(value<base?'':token(Math.floor(value/base)))+((value%=base)>35?String.fromCharCode(value+29):value.toString(36));
    for(let i=0;i<count;i++)if(words[i])lookup.set(token(i),words[i]);
    let size=0;
    return payload.replace(/\\'/g,"'").replace(/\\\\/g,'\\').replace(/\b[0-9a-z]+\b/gi,word=>{
      const value=lookup.get(word)??word;size+=value.length;return size<=2*1024*1024?value:'';
    });
  });
}
export function extractLiveUrls(text, base) {
  if (typeof text !== 'string') return [];
  const bounded=text.slice(0,2*1024*1024);
  const decoded=unpackLiveData(bounded);
  const literals=[];
  for(const match of bounded.matchAll(/atob\(\s*['"]([A-Za-z0-9+/=]{12,12000})['"]\s*\)/g)){
    try{literals.push(atob(match[1]));}catch{}
    if(literals.length>=16)break;
  }
  const clean = (decoded+'\n'+literals.join('\n')).replace(/\\\//g, '/').replace(/\\u0026/gi, '&').replace(/&amp;|&#38;|&#x26;/gi, '&');
  const found = new Map();
  for (const match of clean.matchAll(/(?:https?:)?\/\/[^\s"'<>`\\]+|["']((?:\.{0,2}\/)?[^"'<>\s]+\.(?:m3u8?|mpd|mp4|webm)(?:\?[^"'<>\s]*)?)["']/gi)) {
    const source = sourceFromUrl(match[1] ?? match[0], '', base);
    if (source) found.set(source.url, source);
    if (found.size >= MAX_LIVE_SOURCES) break;
  }
  return [...found.values()];
}
export function extractLiveFrames(text, base) {
  if (typeof text !== 'string') return [];
  return [...new Set([...text.slice(0, 2 * 1024 * 1024).matchAll(/<iframe\b[^>]*?\bsrc\s*=\s*["']([^"']+)["']/gi)].map(match => publicLiveUrl(match[1].replace(/&amp;/g, '&'), base)).filter(Boolean))].slice(0, 12);
}
export function uniqueLiveSources(sources) {
  const found = new Map();
  for (const candidate of sources) {
    const source = sourceFromUrl(candidate?.url, candidate?.mime ?? '');
    if (source && !found.has(source.url)) found.set(source.url, source);
    if (found.size >= MAX_LIVE_SOURCES) break;
  }
  return [...found.values()];
}
