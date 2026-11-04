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
  return {ok:true,configured:Boolean(await getCredential()),automatic:true};
}
const TMDB_SITE_SCRIPT_ID='cinematch-tmdb-site';
async function tmdbSiteState(){
  const [stored,allowed]=await Promise.all([chrome.storage.local.get('tmdbSiteButton'),chrome.permissions.contains({origins:[TMDB_SITE_ORIGIN]})]);
  return {requested:stored.tmdbSiteButton===true,allowed,enabled:stored.tmdbSiteButton===true&&allowed};
}
async function unregisterTmdbSite(){
  try{await chrome.scripting.unregisterContentScripts({ids:[TMDB_SITE_SCRIPT_ID]});}catch{}
}
async function registerTmdbSite(){
  await unregisterTmdbSite();
  await chrome.scripting.registerContentScripts([{id:TMDB_SITE_SCRIPT_ID,matches:[TMDB_SITE_ORIGIN],js:['overlay.js','tmdb-site.js'],css:['tmdb-site.css'],runAt:'document_idle',persistAcrossSessions:true}]);
}
async function syncTmdbSite({inject=false}={}){
  const state=await tmdbSiteState();
  if(!state.enabled){await unregisterTmdbSite();return state;}
  await registerTmdbSite();
  if(inject){
    const tabs=await chrome.tabs.query({url:'https://www.themoviedb.org/*'});
    const installation=Promise.allSettled(tabs.flatMap(tab=>[
      chrome.scripting.insertCSS({target:{tabId:tab.id},files:['tmdb-site.css']}),
      chrome.scripting.executeScript({target:{tabId:tab.id},files:['overlay.js','tmdb-site.js']})
    ]));
    await Promise.race([installation,new Promise(resolve=>setTimeout(resolve,2500))]);
    await Promise.allSettled(tabs.map(tab=>chrome.tabs.sendMessage(tab.id,{type:'cinematch:tmdb-site-enabled'})));
  }
  return state;
}
async function disableTmdbSite(){
  const tabs=await chrome.tabs.query({url:'https://www.themoviedb.org/*'}).catch(()=>[]);
  const cleanup=Promise.allSettled(tabs.flatMap(tab=>[
    chrome.tabs.sendMessage(tab.id,{type:'cinematch:tmdb-site-disabled'}),
    chrome.scripting.removeCSS({target:{tabId:tab.id},files:['tmdb-site.css']})
  ]));
  // A closing tab must never keep the Options toggle stuck.
  await Promise.race([cleanup,new Promise(resolve=>setTimeout(resolve,1500))]);
  if((await tmdbSiteState()).enabled){await syncTmdbSite({inject:true});return;}
  await unregisterTmdbSite();
}
async function notifyChats(){
  const status=await tmdbStatus();
  const tabs=await chrome.tabs.query({});
  const messages=tabs.filter(tab=>providerForUrl(tab.url)).map(tab=>chrome.tabs.sendMessage(tab.id,{type:'tmdb-preferences-changed',...status}));
  if((await tmdbSiteState()).enabled)for(const tab of tabs.filter(tab=>/^https:\/\/www\.themoviedb\.org\//.test(tab.url??'')))messages.push(chrome.tabs.sendMessage(tab.id,{type:'tmdb-preferences-changed',...status}));
  await Promise.allSettled(messages);
}
const extensionPage=(sender,path)=>sender.id===chrome.runtime.id&&sender.url?.split(/[?#]/)[0]===chrome.runtime.getURL(path);
const playerPage=sender=>extensionPage(sender,'watch.html')||extensionPage(sender,'cinema.html');
const TMDB_SITE_ORIGIN='https://www.themoviedb.org/*';
const tmdbPageSender=sender=>Boolean(sender.tab&&sender.frameId===0&&/^https:\/\/www\.themoviedb\.org\//.test(sender.url??''));
const mediaKind=value=>value===undefined||value==='movie'?'movie':value==='tv'?'tv':null;
const episodeNumber=value=>Number.isSafeInteger(value)&&value>=0?value:null;
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id||!['watch-open','watch-movie','watch-media','watch-season'].includes(message?.type))return false;
  const chat=sender.tab&&sender.frameId===0&&providerForUrl(sender.url);
  const tmdbSite=tmdbPageSender(sender);
  if(message.type==='watch-open'&&!chat&&!tmdbSite)return false;
  if(['watch-movie','watch-media','watch-season'].includes(message.type)&&!playerPage(sender))return false;
  const kind=message.type==='watch-movie'?'movie':mediaKind(message.kind);
  if(!kind||!Number.isSafeInteger(message.id)||message.id<1){respond({ok:false,error:'Titolo non valido.'});return false;}
  (async()=>{
    if(message.type==='watch-open'){
      if(tmdbSite&&!await chrome.permissions.contains({origins:[TMDB_SITE_ORIGIN]}))throw new Error('Permesso TMDB non disponibile.');
      const season=episodeNumber(message.season),episode=Number.isSafeInteger(message.episode)&&message.episode>0?message.episode:null;
      const url=kind==='tv'?`cinema.html?kind=tv&id=${message.id}${season!==null?`&season=${season}`:''}${episode!==null?`&episode=${episode}`:''}`:`watch.html?movie=${message.id}`;
      await chrome.tabs.create({url:chrome.runtime.getURL(url)});return {ok:true};
    }
    if(message.type==='watch-season'){
      const season=episodeNumber(message.season);
      if(season===null)throw new Error('Stagione non valida.');
      return {ok:true,item:await tmdb.season(message.id,season)};
    }
    return {ok:true,item:await tmdb.details(kind,message.id)};
  })().then(respond).catch(error=>respond({ok:false,error:error.message??'Player non disponibile.'}));
  return true;
});
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id)return false;
  if(message?.type==='tmdb-site-status'){
    if(!extensionPage(sender,'options.html'))return false;
    (async()=>{const state=await tmdbSiteState(),status=await tmdbStatus();respond({ok:true,...state,...status});})().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='tmdb-site-sync'){
    if(!extensionPage(sender,'options.html'))return false;
    (async()=>{await syncTmdbSite({inject:true});const state=await tmdbSiteState(),status=await tmdbStatus();respond({ok:true,...state,...status});})().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='tmdb-site-disable'){
    if(!extensionPage(sender,'options.html'))return false;
    (async()=>{
      await chrome.storage.local.set({tmdbSiteButton:false});
      disableTmdbSite().catch(()=>{});
      respond({ok:true,enabled:false,allowed:await chrome.permissions.contains({origins:[TMDB_SITE_ORIGIN]})});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='tmdb-site-configured'){
    if(!tmdbPageSender(sender))return false;
    (async()=>{const status=await tmdbStatus();respond({ok:true,configured:status.configured});})().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type?.startsWith("tmdb-site-"))return false;
  if(!message?.type?.startsWith("tmdb-"))return false;
  const settings=extensionPage(sender,"options.html");
  const trusted=settings||extensionPage(sender,"panel.html")||extensionPage(sender,"cinema.html");
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
    if(message.type==="tmdb-remove"&&settings){await chrome.storage.local.remove("tmdbCredential");await chrome.storage.session.remove("tmdbCredential");tmdb.clear();await notifyChats();return {ok:true};}
    if(message.type==="tmdb-search"&&chat)return {ok:true,...await tmdb.search(message.reference)};
    if(message.type==="tmdb-details"&&chat)return {ok:true,item:await tmdb.details(message.kind,message.id)};
    if(message.type==="tmdb-image"&&(chat||trusted))return {ok:true,data:await tmdb.image(message.path,message.size)};
    return {ok:false,error:"Operazione non disponibile."};
  })().then(respond).catch(error=>respond({ok:false,error:error instanceof Error?error.message:"TMDB non disponibile. Riprova."}));
  return true;
});

chrome.sidePanel.setPanelBehavior({openPanelOnActionClick:true}).catch(()=>{});
(async()=>{const state=await tmdbSiteState();if(state.enabled)await registerTmdbSite();else await unregisterTmdbSite();})().catch(()=>{});
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id||message?.type!=="open-panel")return false;
  const chat=sender.tab&&sender.frameId===0&&providerForUrl(sender.url);
  if(!chat)return false;
  chrome.tabs.create({url:chrome.runtime.getURL("panel.html?view=full")}).then(()=>respond({ok:true})).catch(()=>respond({ok:false}));
  return true;
});
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

/* Lightweight update badge. The full download/install flow lives in options. */
const updateChannel="https://github.com/Emanueleparis98/cinematch/releases/latest/download/latest.json";
const updateOrigins=["https://github.com/*","https://objects.githubusercontent.com/*","https://release-assets.githubusercontent.com/*"];
function newerVersion(candidate,current){
  const a=String(candidate).split(".").map(Number),b=String(current).split(".").map(Number);
  if(a.length!==3||a.some(n=>!Number.isSafeInteger(n)))return false;
  for(let i=0;i<3;i++)if(a[i]!==b[i])return a[i]>b[i];
  return false;
}
async function checkUpdateBadge(){
  try{
    if(!await chrome.permissions.contains({origins:updateOrigins}))return;
    const response=await fetch(updateChannel,{cache:"no-store",credentials:"omit",redirect:"follow"});
    if(!response.ok)return;
    const data=await response.json();
    if(!newerVersion(data?.version,chrome.runtime.getManifest().version)){await chrome.action.setBadgeText({text:""});return;}
    await chrome.action.setBadgeText({text:"1"});
    await chrome.action.setBadgeBackgroundColor({color:"#315fd1"});
    await chrome.action.setTitle({title:`CineMatch · aggiornamento ${data.version} disponibile`});
  }catch{}
}
chrome.runtime.onInstalled.addListener(()=>{chrome.alarms.create("cinematch-update",{periodInMinutes:360});checkUpdateBadge();});
chrome.runtime.onStartup.addListener(()=>{chrome.alarms.create("cinematch-update",{periodInMinutes:360});checkUpdateBadge();});
chrome.alarms.onAlarm.addListener(alarm=>{if(alarm.name==="cinematch-update")checkUpdateBadge();});
