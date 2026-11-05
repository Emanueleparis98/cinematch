import {PROVIDERS,providerForUrl} from "./core.js";
import {insertIntoComposer} from "./composer.js";
import {credential,createTmdbClient} from "./tmdb.js";
import {defaultEpisode,publicMedia} from "./cb01-resolver.js";
import {Cb01Error,inspectArticle,resolveEpisode,resolveMovie} from "./cb01-service.js";
import {installLiveService} from "./live-service.js";

const storageReady=Promise.all([
  chrome.storage.local.setAccessLevel({accessLevel:"TRUSTED_CONTEXTS"}),
  chrome.storage.session.setAccessLevel({accessLevel:"TRUSTED_CONTEXTS"})
]);
installLiveService();
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
const DEEPSEEK_URL='https://chat.deepseek.com/*';
const deepseekPageSender=sender=>Boolean(sender.tab&&sender.frameId===0&&/^https:\/\/chat\.deepseek\.com\//.test(sender.url??''));
async function deepseekCurrentsEnabled(){
  await storageReady;
  return (await chrome.storage.local.get('deepseekCurrentsButton')).deepseekCurrentsButton===true;
}
async function notifyDeepseekCurrents(enabled){
  if(enabled===undefined)enabled=await deepseekCurrentsEnabled();
  const tabs=await chrome.tabs.query({url:DEEPSEEK_URL});
  await Promise.allSettled(tabs.map(tab=>chrome.tabs.sendMessage(tab.id,{type:'cinematch:deepseek-currents',enabled})));
}
/* ---------- CB01 ---------- */
const CB01_RULESET='cb01_ads';
const CB01_SCRIPT_IDS=['cinematch-cb01-main','cinematch-cb01-site'];
const CB01_MATCHES=['https://cb01one.org/*','https://www.cb01one.org/*'];
const CB01_ORIGINS=[
  'https://cb01one.org/*',
  'https://www.cb01one.org/*',
  'https://stayonline.pro/*',
  'https://uprot.net/*',
  'https://*.mxdrop.click/*',
  'https://*.m1xdrop.net/*',
  'https://*.mixdrop.co/*',
  'https://*.mixdrop.ag/*',
  'https://*.mxdrop.to/*',
  'https://*.mixdrop.to/*',
  'https://*.mxcontent.net/*'
];
const CB01_TOKEN_PREFIX='cb01Token:';
const CB01_TOKEN_TTL=30*60*1000;
const cb01PageSender=sender=>Boolean(sender.tab&&sender.frameId===0&&/^https:\/\/(?:www\.)?cb01one\.org\//.test(sender.url??''));
async function cb01State(){
  const [stored,allowed]=await Promise.all([chrome.storage.local.get('cb01Mode'),chrome.permissions.contains({origins:CB01_ORIGINS})]);
  return {requested:stored.cb01Mode===true,allowed,enabled:stored.cb01Mode===true&&allowed};
}
async function setCb01Rules(enabled){
  try{
    if(enabled)await chrome.declarativeNetRequest.updateEnabledRulesets({enableRulesetIds:[CB01_RULESET]});
    else await chrome.declarativeNetRequest.updateEnabledRulesets({disableRulesetIds:[CB01_RULESET]});
  }catch{}
}
async function unregisterCb01(){
  try{await chrome.scripting.unregisterContentScripts({ids:CB01_SCRIPT_IDS});}catch{}
}
async function registerCb01(){
  await unregisterCb01();
  await chrome.scripting.registerContentScripts([
    {id:'cinematch-cb01-main',matches:CB01_MATCHES,js:['cb01-main.js'],runAt:'document_start',world:'MAIN',persistAcrossSessions:true},
    {id:'cinematch-cb01-site',matches:CB01_MATCHES,js:['overlay.js','cb01-site.js'],css:['cb01.css'],runAt:'document_idle',persistAcrossSessions:true}
  ]);
}
async function injectCb01(){
  const tabs=await chrome.tabs.query({url:CB01_MATCHES});
  if(!tabs.length)return;
  const installation=Promise.allSettled(tabs.flatMap(tab=>[
    chrome.scripting.insertCSS({target:{tabId:tab.id},files:['cb01.css']}),
    chrome.scripting.executeScript({target:{tabId:tab.id},files:['overlay.js','cb01-site.js']}),
    chrome.scripting.executeScript({target:{tabId:tab.id},files:['cb01-main.js'],world:'MAIN'})
  ]));
  await Promise.race([installation,new Promise(resolve=>setTimeout(resolve,2500))]);
  await Promise.allSettled(tabs.map(tab=>chrome.tabs.sendMessage(tab.id,{type:'cinematch:cb01-enabled'})));
}
async function syncCb01({inject=false}={}){
  const state=await cb01State();
  if(!state.enabled){
    await setCb01Rules(false);
    await unregisterCb01();
    return state;
  }
  await setCb01Rules(true);
  await registerCb01();
  if(inject)await injectCb01();
  return state;
}
async function disableCb01(){
  const tabs=await chrome.tabs.query({url:CB01_MATCHES}).catch(()=>[]);
  const cleanup=Promise.allSettled(tabs.flatMap(tab=>[
    chrome.tabs.sendMessage(tab.id,{type:'cinematch:cb01-disabled'}),
    chrome.scripting.removeCSS({target:{tabId:tab.id},files:['cb01.css']})
  ]));
  await Promise.race([cleanup,new Promise(resolve=>setTimeout(resolve,1500))]);
  await setCb01Rules(false);
  await unregisterCb01();
}
async function pruneCb01Tokens(){
  const all=await chrome.storage.session.get(null),now=Date.now();
  const expired=Object.entries(all).filter(([key,value])=>key.startsWith(CB01_TOKEN_PREFIX)&&(!value?.expires||value.expires<now)).map(([key])=>key);
  if(expired.length)await chrome.storage.session.remove(expired);
}
async function saveCb01Token(record){
  await pruneCb01Tokens();
  const token=crypto.randomUUID();
  await chrome.storage.session.set({[CB01_TOKEN_PREFIX+token]:{...record,expires:Date.now()+CB01_TOKEN_TTL}});
  return {token};
}
async function getCb01Token(token){
  if(typeof token!=='string'||!/^[a-f0-9-]{36}$/i.test(token))throw new Cb01Error('Sessione CB01 non valida.','token');
  const key=CB01_TOKEN_PREFIX+token,data=await chrome.storage.session.get(key),record=data[key];
  if(!record||record.expires<Date.now()){await chrome.storage.session.remove(key);throw new Cb01Error('La sessione CB01 è scaduta. Riprova dalla pagina.','token');}
  return record;
}
async function cb01Playback(token){
  const record=await getCb01Token(token);
  return {media:record.media,stream:record.stream??null,episode:record.episode??null};
}
async function cb01Episode(token,season,episode){
  const record=await getCb01Token(token);
  if(record.parsed?.kind!=='tv')throw new Cb01Error('Questa sessione non è una serie TV.','episode');
  if(!Number.isSafeInteger(season)||season<0||!Number.isSafeInteger(episode)||episode<1)throw new Cb01Error('Episodio non valido.','episode');
  const result=await resolveEpisode(record.parsed,season,episode,{manualUrl:record.media?.key??record.parsed.articleUrl});
  return {stream:result.stream,episode:{season,episode}};
}
async function cb01Article(articleUrl,season=null,episode=null){
  const parsed=await inspectArticle(articleUrl);
  let stream=null,target=null;
  if(parsed.kind==='movie')stream=(await resolveMovie(parsed)).stream;
  else{
    target=season!==null&&episode!==null?{season,episode}:defaultEpisode(parsed);
    if(!target)throw new Cb01Error('Questa serie non contiene episodi disponibili.','no-sources',parsed.articleUrl);
    stream=(await resolveEpisode(parsed,target.season,target.episode,{manualUrl:parsed.articleUrl})).stream;
  }
  const media=publicMedia(parsed);
  const {token}=await saveCb01Token({parsed,media,stream,episode:target});
  return {token,media,stream,episode:target};
}
const extensionPage=(sender,path)=>sender.id===chrome.runtime.id&&sender.url?.split(/[?#]/)[0]===chrome.runtime.getURL(path);
const settingsPage=sender=>extensionPage(sender,'settings-popup.html');
const playerPage=sender=>extensionPage(sender,'watch.html')||extensionPage(sender,'cinema.html');
const TMDB_SITE_ORIGIN='https://www.themoviedb.org/*';
const tmdbPageSender=sender=>Boolean(sender.tab&&sender.frameId===0&&/^https:\/\/www\.themoviedb\.org\//.test(sender.url??''));
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id)return false;
  if(message?.type==='deepseek-currents-status'){
    if(!deepseekPageSender(sender))return false;
    (async()=>respond({ok:true,enabled:await deepseekCurrentsEnabled()}))().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='deepseek-currents-sync'){
    if(!settingsPage(sender))return false;
    (async()=>{
      const enabled=await deepseekCurrentsEnabled();
      await notifyDeepseekCurrents(enabled);
      respond({ok:true,enabled});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='deepseek-insert-prompt'){
    if(!extensionPage(sender,'compact.html')||typeof message.prompt!=='string')return false;
    const mode=['empty','append','replace'].includes(message.mode)?message.mode:'empty';
    (async()=>{
      let tab=sender.tab;
      if(!tab||providerForUrl(tab.url)?.id!=='deepseek'){
        const tabs=await chrome.tabs.query({url:'https://chat.deepseek.com/*'});
        tab=tabs.find(item=>item.active)??tabs.at(-1);
      }
      if(!tab?.id||providerForUrl(tab.url)?.id!=='deepseek'){
        respond({ok:false,code:'no-deepseek',error:'Apri chat.deepseek.com per inserire il prompt.'});return;
      }
      if(!sender.tab)await chrome.tabs.update(tab.id,{active:true}).catch(()=>{});
      const results=await chrome.scripting.executeScript({target:{tabId:tab.id},func:insertIntoComposer,args:[{provider:'deepseek',prompt:message.prompt,mode}]});
      respond(results[0]?.result??{ok:false,code:'no-editor'});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  return false;
});
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
  if(sender.id!==chrome.runtime.id||!message?.type?.startsWith('cb01-'))return false;
  const settings=settingsPage(sender);
  if(message.type==='cb01-status'){
    if(!settings)return false;
    (async()=>{
      const state=await cb01State();
      let rules=[];
      try{rules=await chrome.declarativeNetRequest.getEnabledRulesets();}catch{}
      respond({ok:true,...state,rules:rules.includes(CB01_RULESET)});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message.type==='cb01-sync'){
    if(!settings)return false;
    (async()=>{
      await chrome.storage.local.set({cb01Mode:true});
      const state=await syncCb01({inject:true});
      respond({ok:true,...state});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message.type==='cb01-disable'){
    if(!settings)return false;
    (async()=>{
      await chrome.storage.local.set({cb01Mode:false});
      await Promise.race([disableCb01().catch(()=>{}),new Promise(resolve=>setTimeout(resolve,3000))]);
      respond({ok:true,enabled:false,allowed:await chrome.permissions.contains({origins:CB01_ORIGINS})});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message.type==='cb01-resolve'){
    if(!cb01PageSender(sender))return false;
    (async()=>{
      if(!(await cb01State()).enabled)throw new Cb01Error('Modalità CB01 non attiva.','disabled');
      const season=Number.isSafeInteger(message.season)&&message.season>=0?message.season:null;
      const episode=Number.isSafeInteger(message.episode)&&message.episode>0?message.episode:null;
      respond({ok:true,...await cb01Article(message.articleUrl,season,episode)});
    })().catch(error=>respond({ok:false,error:error.message??'Sorgente CB01 non disponibile.',code:error.code,manualUrl:error.manualUrl,failures:error.failures}));
    return true;
  }
  if(message.type==='cb01-episode'){
    if(!playerPage(sender)&&!cb01PageSender(sender))return false;
    (async()=>respond({ok:true,...await cb01Episode(message.token,message.season,message.episode)}))()
      .catch(error=>respond({ok:false,error:error.message??'Episodio non disponibile.',code:error.code,manualUrl:error.manualUrl}));
    return true;
  }
  if(message.type==='cb01-playback'){
    if(!playerPage(sender))return false;
    (async()=>respond({ok:true,...await cb01Playback(message.token)}))()
      .catch(error=>respond({ok:false,error:error.message??'Sessione CB01 non disponibile.',code:error.code}));
    return true;
  }
  if(message.type==='cb01-open'){
    if(!playerPage(sender)&&!cb01PageSender(sender))return false;
    (async()=>{
      await getCb01Token(message.token);
      await chrome.tabs.create({url:chrome.runtime.getURL(`cinema.html?directToken=${encodeURIComponent(message.token)}`)});
      respond({ok:true});
    })().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  return false;
});
chrome.runtime.onMessage.addListener((message,sender,respond)=>{
  if(sender.id!==chrome.runtime.id)return false;
  if(message?.type==='tmdb-site-status'){
    if(!settingsPage(sender))return false;
    (async()=>{const state=await tmdbSiteState(),status=await tmdbStatus();respond({ok:true,...state,...status});})().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='tmdb-site-sync'){
    if(!settingsPage(sender))return false;
    (async()=>{await syncTmdbSite({inject:true});const state=await tmdbSiteState(),status=await tmdbStatus();respond({ok:true,...state,...status});})().catch(error=>respond({ok:false,error:error.message}));
    return true;
  }
  if(message?.type==='tmdb-site-disable'){
    if(!settingsPage(sender))return false;
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
  const settings=settingsPage(sender);
  const trusted=settings||extensionPage(sender,"panel.html")||extensionPage(sender,"cinema.html");
  const chat=sender.tab&&sender.frameId===0&&providerForUrl(sender.url);
  if(!trusted&&!chat)return false;
  (async()=>{
    await storageReady;
    if(message.type==="tmdb-status")return tmdbStatus();
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

(async()=>{const state=await tmdbSiteState();if(state.enabled)await registerTmdbSite();else await unregisterTmdbSite();})().catch(()=>{});
(async()=>{const state=await cb01State();if(state.enabled)await syncCb01();else{await setCb01Rules(false);await unregisterCb01();}})().catch(()=>{});
chrome.permissions.onRemoved.addListener(permissions=>{
  if(!permissions.origins?.some(origin=>CB01_ORIGINS.includes(origin)))return;
  syncCb01().catch(()=>{});
});
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
