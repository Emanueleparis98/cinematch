/* Isolated-world collector: no eval, remote scripts or page-to-extension bridge.
   In authorized child frames it only reports media/frame URLs to the worker. */
(() => {
  if (globalThis.cineMatchLiveLoaded) return;
  globalThis.cineMatchLiveLoaded = true;
  let enabled=false, top=false, observer, timer, host, button, note, busy=false;
  const send=message=>chrome.runtime.sendMessage(message).catch(()=>({ok:false,error:'CineMatch non disponibile. Ricarica la pagina.'}));
  const clean=value=>String(value??'').replace(/\\\//g,'/').replace(/\\u0026/gi,'&').replace(/&amp;/g,'&');
  const media=value=>/\.(?:m3u8?|mpd|mp4|webm|mkv)(?:[?#]|$)/i.test(value);
  async function collect() {
    if(!enabled)return;
    const sources=new Set(),frames=new Set();
    for(const node of document.querySelectorAll('video,video source,a[href]')){
      const value=node.currentSrc||node.src||node.href;
      if(value&&media(value))sources.add(value);
    }
    // Blob-backed players still expose their HTTP manifests in resource timing.
    for(const entry of performance.getEntriesByType('resource'))if(media(entry.name))sources.add(entry.name);
    for(const script of [...document.scripts].slice(0,120)){
      const text=clean(script.textContent).slice(0,200000);
      for(const match of text.matchAll(/(?:https?:)?\/\/[^\s"'<>`\\]+|["']((?:\.{0,2}\/)?[^"'<>\s]+\.(?:m3u8?|mpd|mp4|webm)(?:\?[^"'<>\s]*)?)["']/gi)){
        const value=match[1]??match[0];if(media(value))sources.add(value);
        if(sources.size>=40)break;
      }
      if(sources.size>=40)break;
    }
    for(const frame of document.querySelectorAll('iframe[src]')){
      if(frame.src.startsWith('http'))frames.add(frame.src);
      if(frames.size>=12)break;
    }
    const result=await send({type:'live-collect',sources:[...sources].slice(0,40),frames:[...frames]});
    return result;
  }
  function schedule(){if(!enabled)return;clearTimeout(timer);timer=setTimeout(()=>collect(),350);}
  async function open() {
    if(busy)return;busy=true;button.disabled=true;button.textContent='Cerco i flussi…';note.textContent='';
    try{
      await collect();
      const result=await send({type:'live-create'});
      if(!result?.ok)throw new Error(result?.error??'Sorgenti non disponibili.');
      if(!globalThis.CineMatchCinema?.openLive?.({token:result.token,title:result.title,posterEl:button})){
        const opened=await send({type:'live-open',token:result.token});
        if(!opened?.ok)throw new Error(opened?.error??'Player non disponibile.');
      }
    }catch(error){note.textContent=error.message;}
    finally{busy=false;button.disabled=false;button.textContent='▶ Flussi';}
  }
  function mount() {
    if(!top||host?.isConnected||!document.body)return;
    host=document.createElement('div');host.id='cinematch-live-button';host.setAttribute('data-cinematch-ui','live');
    const shadow=host.attachShadow({mode:'closed'}),style=document.createElement('style');
    style.textContent=':host{all:initial;position:fixed;right:20px;bottom:24px;z-index:2147483600;font:13px system-ui,sans-serif}button{display:block;margin-left:auto;border:1px solid #8caaff;border-radius:999px;background:#17243f;color:#f3f6ff;padding:12px 18px;font:700 13px system-ui,sans-serif;box-shadow:0 6px 25px #0007;cursor:pointer}button:hover{background:#263c65}button:focus-visible{outline:3px solid #a9c2ff;outline-offset:3px}button:disabled{opacity:.7}p{max-width:260px;background:#17243f;color:white;border-radius:9px;padding:8px;margin:6px 0 0;font:12px/1.4 system-ui,sans-serif}p:empty{display:none}';
    button=document.createElement('button');button.type='button';button.textContent='▶ Flussi';button.setAttribute('aria-label','Apri i flussi Live nel player CineMatch');
    button.addEventListener('click',event=>{if(!event.isTrusted)return;event.preventDefault();event.stopPropagation();open();});
    note=document.createElement('p');note.setAttribute('role','status');shadow.append(style,button,note);document.documentElement.append(host);
  }
  async function init() {
    const context=await send({type:'live-context'});if(!context?.ok){cleanup();return;}
    enabled=true;top=context.top;
    mount();
    if(!observer){observer=new MutationObserver(changes=>{
      if(changes.every(change=>change.target instanceof Element&&change.target.closest('[data-cinematch-ui]')))return;
      mount();schedule();
    });observer.observe(document.documentElement,{childList:true,subtree:true,attributes:true,attributeFilter:['src']});}
    await collect();
  }
  function cleanup(){enabled=false;observer?.disconnect();observer=null;clearTimeout(timer);host?.remove();host=null;globalThis.CineMatchCinema?.close?.();window.dispatchEvent(new Event('cinematch:live-disabled'));}
  chrome.runtime.onMessage.addListener((message,sender,respond)=>{
    if(sender.id!==chrome.runtime.id)return false;
    if(message?.type==='cinematch:live-disabled'){cleanup();return;}
    if(message?.type==='cinematch:live-enabled'){window.dispatchEvent(new Event('cinematch:live-enabled'));init();return;}
    if(message?.type==='cinematch:live-pause'&&enabled){for(const video of document.querySelectorAll('video'))video.pause();respond({ok:true});return;}
    if(message?.type==='cinematch:live-scan'&&enabled){collect().then(()=>respond({ok:true}));return true;}
  });
  document.addEventListener('loadedmetadata',schedule,true);
  // Intercept our control before the page can attach click/popunder handlers.
  window.addEventListener('click',event=>{
    if(enabled&&top&&event.isTrusted&&event.target===host){event.preventDefault();event.stopImmediatePropagation();open();}
    else if(enabled&&top&&event.isTrusted&&event.target instanceof Element&&!event.target.closest('[data-cinematch-ui]')){
      performance.clearResourceTimings();send({type:'live-reset'}).then(schedule);
    }
  },true);
  window.addEventListener('popstate',schedule);
  if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',init,{once:true});else init();
})();
