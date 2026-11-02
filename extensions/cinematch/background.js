import {PROVIDERS,providerForUrl} from "./core.js";
import {insertIntoComposer} from "./composer.js";

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
