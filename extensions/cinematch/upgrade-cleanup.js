/* One-time removal of state left by the withdrawn 3.19.0 release. */
export async function cleanupWithdrawnRelease(api=globalThis.chrome){
  const local=await api.storage.local.get(['liveMode','liveFrameOrigins']);
  const scripts=await api.scripting.getRegisteredContentScripts();
  const ids=scripts.filter(script=>script.id.startsWith('cinematch-live-')).map(script=>script.id);
  if(ids.length)await api.scripting.unregisterContentScripts({ids});
  for(const kind of ['Dynamic','Session']){
    const rules=await api.declarativeNetRequest[`get${kind}Rules`]();
    if(rules.some(rule=>rule.id===19001))await api.declarativeNetRequest[`update${kind}Rules`]({removeRuleIds:[19001]});
  }
  const session=await api.storage.session.get(null);
  const keys=Object.keys(session).filter(key=>key.startsWith('liveTab:')||key.startsWith('liveToken:'));
  if(keys.length)await api.storage.session.remove(keys);
  await api.storage.local.remove(['liveMode','liveFrameOrigins']);
  if(local.liveMode===undefined&&!local.liveFrameOrigins&&!ids.length)return;
  // Keep permissions used by addons; only remove the withdrawn feature's UI.
  const tabs=await api.tabs.query({});
  await Promise.allSettled(tabs.map(tab=>api.scripting.executeScript({target:{tabId:tab.id,allFrames:true},func:()=>{
    window.dispatchEvent(new Event('cinematch:live-disabled'));
    const button=document.getElementById('cinematch-live-button');
    if(button){globalThis.CineMatchCinema?.close?.();button.remove();}
  }})));
}
