import {DEBRID,createDebridClient,torrentHash} from './debrid.js';
export function installDebridService(){
  const page=(sender,path)=>sender.id===chrome.runtime.id&&sender.url?.split(/[?#]/)[0]===chrome.runtime.getURL(path);
  const credential=async()=>{
    const data=await chrome.storage.local.get(['debridProvider','debridKey']);
    if(!DEBRID[data.debridProvider]||!data.debridKey)return null;
    if(!await chrome.permissions.contains({origins:[DEBRID[data.debridProvider].origin]}))return null;
    return {provider:data.debridProvider,key:data.debridKey};
  };
  const active=new Map();
  async function client(value){
    const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(value.key));
    const namespace='debridTask:'+value.provider+':'+Array.from(new Uint8Array(digest),n=>n.toString(16).padStart(2,'0')).join('')+':';
    return createDebridClient({...value,getTask:async hash=>(await chrome.storage.session.get(namespace+hash))[namespace+hash],setTask:async(hash,id)=>chrome.storage.session.set({[namespace+hash]:id})});
  }
  chrome.runtime.onMessage.addListener((message,sender,respond)=>{
    if(!message?.type?.startsWith('debrid-')||sender.id!==chrome.runtime.id)return false;
    const settings=page(sender,'settings-popup.html'),player=page(sender,'cinema.html')||page(sender,'watch.html');
    if(!settings&&!player)return false;
    (async()=>{
      if(message.type==='debrid-status'){const value=await credential();return {ok:true,enabled:Boolean(value),provider:value?.provider??null};}
      if(message.type==='debrid-save'&&settings){
        const {provider,key}=message;
        if(!DEBRID[provider]||!await chrome.permissions.contains({origins:[DEBRID[provider].origin]}))throw new Error('Autorizza il servizio selezionato.');
        await createDebridClient({provider,key}).verify();
        await chrome.storage.local.set({debridProvider:provider,debridKey:key});return {ok:true};
      }
      if(message.type==='debrid-remove'&&settings){await chrome.storage.local.remove(['debridProvider','debridKey']);return {ok:true};}
      if(message.type==='debrid-resolve'&&player){
        const value=await credential();if(!value)throw new Error('Collega Real-Debrid o TorBox nelle impostazioni → Flussi.');
        const stream=message.stream??{};
        const hash=torrentHash(stream);
        const context={season:message.season,episode:message.episode};
        const id=JSON.stringify([value.provider,value.key,hash,stream.fileIdx,context.season,context.episode]);
        if(!active.has(id))active.set(id,client(value).then(api=>api.resolve(stream,context)).finally(()=>active.delete(id)));
        const result=await active.get(id);const current=await credential();
        if(!current||current.provider!==value.provider||current.key!==value.key)throw new Error('Servizio scollegato o modificato. Riapri le sorgenti e riprova.');
        return {ok:true,...result,provider:value.provider};
      }
      throw new Error('Operazione non autorizzata.');
    })().then(respond).catch(error=>respond({ok:false,error:error.message}));return true;
  });
}
