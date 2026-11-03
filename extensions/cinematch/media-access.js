import {httpUrl,permissionOrigin} from './streams.js';
export const RD_DOWNLOADS='https://*.download.real-debrid.com/*';
export function mediaPermission(value){
  const u=httpUrl(value);return u.protocol==='https:'&&(u.hostname==='download.real-debrid.com'||u.hostname.endsWith('.download.real-debrid.com'))?RD_DOWNLOADS:permissionOrigin(u.href);
}
export function playbackPermissions(stream){
  const origins=[mediaPermission(stream.url)];
  if(/\bRD\b|real[ -]?debrid/i.test([stream.name,stream.description].filter(Boolean).join(' ')))origins.push(RD_DOWNLOADS);
  return [...new Set(origins)];
}
export function permissionLabel(origin){return origin===RD_DOWNLOADS?'Real-Debrid':httpUrl(origin).hostname;}
function permissionError(value){
  const origin=mediaPermission(value),error=new Error(`Il flusso usa un altro server. Consenti l’accesso a ${permissionLabel(origin)} per leggere il video.`);error.mediaOrigin=origin;return error;
}
function requestKey(value){
  try{const u=httpUrl(value);return u.origin+u.pathname+'?'+u.searchParams.toString();}catch{return '';}
}
// Redirect URLs stay in memory. Only host permission patterns reach the UI.
export async function mediaFetch(url,init={}, {fetcher=fetch,api=globalThis.chrome}={}){
  if(init.signal?.aborted)throw new DOMException('Stopped','AbortError');
  const initial=httpUrl(url).href,keys=new Set([requestKey(initial)]),redirects=[],requests=new Set();let networkCode,finalTarget;
  const hasAccess=async value=>!api?.permissions||await api.permissions.contains({origins:[permissionOrigin(value)]});
  const initialAccess=await hasAccess(initial);
  if(init.signal?.aborted)throw new DOMException('Stopped','AbortError');
  if(!initialAccess)throw permissionError(initial);
  const ours=details=>details.initiator===api.runtime.getURL('').replace(/\/$/,'')&&(keys.has(requestKey(details.url))||requests.has(details.requestId));
  const observe=details=>{
    if(!ours(details))return;requests.add(details.requestId);
    try{finalTarget=httpUrl(details.redirectUrl).href;redirects.push(finalTarget);}catch{}
  };
  const failed=details=>{if(ours(details)&&/^net::ERR_[A-Z0-9_]+$/.test(details.error))networkCode=details.error.slice(5);};
  const event=api?.webRequest?.onBeforeRedirect,errors=api?.webRequest?.onErrorOccurred;
  const filter={urls:['http://*/*','https://*/*'],types:['xmlhttprequest']};
  event?.addListener(observe,filter);errors?.addListener(failed,filter);
  let response,failure;
  const cancel=async()=>{await response?.body?.cancel().catch(()=>{});};
  const checkAbort=async()=>{if(init.signal?.aborted){await cancel();throw new DOMException('Stopped','AbortError');}};
  try{
    try{response=await fetcher(initial,init);}catch(error){failure=error;}
    await checkAbort();
    for(const target of redirects)if(!await hasAccess(target)){await cancel();throw permissionError(target);}
    // A failed/tainted redirect can leave CORS-filtered headers even after an
    // optional grant. Retry the observed destination directly, only if granted.
    if(finalTarget&&finalTarget!==initial&&(failure||(response?.status===206&&!response.headers.get('content-range')))){
      await cancel();networkCode=undefined;failure=null;
      keys.add(requestKey(finalTarget));
      try{response=await fetcher(finalTarget,init);}catch(error){response=null;failure=error;}
      await checkAbort();
    }
    for(const target of redirects)if(!await hasAccess(target)){await cancel();throw permissionError(target);}
    if(response?.url&&!await hasAccess(response.url)){await cancel();throw permissionError(response.url);}
    if(failure){
      if(networkCode&&networkCode!=='ERR_ABORTED'){
        const messages={ERR_NAME_NOT_RESOLVED:'Il nome del server video non viene risolto. Controlla rete o DNS.',ERR_CONNECTION_REFUSED:'Il server video rifiuta la connessione. Aggiorna le sorgenti o riprova.',ERR_CONNECTION_TIMED_OUT:'Il server video non risponde in tempo. Riprova.',ERR_CERT_AUTHORITY_INVALID:'Il certificato del server video non è valido.',ERR_CERT_DATE_INVALID:'Il certificato del server video è scaduto o la data del dispositivo è errata.'};
        throw Object.assign(new Error(`${messages[networkCode]??'Il browser non ha completato la connessione al server video. Aggiorna le sorgenti o verifica rete e accesso al sito.'} (${networkCode})`),{networkCode});
      }
      throw failure;
    }
    return response;
  }finally{event?.removeListener(observe);errors?.removeListener(failed);}
}
