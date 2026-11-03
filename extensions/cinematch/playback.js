import {httpUrl} from './streams.js';
import {mediaFetch} from './media-access.js';
const mimes={hevc:['video/mp4; codecs="hvc1.1.6.L93.B0"','video/mp4; codecs="hev1.1.6.L93.B0"'],h264:['video/mp4; codecs="avc1.42E01E"'],ac3:['audio/mp4; codecs="ac-3"'],eac3:['audio/mp4; codecs="ec-3"'],aac:['audio/mp4; codecs="mp4a.40.2"'],dts:['audio/mp4; codecs="dtsc"'],truehd:['audio/mp4; codecs="mlpa"']};
const labels={hevc:'H.265/HEVC',h264:'H.264/AVC',ac3:'AC3',eac3:'E-AC3',aac:'AAC',dts:'DTS',truehd:'TrueHD'};
export function declaredCodecs(stream){
  let path='';try{path=decodeURIComponent(new URL(stream.url).pathname);}catch{}
  const text=[stream.name,stream.description,stream.filename,path].filter(Boolean).join(' ');
  return {video:/\b(?:hevc|[hx][. ]?265)\b/i.test(text)?'hevc':/\b(?:avc|[hx][. ]?264)\b/i.test(text)?'h264':null,
    audio:/\b(?:e[. -]?ac[. -]?3|ddp)\b|\bdd\+/i.test(text)?'eac3':/\b(?:ac[. -]?3|dolby digital)\b/i.test(text)?'ac3':/\btruehd\b/i.test(text)?'truehd':/\bdts(?:[. -]?hd)?\b/i.test(text)?'dts':/\baac\b/i.test(text)?'aac':null};
}
const containers=['mkv','webm','avi','mp4','m4v','mov','flv','ts','mpg','mpeg'];
function pathExtension(value=''){
  try{return decodeURIComponent(new URL(value).pathname).toLowerCase().match(/\.([a-z0-9]{2,5})$/)?.[1]??'';}catch{return '';}
}
// Prefer the bytes actually received, then the addon URL, then the resolved
// CDN URL. Addon endpoints often hide the container behind /play?id=... and
// the final signed CDN URL is the only place where .mkv or .mp4 appears.
export function containerExtension(stream,probe={}){
  if(probe.kind==='matroska')return 'mkv';
  for(const value of [stream?.url,probe?.url]){
    const ext=pathExtension(value);if(containers.includes(ext))return ext;
  }
  const declared=declaredCodecs(stream??{});
  if(declared.video==='hevc'||['ac3','eac3','dts','truehd'].includes(declared.audio))return 'mkv';
  return '';
}
export function likelyMatroska(stream,probe={}){
  const ext=containerExtension(stream,probe);return probe.kind==='matroska'||ext==='mkv'||ext==='webm'||ext==='avi';
}
export function compatibility(codecs,canPlayType){
  const supports=c=>!mimes[c]||mimes[c].some(m=>Boolean(canPlayType(m))),audios=codecs.audios??[codecs.audio].filter(Boolean);
  const unsupported=[...(codecs.video&&!supports(codecs.video)?[codecs.video]:[]),...(audios.length&&!audios.some(supports)?audios:[])];
  return {unsupported,warning:unsupported.length?`Codec dichiarati non supportati da questo browser: ${unsupported.map(c=>labels[c]).join(', ')}. Cerca H.264/AVC con audio AAC.`:'',score:unsupported.length?-1:codecs.video==='h264'&&codecs.audio==='aac'?2:0};
}
export function inspectPrefix(bytes,mime=''){
  const text=new TextDecoder().decode(bytes.subarray(0,4096)).trimStart();
  if(text.startsWith('#EXTM3U'))return {kind:'hls',codecs:{}};
  if(/dash\+xml/i.test(mime)||/^<\?xml[\s\S]*<MPD\b|^<MPD\b/i.test(text))return {kind:'dash',codecs:{}};
  if(/text\/html|application\/json/i.test(mime)||/^<!doctype html|^<html\b/i.test(text))return {kind:'page',codecs:{}};
  const codecs={},audios=[];
  if(bytes[0]===0x1a&&bytes[1]===0x45&&bytes[2]===0xdf&&bytes[3]===0xa3){
    // Bounded EBML CodecID (0x86) inspection, not a complete demuxer.
    const names={'V_MPEGH/ISO/HEVC':['video','hevc'],'V_MPEG4/ISO/AVC':['video','h264'],'A_AC3':['audio','ac3'],'A_EAC3':['audio','eac3'],'A_AAC':['audio','aac'],'A_DTS':['audio','dts'],'A_TRUEHD':['audio','truehd']};
    for(let i=4;i<bytes.length-2;i++)if(bytes[i]===0x86){
      const first=bytes[i+1];let width=1,mask=0x80;while(width<=4&&!(first&mask)){width++;mask>>=1;}if(width>4)continue;
      let length=first&(mask-1);for(let j=1;j<width;j++)length=length*256+bytes[i+1+j];if(length<2||length>40||i+1+width+length>bytes.length)continue;
      const id=new TextDecoder().decode(bytes.subarray(i+1+width,i+1+width+length)),entry=names[id];if(entry){codecs[entry[0]]=entry[1];if(entry[0]==='audio'&&!audios.includes(entry[1]))audios.push(entry[1]);}
    }
    if(audios.length>1)codecs.audios=audios;return {kind:'matroska',codecs};
  }
  return {kind:/mpegurl/i.test(mime)?'hls':'video',codecs};
}
export async function probePlayback(url,{fetcher=mediaFetch,signal,timeout=20000}={}){
  const controller=new AbortController();let timedOut=false;const timer=setTimeout(()=>{timedOut=true;controller.abort()},Math.max(250,Number(timeout)||20000)),abort=()=>controller.abort();signal?.addEventListener('abort',abort,{once:true});if(signal?.aborted)controller.abort();
  try{
    // Addon playback endpoints often reject HEAD or redirect to a CDN.
    const response=await fetcher(httpUrl(url).href,{method:'GET',headers:{Range:'bytes=0-65535'},credentials:'omit',cache:'no-store',referrerPolicy:'no-referrer',redirect:'follow',signal:controller.signal});
    if(!response.ok){const error=new Error(({401:'La sorgente richiede autenticazione. Controlla la configurazione dell’addon.',403:'Il server video ha rifiutato l’accesso (403). Il link può essere scaduto: aggiorna le sorgenti.',404:'Il file video non è disponibile (404). Aggiorna le sorgenti o scegline un’altra.',429:'Il server video richiede una pausa (429). Attendi e riprova.'})[response.status]??`Il server video ha risposto con errore ${response.status}. Riprova o cambia sorgente.`);error.playback=true;throw error;}
    const reader=response.body.getReader(),prefix=new Uint8Array(65536);let size=0;
    try{while(size<prefix.length){const {value,done}=await reader.read();if(done)break;const take=Math.min(value.length,prefix.length-size);prefix.set(value.subarray(0,take),size);size+=take;}}finally{controller.abort();await reader.cancel().catch(()=>{});}
    return {...inspectPrefix(prefix.subarray(0,size),response.headers.get('content-type')??''),url:response.url||url};
  }catch(error){if(error.playback||error.mediaOrigin||error.networkCode)throw error;if(signal?.aborted)throw new DOMException('Aborted','AbortError');return {kind:'unknown',codecs:{},url,timedOut};}
  finally{clearTimeout(timer);signal?.removeEventListener('abort',abort);}
}
export function nativePlaybackError(code){
  return ({2:'La connessione al video si è interrotta. Aggiorna le sorgenti o riprova.',3:'Il browser non riesce a decodificare il file. Scegli H.264/AVC con audio AAC.',4:'Formato non riproducibile o accesso al video bloccato. Prova una sorgente H.264/AVC con audio AAC.'})[code]??'Il video non si avvia. Aggiorna le sorgenti e riprova.';
}
