/* Independent implementation of the public Stremio JSON protocol. */
export const MAX_JSON=2*1024*1024;
const text=(value,max=300)=>typeof value==='string'?value.slice(0,max):'';
export function httpUrl(value){
  if(typeof value!=='string'||value.length>16000)throw new Error('Indirizzo non valido.');
  let url;try{url=new URL(value);}catch{throw new Error('Inserisci un indirizzo completo HTTPS.');}
  if(!['https:','http:'].includes(url.protocol)||url.username||url.password)throw new Error('Sono supportati soltanto indirizzi HTTP e HTTPS senza credenziali incorporate.');
  url.hash='';return url;
}
export function manifestUrl(value){
  const url=httpUrl(String(value).trim().replace(/^stremio:\/\//i,'https://'));
  if(/\/(?:app\/)?configure\/?$/i.test(url.pathname))throw new Error('Questo è il configuratore. Aprilo e copia il link di installazione che termina con manifest.json.');
  if(!url.pathname.endsWith('/manifest.json')){
    if(/\.json$/i.test(url.pathname))throw new Error('Per un addon serve manifest.json. Per una lista di flussi usa Importa JSON.');
    url.pathname=url.pathname.replace(/\/$/,'')+'/manifest.json';
  }
  return url.href;
}
export function permissionOrigin(value){const u=httpUrl(value);return `${u.protocol}//${u.hostname}/*`;}
export function parseManifest(data,url){
  if(!data||typeof data!=='object'||!text(data.id)||!text(data.name)||!Array.isArray(data.resources)||!Array.isArray(data.types))throw new Error('Manifest non valido: mancano nome, id, tipi o risorse.');
  if(data.behaviorHints?.configurationRequired)throw new Error('Configura prima questo addon sul suo sito e copia il manifest personalizzato.');
  const resources=data.resources.filter(r=>r==='stream'||r?.name==='stream').map(r=>({types:r==='stream'?data.types:r.types,prefixes:r==='stream'?data.idPrefixes:r.idPrefixes})).map(r=>({types:Array.isArray(r.types)?r.types.filter(t=>t==='movie'||t==='series'):[],prefixes:Array.isArray(r.prefixes)?r.prefixes.filter(p=>typeof p==='string'&&p.length<=60):[]})).filter(r=>r.types.length);
  if(!resources.length)throw new Error('Questo addon non offre flussi per film o serie TV.');
  return {url:manifestUrl(url),id:text(data.id,160),name:text(data.name,100),version:text(data.version,30),description:text(data.description,400),resources,enabled:true};
}
export function resourceUrl(addon,media,episode=null){
  // Prefer IMDb for standard addons; use tmdb: only when explicitly supported.
  const isSeries=media?.type==='tv';
  if(isSeries&&(!Number.isSafeInteger(episode?.season)||episode.season<0||!Number.isSafeInteger(episode?.episode)||episode.episode<1))return null;
  const type=isSeries?'series':'movie';
  const suffix=isSeries?`:${episode.season}:${episode.episode}`:'';
  const candidates=isSeries
    ? [media.imdbId?`${media.imdbId}${suffix}`:null,`tmdb:${media.id}${suffix}`].filter(Boolean)
    : [media.imdbId,`tmdb:${media.id}`].filter(Boolean);
  const id=candidates.find(candidate=>addon.resources.some(resource=>resource.types.includes(type)&&(!resource.prefixes.length?candidate.startsWith('tt'):resource.prefixes.some(prefix=>candidate.startsWith(prefix)))));
  if(!id)return null;
  const url=httpUrl(addon.url);url.pathname=url.pathname.slice(0,-'manifest.json'.length)+`stream/${type}/${encodeURIComponent(id)}.json`;return url.href;
}
export function parseStreams(data){
  const list=Array.isArray(data)?data:data?.streams;
  if(!Array.isArray(list))throw new Error('Il JSON deve contenere una lista streams.');
  if(list.length>500)throw new Error('La lista supera 500 flussi. Importa una lista più piccola.');
  const seen=new Set();return list.flatMap((s,index)=>{
    if(!s||typeof s!=='object')return [];
    let url=null,externalUrl=null;try{if(s.url)url=httpUrl(s.url).href;}catch{}
    try{if(s.externalUrl)externalUrl=httpUrl(s.externalUrl).href;}catch{}
    const torrent=typeof s.infoHash==='string'||/^magnet:/i.test(s.url??'');
    const headers=s.behaviorHints?.proxyHeaders;
    const needsHeaders=Boolean(headers&&((headers.request&&Object.keys(headers.request).length)||(headers.response&&Object.keys(headers.response).length)));
    const dash=Boolean(url&&/\.mpd(?:\?|$)/i.test(url));
    const kind=url?(needsHeaders?'headers':dash?'dash':/\.m3u8(?:\?|$)/i.test(url)?'hls':'video'):externalUrl?'external':torrent?'torrent':'unsupported';
    const key=url??externalUrl??(torrent?`${s.infoHash??s.url}:${s.fileIdx??''}`:`unknown:${index}`);if(seen.has(key))return [];seen.add(key);
    const reasons={headers:'Richiede un proxy configurato nell’addon per gli header video.',dash:'DASH non supportato da questo player. Scegli una sorgente MP4 o HLS.',torrent:'Torrent: configura nell’addon un servizio che restituisca un flusso HTTP.',unsupported:'Questo tipo di sorgente non è riproducibile nel browser.'};
    return [{name:text(s.name,100)||'Sorgente video',description:text(s.description??s.title,700),filename:text(s.behaviorHints?.filename,300),url,externalUrl,kind,playable:['video','hls','dash'].includes(kind),reason:kind==='dash'?'':reasons[kind]??'',subtitles:(Array.isArray(s.subtitles)?s.subtitles:[]).slice(0,30).flatMap(sub=>{try{return [{url:httpUrl(sub.url).href,lang:text(sub.lang,30)||'Sottotitoli'}];}catch{return [];}})}];
  });
}
export async function fetchJson(url,{signal,fetcher=fetch}={}){
  const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),20000);
  const abort=()=>controller.abort();signal?.addEventListener('abort',abort,{once:true});if(signal?.aborted)controller.abort();
  try{
    const response=await fetcher(httpUrl(url).href,{signal:controller.signal,credentials:'omit',cache:'no-store',redirect:'error',referrerPolicy:'no-referrer',headers:{accept:'application/json'}});
    if(!response.ok)throw new Error(`Il server ha risposto con errore ${response.status}.`);
    if(Number(response.headers.get('content-length'))>MAX_JSON)throw new Error('JSON troppo grande (massimo 2 MB).');
    const reader=response.body.getReader();let length=0;const chunks=[];
    try{for(;;){const {value,done}=await reader.read();if(done)break;length+=value.byteLength;if(length>MAX_JSON)throw new Error('JSON troppo grande (massimo 2 MB).');chunks.push(value);}}catch(error){await reader.cancel().catch(()=>{});throw error;}
    const bytes=new Uint8Array(length);let offset=0;for(const c of chunks){bytes.set(c,offset);offset+=c.length;}
    try{return JSON.parse(new TextDecoder().decode(bytes));}catch{throw new Error('Il server non ha restituito JSON valido. Usa il manifest, non la pagina di configurazione.');}
  }catch(error){if(controller.signal.aborted)throw new Error('Richiesta interrotta o server troppo lento. Riprova.');if(error instanceof TypeError)throw new Error('Server non raggiungibile: controlla indirizzo, accesso al sito e possibili reindirizzamenti.');throw error;}
  finally{clearTimeout(timer);signal?.removeEventListener('abort',abort);}
}
