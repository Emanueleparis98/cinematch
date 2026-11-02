import {PROVIDERS,providerForUrl} from "./core.js";
import {insertIntoComposer} from "./composer.js";
import {credential,createTmdbClient} from "./tmdb.js";

const storageReady=Promise.all([
  chrome.storage.local.setAccessLevel({accessLevel:"TRUSTED_CONTEXTS"}),
  chrome.storage.session.setAccessLevel({accessLevel:"TRUSTED_CONTEXTS"})
]);
async function getCredential(){
  await storageReady;
  const session=await chrome.storage.session.get("tmdbCredential");
  return session.tmdbCredential??(await chrome.storage.local.get("tmdbCredential")).tmdbCredential??null;
}
const tmdb=createTmdbClient({getCredential});
async function tmdbStatus(){
  await storageReady;
  return {ok:true,configured:Boolean(await getCredential()),automatic:(await chrome.storage.local.get('tmdbAutomatic')).tmdbAutomatic!==false};
}
async function notifyChats(){
  const status=await tmdbStatus();
  const tabs=await chrome.tabs.query({});
  await Promise.allSettled(tabs.filter(tab=>providerForUrl(tab.url)).map(tab=>chrome.tabs.sendMessage(tab.id,{type:'tmdb-preferences-changed',...status})));
}
const extensionPage=(sender,path)=>sender.id===chrome.runtime.id&&sender.url?.split(/[?#]/)[0]===chrome.runtime.getURL(path);
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id||!message?.type?.startsWith("tmdb-"))return false;
  const settings=extensionPage(sender,"options.html");
  const trusted=settings||extensionPage(sender,"panel.html");
  const chat=sender.tab&&sender.frameId===0&&providerForUrl(sender.url);
  if(!trusted&&!chat)return false;
  (async()=>{
    await storageReady;
    if(message.type==="tmdb-status")return tmdbStatus();
    if(message.type==="tmdb-options"){await chrome.runtime.openOptionsPage();return {ok:true};}
    if(message.type==="tmdb-save"&&settings){
      const value=credential(message.credential);
      // Validate before replacing a working credential. Never return it to a tab.
      const candidate=createTmdbClient({getCredential:async()=>value});await candidate.verify();
      if(message.remember){await chrome.storage.local.set({tmdbCredential:value});await chrome.storage.session.remove("tmdbCredential");}
      else {await chrome.storage.session.set({tmdbCredential:value});await chrome.storage.local.remove("tmdbCredential");}
      tmdb.clear();await notifyChats();return {ok:true};
    }
    if(message.type==="tmdb-automatic"&&settings){
      if(typeof message.enabled!=="boolean")return {ok:false,error:"Impostazione non valida."};
      await chrome.storage.local.set({tmdbAutomatic:message.enabled});await notifyChats();return {ok:true};
    }
    if(message.type==="tmdb-remove"&&settings){await chrome.storage.local.remove("tmdbCredential");await chrome.storage.session.remove("tmdbCredential");tmdb.clear();await notifyChats();return {ok:true};}
    if(message.type==="tmdb-search"&&chat)return {ok:true,...await tmdb.search(message.reference)};
    if(message.type==="tmdb-details"&&chat)return {ok:true,item:await tmdb.details(message.kind,message.id)};
    if(message.type==="tmdb-image"&&chat)return {ok:true,data:await tmdb.image(message.path,message.size)};
    return {ok:false,error:"Operazione non disponibile."};
  })().then(respond).catch(error=>respond({ok:false,error:error instanceof Error?error.message:"TMDB non disponibile. Riprova."}));
  return true;
});

chrome.sidePanel.setPanelBehavior({openPanelOnActionClick:true}).catch(()=>{});
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if (sender.id !== chrome.runtime.id || !sender.url?.startsWith(chrome.runtime.getURL("panel.html"))) return false;
  if (message?.type !== "insert-prompt") return false;
  (async()=>{
    const provider = PROVIDERS.find(p=>p.id===message.provider);
    if (!provider || !Number.isInteger(message.windowId) || typeof message.prompt !== "string") return {ok:false,code:"invalid-request"};
    let [tab] = await chrome.tabs.query({active:true,windowId:message.windowId});
    if (tab?.id === sender.tab?.id && Number.isInteger(message.targetTabId)) {
      tab=await chrome.tabs.get(message.targetTabId);
    }
    if (!tab?.id || tab.windowId !== message.windowId || providerForUrl(tab.url)?.id !== provider.id) return {ok:false,code:"wrong-site"};
    // Native rich-text editors require focus. Only the explicit insertion action
    // may activate the chat selected from CineMatch's full-page view.
    if (!tab.active) await chrome.tabs.update(tab.id,{active:true});
    const results = await chrome.scripting.executeScript({target:{tabId:tab.id},func:insertIntoComposer,args:[{provider:provider.id,prompt:message.prompt,mode:message.mode??"empty",draftToken:message.draftToken}]});
    return results[0]?.result ?? {ok:false,code:"no-editor"};
  })().then(respond).catch(()=>respond({ok:false,code:"unavailable"}));
  return true;
});
