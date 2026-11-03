import {httpUrl,permissionOrigin} from './streams.js';

// Observe only this fetch's redirect chain. Destinations stay in memory and are
// reduced to host permissions before they reach UI; signed paths never do.
export async function mediaFetch(url,init={}, {fetcher=fetch,api=globalThis.chrome}={}){
  const initial=httpUrl(url).href,redirects=[],requests=new Set();let networkCode;
  const ours=details=>details.initiator===api.runtime.getURL('').replace(/\/$/,'')&&(details.url===initial||requests.has(details.requestId));
  const observe=details=>{
    if(!ours(details))return;
    requests.add(details.requestId);
    try{redirects.push(permissionOrigin(details.redirectUrl));}catch{}
  };
  const failed=details=>{if(ours(details)&&/^net::ERR_[A-Z0-9_]+$/.test(details.error))networkCode=details.error.slice(5);};
  const event=api?.webRequest?.onBeforeRedirect;
  const errors=api?.webRequest?.onErrorOccurred;
  event?.addListener(observe,{urls:['http://*/*','https://*/*'],types:['xmlhttprequest']});
  errors?.addListener(failed,{urls:['http://*/*','https://*/*'],types:['xmlhttprequest']});
  let response,failure;
  try{response=await fetcher(initial,init);}catch(error){failure=error;}
  finally{event?.removeListener(observe);errors?.removeListener(failed);}
  if(init.signal?.aborted){await response?.body?.cancel().catch(()=>{});throw failure??new DOMException('Stopped','AbortError');}
  if(api?.permissions){
    if(response?.url)redirects.push(permissionOrigin(response.url));
    for(const origin of new Set(redirects))if(!await api.permissions.contains({origins:[origin]})){
      await response?.body?.cancel().catch(()=>{});
      const error=new Error(`Il flusso usa un altro server (${new URL(origin).hostname}). Consenti l’accesso per continuare.`);
      error.mediaOrigin=origin;throw error;
    }
  }
  if(failure){
    if(networkCode&&networkCode!=='ERR_ABORTED'){
      const messages={ERR_NAME_NOT_RESOLVED:'Il nome del server video non viene risolto. Controlla rete o DNS.',ERR_CONNECTION_REFUSED:'Il server video rifiuta la connessione. Aggiorna le sorgenti o riprova.',ERR_CONNECTION_TIMED_OUT:'Il server video non risponde in tempo. Riprova.',ERR_CERT_AUTHORITY_INVALID:'Il certificato del server video non è valido.',ERR_CERT_DATE_INVALID:'Il certificato del server video è scaduto o la data del dispositivo è errata.'};
      throw Object.assign(new Error(`${messages[networkCode]??'Il browser non ha completato la connessione al server video. Aggiorna le sorgenti o verifica rete e accesso al sito.'} (${networkCode})`),{networkCode});
    }
    throw failure;
  }
  return response;
}
