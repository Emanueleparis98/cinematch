import {AD_DOMAINS, LIVE_TTL, publicLiveUrl, matchLiveSite, liveOrigins, sourceFromUrl, uniqueLiveSources, extractLiveUrls, extractLiveFrames} from './live-core.js';

const TAB_PREFIX = 'liveTab:', TOKEN_PREFIX = 'liveToken:';
const SCRIPT_IDS = ['cinematch-live-site','cinematch-live-frames','cinematch-live-main'];
const catalog = fetch(chrome.runtime.getURL('data/live-sites.json')).then(response => response.json());
const page = (sender,path) => sender.id === chrome.runtime.id && sender.url?.split(/[?#]/)[0] === chrome.runtime.getURL(path);
const settings = sender => page(sender,'settings-popup.html');
const player = sender => page(sender,'cinema.html');
const serial = new Map();
const pageIdentity=value=>{
  const safe=publicLiveUrl(value);
  return safe?safe+new URL(value).hash:null;
};
let adQueue=Promise.resolve();
function syncAdTabs(){
  adQueue=adQueue.catch(()=>{}).then(async()=>{
    const enabled=(await chrome.storage.local.get('liveMode')).liveMode===true;
    const tabs=enabled?await chrome.tabs.query({}):[];
    const ids=tabs.filter(tab=>matchLiveSite(tab.url,cachedSites??[])).map(tab=>tab.id);
    await chrome.declarativeNetRequest.updateSessionRules({removeRuleIds:[19001],addRules:ids.length?[{id:19001,priority:1,action:{type:'block'},condition:{tabIds:ids,requestDomains:AD_DOMAINS,resourceTypes:['main_frame','script','image','xmlhttprequest','sub_frame','stylesheet','media','ping','other']}}]:[]});
  });
  return adQueue;
}
function locked(tabId, fn) {
  const operation = (serial.get(tabId) ?? Promise.resolve()).catch(()=>{}).then(fn);
  serial.set(tabId,operation);
  operation.finally(()=>{if(serial.get(tabId)===operation)serial.delete(tabId);}).catch(()=>{});
  return operation;
}
async function supportedTab(tabId) {
  if (!Number.isInteger(tabId) || tabId < 0) return null;
  const tab = await chrome.tabs.get(tabId).catch(()=>null);
  const data = await chrome.storage.local.get('liveMode');
  if (!tab?.url || data.liveMode !== true) return null;
  const site = matchLiveSite(tab.url,(await catalog).sites);
  if (!site || !await chrome.permissions.contains({origins:[new URL(tab.url).origin+'/*']})) return null;
  return {tab,site};
}
async function updateTab(tabId, change) {
  return locked(tabId,async()=>{
    const supported = await supportedTab(tabId); if (!supported) return null;
    const key = TAB_PREFIX+tabId, url = pageIdentity(supported.tab.url);
    const previous = (await chrome.storage.session.get(key))[key];
    const record = previous?.page === url ? previous : {page:url,title:supported.tab.title??supported.site.name,site:supported.site.name,sources:[],frames:[]};
    record.title=supported.tab.title??record.title;
    change(record);
    record.updated = Date.now();
    record.sources = record.sources.slice(-40); record.frames = [...new Set(record.frames)].slice(-12);
    await chrome.storage.session.set({[key]:record});
    return record;
  });
}
async function status() {
  const saved = await chrome.storage.local.get('liveMode');
  const scripts = await chrome.scripting.getRegisteredContentScripts();
  return {ok:true,requested:saved.liveMode===true,enabled:saved.liveMode===true&&scripts.some(script=>script.id===SCRIPT_IDS[0]),count:(await catalog).sites.length};
}
let syncQueue = Promise.resolve();
function sync({inject=false}={}) {
  syncQueue = syncQueue.catch(()=>{}).then(async()=>{
    const data = await chrome.storage.local.get(['liveMode','liveFrameOrigins']);
    const registered=await chrome.scripting.getRegisteredContentScripts();
    const ids=registered.filter(script=>SCRIPT_IDS.includes(script.id)).map(script=>script.id);
    if(ids.length)await chrome.scripting.unregisterContentScripts({ids});
    const rules = await chrome.declarativeNetRequest.getDynamicRules();
    await chrome.declarativeNetRequest.updateDynamicRules({removeRuleIds:rules.filter(rule=>rule.id===19001).map(rule=>rule.id)});
    await syncAdTabs();
    if (!data.liveMode) return;
    const origins = liveOrigins((await catalog).sites), allowed = [];
    for (const origin of origins) if (await chrome.permissions.contains({origins:[origin]})) allowed.push(origin);
    const frames = [];
    for (const origin of (data.liveFrameOrigins??[]).slice(0,64)) if (await chrome.permissions.contains({origins:[origin]})) frames.push(origin);
    if (allowed.length) {
      await chrome.scripting.registerContentScripts([{id:SCRIPT_IDS[0],matches:allowed,js:['overlay.js','live-site.js'],allFrames:true,runAt:'document_start',persistAcrossSessions:true}]);
      const mainMatches=(await catalog).sites.flatMap(site=>{
        const host=site.host.replace(/^www\./,''),path=site.path==='/'?'/*':site.host==='zhangboheng.github.io'?site.path.replace(/\/[^/]*$/,'/*'):site.path+'*';
        return ['http:','https:'].flatMap(scheme=>[host,'www.'+host].map(name=>`${scheme}//${name}${path}`));
      }).filter(pattern=>allowed.some(origin=>new URL(origin).origin===new URL(pattern).origin));
      await chrome.scripting.registerContentScripts([{id:SCRIPT_IDS[2],matches:[...new Set(mainMatches)],js:['live-main.js'],world:'MAIN',runAt:'document_start',persistAcrossSessions:true}]);
    }
    if (frames.length) await chrome.scripting.registerContentScripts([{id:SCRIPT_IDS[1],matches:frames,js:['live-site.js'],allFrames:true,runAt:'document_start',persistAcrossSessions:true}]);
    if (inject) {
      const tabs = await chrome.tabs.query({});
      await Promise.allSettled(tabs.filter(tab=>matchLiveSite(tab.url,(cachedSites??[]))).map(async tab=>{
        if (!await supportedTab(tab.id)) return;
        await chrome.scripting.executeScript({target:{tabId:tab.id},files:['overlay.js','live-site.js']}).catch(()=>{});
        await chrome.scripting.executeScript({target:{tabId:tab.id},files:['live-main.js'],world:'MAIN'}).catch(()=>{});
        await chrome.tabs.sendMessage(tab.id,{type:'cinematch:live-enabled'}).catch(()=>{});
      }));
    }
  });
  return syncQueue;
}
let cachedSites;
catalog.then(data=>{cachedSites=data.sites;}).catch(()=>{});

async function boundedPage(url) {
  const response = await fetch(url,{credentials:'omit',referrerPolicy:'no-referrer',redirect:'error',signal:AbortSignal.timeout(6000)});
  if (!response.ok) throw new Error('Pagina non disponibile.');
  if (!/text\/html|application\/(?:json|javascript)|text\/plain/i.test(response.headers.get('content-type')??'')) return '';
  const reader=response.body.getReader(),decoder=new TextDecoder();let size=0,text='';
  try { while(true) {const {value,done}=await reader.read();if(done)break;size+=value.byteLength;if(size>2*1024*1024)throw new Error('Pagina troppo grande.');text+=decoder.decode(value,{stream:true});}return text+decoder.decode(); }
  finally {await reader.cancel().catch(()=>{});}
}
async function resolveRecord(record) {
  const streams = [...record.sources], missing = new Set(), failures = [];
  const queue = [...(record.sources.length?[]:[record.page]),...record.frames].filter(Boolean).map(url=>({url,depth:0})), seen = new Set();
  const deadline=Date.now()+12000;
  while(queue.length && seen.size<8 && Date.now()<deadline) {
    const {url,depth} = queue.shift();
    if(seen.has(url)||!publicLiveUrl(url))continue;seen.add(url);
    const origin = new URL(url).origin+'/*';
    if(!await chrome.permissions.contains({origins:[origin]})){missing.add(origin);continue;}
    try {
      const html=await boundedPage(url);
      streams.push(...extractLiveUrls(html,url));
      if(depth<2)for(const frame of extractLiveFrames(html,url))queue.push({url:frame,depth:depth+1});
    } catch { failures.push(new URL(url).hostname); }
  }
  return {...record,streams:uniqueLiveSources(streams),pendingOrigins:[...missing].slice(0,12),failures:[...new Set(failures)],expires:Date.now()+LIVE_TTL};
}
async function prune() {
  const all=await chrome.storage.session.get(null);
  await chrome.storage.session.remove(Object.entries(all).filter(([key,value])=>key.startsWith(TOKEN_PREFIX)&&(!value?.expires||value.expires<Date.now())).map(([key])=>key));
}
async function tokenRecord(token) {
  if(typeof token!=='string'||!/^[a-f0-9-]{36}$/i.test(token))throw new Error('Sessione Live non valida.');
  const record=(await chrome.storage.session.get(TOKEN_PREFIX+token))[TOKEN_PREFIX+token];
  if(!record||record.expires<Date.now())throw new Error('Sessione scaduta. Premi di nuovo Flussi sul sito.');
  if(!await supportedTab(record.tabId))throw new Error('Modalità Live disattivata o sito non più disponibile.');
  const tab=await chrome.tabs.get(record.tabId);
  if(pageIdentity(tab.url)!==record.page)throw new Error('La pagina è cambiata. Premi di nuovo Flussi sul nuovo evento.');
  return record;
}

export function installLiveService() {
  sync().catch(()=>{});
  chrome.permissions.onRemoved.addListener(()=>sync().catch(()=>{}));
  chrome.tabs.onRemoved.addListener(tabId=>{
    locked(tabId,()=>chrome.storage.session.remove(TAB_PREFIX+tabId)).catch(()=>{});
    syncAdTabs().catch(()=>{});
  });
  chrome.tabs.onUpdated.addListener((tabId,change)=>{
    if(change.url){locked(tabId,()=>chrome.storage.session.remove(TAB_PREFIX+tabId)).catch(()=>{});syncAdTabs().catch(()=>{});}
  });
  chrome.webRequest.onHeadersReceived.addListener(details=>{
    const mime=details.responseHeaders?.find(header=>header.name.toLowerCase()==='content-type')?.value??'';
    const source=sourceFromUrl(details.url,mime);
    if(!source||details.statusCode<200||details.statusCode>=300)return;
    updateTab(details.tabId,record=>{
      record.sources=record.sources.filter(item=>item.url!==source.url);
      record.sources.unshift({...source,mime});
    }).catch(()=>{});
  },{urls:['http://*/*','https://*/*'],types:['media','xmlhttprequest','other']},['responseHeaders']);
  chrome.runtime.onMessage.addListener((message,sender,respond)=>{
    if(sender.id!==chrome.runtime.id||!message?.type?.startsWith('live-'))return false;
    const run=async()=>{
      if(message.type==='live-status'&&settings(sender))return status();
      if(message.type==='live-sync'&&settings(sender)){
        await chrome.storage.local.set({liveMode:true});await sync({inject:true});return status();
      }
      if(message.type==='live-disable'&&settings(sender)){
        await chrome.storage.local.set({liveMode:false});await sync();
        const tabs=await chrome.tabs.query({});
        await Promise.allSettled(tabs.filter(tab=>matchLiveSite(tab.url,cachedSites??[])).map(tab=>chrome.tabs.sendMessage(tab.id,{type:'cinematch:live-disabled'})));
        const all=await chrome.storage.session.get(null);await chrome.storage.session.remove(Object.keys(all).filter(key=>key.startsWith(TAB_PREFIX)||key.startsWith(TOKEN_PREFIX)));
        return {ok:true,enabled:false};
      }
      if(['live-collect','live-context','live-create','live-reset'].includes(message.type)){
        const context=await supportedTab(sender.tab?.id);
        if(!context||!publicLiveUrl(sender.url)||!await chrome.permissions.contains({origins:[new URL(sender.url).origin+'/*']}))throw new Error('Sito Live non autorizzato.');
        if(message.type==='live-context'){if(sender.frameId===0)await syncAdTabs();return {ok:true,top:sender.frameId===0,site:context.site.name};}
        if(message.type==='live-reset'){
          if(sender.frameId!==0)throw new Error('Pagina non valida.');
          await updateTab(sender.tab.id,record=>{record.sources=[];record.frames=[];});return {ok:true};
        }
        if(message.type==='live-collect'){
          // Ignore delayed messages from frames of the previous document.
          if(sender.frameId===0&&pageIdentity(sender.url)!==pageIdentity(context.tab.url))return {ok:false};
          const sources=Array.isArray(message.sources)?message.sources.slice(0,40).map(url=>sourceFromUrl(url,'',sender.url)).filter(Boolean):[];
          const frames=Array.isArray(message.frames)?message.frames.slice(0,12).map(url=>publicLiveUrl(url,sender.url)).filter(Boolean):[];
          await updateTab(sender.tab.id,record=>{
            record.sources=uniqueLiveSources([...sources,...record.sources]);
            record.frames.push(...frames);
          });return {ok:true};
        }
        if(sender.frameId!==0)throw new Error('Apri Flussi dalla pagina principale.');
        await chrome.tabs.sendMessage(sender.tab.id,{type:'cinematch:live-scan'}).catch(()=>{});
        const record=await updateTab(sender.tab.id,()=>{});
        const token=crypto.randomUUID();await prune();
        await chrome.storage.session.set({[TOKEN_PREFIX+token]:{...record,tabId:sender.tab.id,expires:Date.now()+LIVE_TTL}});
        return {ok:true,token,title:record.title};
      }
      if(['live-playback','live-refresh','live-pause-original'].includes(message.type)&&player(sender)){
        const record=await tokenRecord(message.token);
        if(message.type==='live-pause-original'){
          await chrome.tabs.sendMessage(record.tabId,{type:'cinematch:live-pause'}).catch(()=>{});return {ok:true};
        }
        if(message.type==='live-refresh'){
          const extra=(record.pendingOrigins??[]).filter(origin=>publicLiveUrl(origin.replace('/*','/')));
          const saved=await chrome.storage.local.get('liveFrameOrigins');
          const origins=[...new Set([...(saved.liveFrameOrigins??[]),...extra])].slice(-64);
          await chrome.storage.local.set({liveFrameOrigins:origins});await sync();
          await chrome.scripting.executeScript({target:{tabId:record.tabId,allFrames:true},files:['live-site.js']}).catch(()=>{});
          await chrome.tabs.sendMessage(record.tabId,{type:'cinematch:live-scan'}).catch(()=>{});
        }
        const latest=(await chrome.storage.session.get(TAB_PREFIX+record.tabId))[TAB_PREFIX+record.tabId];
        const next=await resolveRecord({...record,sources:uniqueLiveSources([...(latest?.sources??[]),...record.sources]),frames:[...new Set([...(latest?.frames??[]),...record.frames])]});
        await tokenRecord(message.token);
        await chrome.storage.session.set({[TOKEN_PREFIX+message.token]:next});
        return {ok:true,...next};
      }
      if(message.type==='live-open'&&(player(sender)||sender.frameId===0&&await supportedTab(sender.tab?.id))){
        const record=await tokenRecord(message.token);
        if(!player(sender)&&record.tabId!==sender.tab?.id)throw new Error('Sessione appartenente a un’altra pagina.');
        await chrome.tabs.create({url:chrome.runtime.getURL('cinema.html?liveToken='+encodeURIComponent(message.token))});return {ok:true};
      }
      throw new Error('Richiesta Live non autorizzata.');
    };
    run().then(respond).catch(error=>respond({ok:false,error:error.message??'Flussi Live non disponibili.'}));return true;
  });
}
