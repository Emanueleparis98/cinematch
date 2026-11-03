import {httpUrl,manifestUrl,permissionOrigin,parseManifest,resourceUrl,parseStreams,fetchJson,MAX_JSON} from './streams.js';
import {enhanceSelects,refreshSelects} from './controls.js';
import {declaredCodecs,compatibility,probePlayback,containerExtension,likelyMatroska,nativePlaybackError} from './playback.js';
import {createCodecPlayer} from './codec-player.js';
import {mediaPermission,playbackPermissions,permissionLabel} from './media-access.js';
const $=id=>document.getElementById(id),el=(tag,text,cls)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;if(cls)n.className=cls;return n;};
const prefix='streamAddon:';let addons=[],movie=null,groups=[],generation=0,searchAbort=null,hls=null,selected=null,blockedUrl=null,playGeneration=0,playTimer,probeAbort,activeProbe=null,failure=null;
const video=$('video');video.crossOrigin='anonymous';
let integrated=null,engineMode='native',engineCleanup=Promise.resolve(),scrubbing=false;
const codecCache=new Map(),checkCodecs=codecs=>compatibility(codecs,mime=>{if(!codecCache.has(mime))codecCache.set(mime,video.canPlayType(mime));return codecCache.get(mime);});
const status=(id,text,error=false)=>{$(id).textContent=text;$(id).classList.toggle('error',error);};
const allowed=async url=>chrome.permissions.contains({origins:[permissionOrigin(url)]});
function grant(url){return chrome.permissions.request({origins:[mediaPermission(url)]});}
function tab(name){const isAddons=name==='addons';$('addons-pane').hidden=!isAddons;$('player-pane').hidden=isAddons;$('show-addons').setAttribute('aria-pressed',String(isAddons));$('show-player').setAttribute('aria-pressed',String(!isAddons));history.replaceState(null,'',location.pathname+location.search+(isAddons?'#addons':''));}
$('show-addons').onclick=()=>tab('addons');$('setup-addons').onclick=()=>{tab('addons');$('addon-url').focus();};$('show-player').onclick=()=>tab('player');
async function readAddons(){
  const data=await chrome.storage.local.get(null);addons=Object.entries(data).filter(([key])=>key.startsWith(prefix)).flatMap(([key,value])=>{
    try{const addon=parseManifest({id:value.id,name:value.name,version:value.version,description:value.description,resources:value.resources.map(r=>({name:'stream',types:r.types,idPrefixes:r.prefixes})),types:['movie']},value.url);return [{...addon,key,enabled:value.enabled!==false}];}catch{return [];}
  }).sort((a,b)=>a.name.localeCompare(b.name,'it'));renderAddons();
}
function renderAddons(){
  $('addon-count').textContent=String(addons.length);$('installed-count').textContent=`${addons.filter(a=>a.enabled).length} attivi`;$('installed-empty').hidden=addons.length>0;$('installed-list').replaceChildren();
  for(const addon of addons){
    const row=el('div',undefined,'installed-addon'),body=el('div'),label=el('label'),check=el('input');check.type='checkbox';check.checked=addon.enabled;check.setAttribute('aria-label',`Attiva ${addon.name}`);
    check.onchange=async()=>{const {key,...saved}=addon;saved.enabled=check.checked;await chrome.storage.local.set({[key]:saved});};label.append(check,el('span',addon.name));body.append(label,el('p',`${httpUrl(addon.url).host}${addon.version?' · '+addon.version:''}`));
    const remove=el('button','Rimuovi');remove.type='button';remove.setAttribute('aria-label',`Rimuovi ${addon.name}`);remove.onclick=async()=>{await chrome.storage.local.remove(addon.key);};row.append(body,remove);$('installed-list').append(row);
  }
}
$('addon-form').onsubmit=async event=>{
  event.preventDefault();const button=event.submitter;
  try{
    const url=manifestUrl($('addon-url').value);button.disabled=true;status('addon-status',`Collegamento a ${httpUrl(url).host}…`);
    if(!await grant(url))throw new Error('Accesso al sito non concesso. Riprova e consenti il collegamento.');
    if(addons.length>=20&&!addons.some(a=>a.url===url))throw new Error('Hai già 20 addon. Rimuovine uno prima di aggiungerne altri.');
    const addon=parseManifest(await fetchJson(url),url),digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(url)),key=prefix+Array.from(new Uint8Array(digest),b=>b.toString(16).padStart(2,'0')).join('');
    await chrome.storage.local.set({[key]:addon});$('addon-url').value='';status('addon-status',`${addon.name} collegato. Torna al player per scegliere una sorgente.`);
  }catch(error){status('addon-status',error.message,true);}finally{button.disabled=false;}
};
function stopVideo(){clearTimeout(playTimer);scrubbing=false;probeAbort?.abort();activeProbe=null;if(hls){hls.destroy();hls=null;}const previous=integrated;integrated=null;engineMode='native';if(previous)engineCleanup=engineCleanup.then(()=>previous.destroy()).catch(()=>{});$('codec-controls').hidden=true;$('codec-screen').hidden=true;document.querySelector('.screen').classList.remove('codec-active');delete $('codec-screen').dataset.audio;delete $('codec-screen').dataset.video;delete $('playback-stage').dataset.engine;video.pause();video.removeAttribute('src');video.replaceChildren();video.load();$('retry-play').hidden=true;$('resume-play').hidden=true;$('grant-media').hidden=true;blockedUrl=null;}
function renderStreams(){
  const filter=$('source-filter'),previous=filter.value;filter.replaceChildren(new Option('Tutti gli addon','all'));
  for(const g of groups)filter.append(new Option(g.name,g.key));filter.value=[...filter.options].some(o=>o.value===previous)?previous:'all';refreshSelects();$('source-filter-wrap').hidden=groups.length<2;
  const visible=groups.filter(g=>filter.value==='all'||filter.value===g.key);$('stream-list').replaceChildren();
  for(const g of visible){
    if(g.error){const row=el('div',undefined,'stream-row'),body=el('div');body.append(el('h3',g.name),el('p',g.error));row.append(body);
      if(g.permission){const retry=el('button','Ricollega');retry.type='button';retry.onclick=async()=>{if(await grant(g.permission))await loadStreams();};row.append(retry);}$('stream-list').append(row);continue;}
    for(const stream of [...g.streams].sort((a,b)=>checkCodecs(declaredCodecs(b)).score-checkCodecs(declaredCodecs(a)).score)){const row=el('div',undefined,'stream-row'),body=el('div');row.classList.toggle('selected',selected===stream);body.append(el('h3',stream.name));if(stream.description)body.append(el('p',stream.description));body.append(el('p',`${g.name} · ${{hls:'HLS',video:'HTTP',external:'Sito esterno',torrent:'Torrent',headers:'Proxy richiesto',dash:'DASH',unsupported:'Non supportato'}[stream.kind]}`,'stream-addon'));if(stream.kind==='video'&&checkCodecs(declaredCodecs(stream)).warning)body.append(el('p','Questo formato usa i decoder integrati. Compatibilità verificata all’avvio.'));row.append(body);
      if(stream.playable){const play=el('button',selected===stream?'Sorgente attiva':'Riproduci','primary');play.type='button';play.setAttribute('aria-label',`Riproduci ${stream.name} da ${g.name}`);play.onclick=()=>playStream(stream);row.append(play);}
      else if(stream.kind==='external'){const link=el('a','Apri sul sito');link.href=stream.externalUrl;link.target='_blank';link.rel='noopener noreferrer';row.append(link);}
      else row.append(el('span',stream.reason,'unavailable'));$('stream-list').append(row);
    }
  }
  const total=groups.reduce((n,g)=>n+g.streams.length,0),playable=groups.reduce((n,g)=>n+g.streams.filter(s=>s.playable).length,0);$('no-sources').hidden=total>0;
  if(total)status('sources-status',`${total} sorgenti · ${playable} flussi HTTP/HLS da verificare all’avvio`);
  else if(movie&&addons.some(a=>a.enabled))status('sources-status','Nessun flusso trovato. Prova un altro addon o importa una lista JSON.');
  else status('sources-status',movie?'Aggiungi un addon o importa una lista JSON per questo film.':'Apri una locandina nella chat oppure importa una lista JSON.');
  $('setup-addons').textContent=addons.length?'Gestisci addon':'Aggiungi il primo addon';
}
async function loadStreams(){
  const current=++generation;searchAbort?.abort();searchAbort=new AbortController();const signal=searchAbort.signal;
  groups=groups.filter(g=>g.imported);renderStreams();if(!movie)return;
  const queue=addons.filter(a=>a.enabled);if(queue.length)status('sources-status','Ricerca dei flussi negli addon…');
  async function next(){while(queue.length){const addon=queue.shift();let group={key:addon.key,name:addon.name,streams:[]};
    try{const url=resourceUrl(addon,movie);if(!url){group.error='Identificativo del film non supportato da questo addon.';}else if(!await allowed(url)){group.error='Il collegamento al sito deve essere autorizzato di nuovo.';group.permission=url;}else{group.streams=parseStreams(await fetchJson(url,{signal}));if(!group.streams.length)group.error='Nessun flusso disponibile per questo film.';}}
    catch(error){group.error=error.message;}if(current!==generation)return;groups.push(group);renderStreams();
  }}
  await Promise.all([next(),next(),next()]);if(current===generation)renderStreams();
}
$('source-filter').onchange=renderStreams;$('refresh-streams').onclick=loadStreams;
async function playStream(stream){
  const current=++playGeneration;selected=stream;stopVideo();failure=null;probeAbort=new AbortController();renderStreams();status('playback-status','Verifica della sorgente e del formato…');
  try{
    // Request directly in the click handler, before asynchronous cleanup can
    // expire Chrome's user activation. Known RD streams include rotating CDNs.
    const origins=playbackPermissions(stream);
    if(!await chrome.permissions.request({origins})){
      for(const origin of origins)if(!await chrome.permissions.contains({origins:[origin]}))throw Object.assign(new Error(`Accesso a ${permissionLabel(origin)} non concesso. Premi Consenti per autorizzare il server video.`),{mediaOrigin:origin});
      throw new Error('Accesso alla sorgente non concesso. Premi Riprova per consentirlo.');
    }
    await engineCleanup;if(current!==playGeneration)return;
    if(current!==playGeneration)return;
    let probe=await probePlayback(stream.url,{signal:probeAbort.signal,timeout:20000});
    if(current!==playGeneration)return;
    let isHls=probe.kind==='hls'||(probe.kind==='unknown'&&stream.kind==='hls');
    // A cold addon/CDN can take longer than the normal probe. When the bytes
    // never arrived we cannot tell MKV from MP4 yet, so retry once before
    // handing a Matroska stream to the native <video> element, which would
    // end immediately without an error.
    if(probe.kind==='unknown'&&probe.timedOut&&!isHls&&!likelyMatroska(stream,probe)){
      status('playback-status','Il server video è lento ad avviarsi. Nuovo tentativo…');
      const retry=await probePlayback(stream.url,{signal:probeAbort.signal,timeout:30000});
      if(current!==playGeneration)return;
      if(retry.kind!=='unknown')probe=retry;else probe={...retry,timedOut:true};
      isHls=probe.kind==='hls'||(probe.kind==='unknown'&&stream.kind==='hls');
    }
    activeProbe=probe;
    if(probe.kind==='page')throw new Error('La sorgente restituisce una pagina o un messaggio del servizio, non un video. Aggiorna le sorgenti o controlla l’addon.');
    const actual=checkCodecs(probe.codecs),declared=checkCodecs(declaredCodecs(stream));
    if(probe.kind==='matroska'||probe.kind==='dash'||stream.kind==='dash'||actual.unsupported.length||(!isHls&&declared.unsupported.length)||(!isHls&&likelyMatroska(stream,probe))){await startIntegrated(stream,current,probe);return;}
    playTimer=setTimeout(()=>{if(current!==playGeneration)return;stopVideo();playbackError('La sorgente non risponde. Riprova o scegli un altro flusso.');},45000);
    document.querySelector('.screen').classList.add('active');
    for(const sub of stream.subtitles){if(/\.vtt(?:\?|$)/i.test(sub.url)){const track=el('track');track.kind='subtitles';track.label=sub.lang;track.srclang=/^[a-z]{2,3}(?:-[a-z]{2})?$/i.test(sub.lang)?sub.lang:'und';track.src=sub.url;video.append(track);}}
    if(isHls&&!video.canPlayType('application/vnd.apple.mpegurl')){
      if(!globalThis.Hls?.isSupported())throw new Error('HLS non è supportato da questo browser. Scegli un altro flusso.');
      hls=new Hls({enableWorker:false,maxBufferLength:30,backBufferLength:15,xhrSetup:xhr=>{xhr.withCredentials=false;}});
      hls.on(Hls.Events.ERROR,async(_,data)=>{
        if(current!==playGeneration)return;
        const failed=data.context?.url??data.response?.url??data.url;
        if(failed){try{if(!await allowed(failed)){if(current!==playGeneration)return;clearTimeout(playTimer);blockedUrl=httpUrl(failed).href;$('grant-media').textContent=`Consenti ${httpUrl(failed).host}`;$('grant-media').hidden=false;status('playback-status','Questo flusso usa un altro server: autorizzalo per continuare.',true);hls?.stopLoad();return;}}catch{}}
        if(data.fatal){hls?.stopLoad();startIntegrated(stream,current,{kind:'hls'}).catch(error=>{if(current===playGeneration)reportPlaybackError(error);});}
      });
      hls.attachMedia(video);hls.loadSource(stream.url);
    }else video.src=stream.url;
    await attemptPlay(current);
  }catch(error){if(current===playGeneration&&error.name!=='AbortError')reportPlaybackError(error);}
}
const clock=seconds=>{seconds=Math.max(0,Number.isFinite(seconds)?Math.floor(seconds):0);const h=Math.floor(seconds/3600),m=Math.floor(seconds/60)%60,s=seconds%60;return h?`${h}:${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')}`:`${m}:${String(s).padStart(2,'0')}`;};
async function startIntegrated(stream,current,probe={}){
  if(current!==playGeneration||engineMode==='integrated')return;probe=probe?.kind?probe:activeProbe??probe;engineMode='integrated';clearTimeout(playTimer);hls?.destroy();hls=null;video.pause();video.removeAttribute('src');video.load();failure=null;
  $('codec-screen').hidden=false;$('codec-controls').hidden=false;document.querySelector('.screen').classList.add('active','codec-active');$('playback-stage').dataset.engine='integrated';status('playback-status','Avvio dei decoder integrati…');
  const active=()=>current===playGeneration&&engineMode==='integrated';
  // High-bitrate remuxes need more than the old 25 s to prefetch the first
  // seconds. Keep the timer generous and let the I/O reader report a real
  // server failure instead of declaring the source dead while it is buffering.
  playTimer=setTimeout(()=>{if(active()){stopVideo();playbackError('La sorgente impiega troppo tempo ad avviarsi. Riprova o scegli un altro flusso.',3);}},90000);
  let ext=containerExtension(stream,probe);
  if(!ext)ext=probe.kind==='video'?'mp4':'mkv';
  const candidate=await createCodecPlayer({container:$('codec-screen'),url:stream.url,kind:probe.kind==='dash'||stream.kind==='dash'?'dash':probe.kind==='hls'||stream.kind==='hls'?'hls':'video',ext,
    onState:state=>{if(!active()||failure)return;$('codec-toggle').textContent=state==='playing'?'Pausa':'Riproduci';status('playback-status',({loading:'Caricamento del video con decoder integrati…',playing:`In riproduzione · ${stream.name} · decoder integrati`,paused:'In pausa',ended:'Riproduzione terminata.'})[state]??state);if(state==='playing'){clearTimeout(playTimer);$('retry-play').hidden=true;$('resume-play').hidden=true;}},
    onTime:(position,duration)=>{if(!active())return;$('codec-seek').max=String(Math.max(1,duration||0));if(!scrubbing)$('codec-seek').value=String(Math.max(0,position));$('codec-time').value=`${clock(position)} / ${clock(duration)}`;},
    onFrame:type=>{if(active())$('codec-screen').dataset[type]='decoded';},
    onTracks:tracks=>{if(!active())return;$('codec-audio').replaceChildren(...tracks.audio.map(t=>new Option(t.label,String(t.id))));$('codec-audio').value=String(tracks.selectedAudio);$('codec-audio-wrap').hidden=tracks.audio.length<2;$('codec-subtitle').replaceChildren(new Option('Disattivati','off'),...tracks.subtitles.map(t=>new Option(t.label,String(t.id))));$('codec-subtitle-wrap').hidden=!tracks.subtitles.length;refreshSelects();},
    onError:error=>{if(active())reportPlaybackError(error);}
  });
  if(!active()){await candidate.destroy();return;}integrated=candidate;candidate.volume(Number($('codec-volume').value));await candidate.start();
}
async function toggleCodec(){if(!integrated)return;try{if(integrated.playing)await integrated.pause();else await integrated.play();}catch{playbackError('Il motore integrato non riesce a riprendere il video. Riprova.',3);}}
$('codec-toggle').onclick=toggleCodec;$('codec-screen').onclick=toggleCodec;
$('codec-seek').oninput=()=>{scrubbing=true;};$('codec-seek').onchange=async()=>{scrubbing=false;try{await integrated?.seek(Number($('codec-seek').value));}catch{playbackError('Il server non consente di raggiungere questa posizione. Riprova.',3);}};
$('codec-volume').oninput=()=>integrated?.volume(Number($('codec-volume').value));
$('codec-audio').onchange=()=>integrated?.audio($('codec-audio').value).catch(()=>playbackError('Traccia audio non disponibile. Scegline un’altra.',3));
$('codec-subtitle').onchange=()=>integrated?.subtitle($('codec-subtitle').value).catch(()=>playbackError('Traccia sottotitoli non disponibile.',3));
$('codec-fullscreen').onclick=()=>{const operation=document.fullscreenElement?document.exitFullscreen():$('playback-stage').requestFullscreen();operation.catch(()=>{});};
$('playback-stage').onkeydown=event=>{if(!integrated||event.target.matches('button,input,select,[role=combobox]'))return;if(event.key===' '){event.preventDefault();toggleCodec();}else if(event.key==='ArrowRight'||event.key==='ArrowLeft'){event.preventDefault();integrated.seek(integrated.currentTime+(event.key==='ArrowRight'?5:-5)).catch(()=>{});}else if(event.key==='f')$('codec-fullscreen').click();};
function reportPlaybackError(error){if(error.mediaOrigin){blockedUrl=error.mediaOrigin;$('grant-media').textContent=`Consenti ${permissionLabel(blockedUrl)}`;$('grant-media').hidden=false;}playbackError(error.message,error.mediaOrigin?4:3,error.recovery);}
function playbackError(message,priority=2,recovery='retry'){if(failure&&failure.priority>priority)return;failure={message,priority,recovery};clearTimeout(playTimer);status('playback-status',message,true);$('retry-play').textContent=recovery==='choose'?'Scegli un altro flusso':'Riprova riproduzione';$('retry-play').hidden=false;}
async function attemptPlay(current=playGeneration){try{await video.play();}catch(error){if(current!==playGeneration||engineMode==='integrated')return;if(error.name==='NotAllowedError'){clearTimeout(playTimer);status('playback-status','Sorgente pronta. Premi Avvia video.');$('resume-play').hidden=false;}else if(error.name!=='AbortError')await startIntegrated(selected,current,activeProbe??{}).catch(error=>{if(current===playGeneration)reportPlaybackError(error);});}}
$('retry-play').onclick=()=>{if(failure?.recovery==='choose'){$('stream-list').scrollIntoView({block:'start'});$('stream-list').querySelector('.stream-row:not(.selected) button')?.focus({preventScroll:true});}else if(selected)playStream(selected);};$('resume-play').onclick=()=>attemptPlay();$('grant-media').onclick=async()=>{try{if(blockedUrl&&await grant(blockedUrl))await playStream(selected);else status('playback-status','Accesso non concesso. Premi Consenti per autorizzare il server video.',true);}catch{status('playback-status','Chrome non ha concesso il permesso. Riprova dal pulsante Consenti.',true);}};
video.addEventListener('playing',()=>{if(engineMode==='integrated')return;failure=null;clearTimeout(playTimer);status('playback-status',selected?`In riproduzione · ${selected.name}`:'In riproduzione');$('retry-play').hidden=true;$('resume-play').hidden=true;});
video.addEventListener('waiting',()=>{if(engineMode==='native'&&selected&&!failure)status('playback-status','Caricamento del video…');});
video.addEventListener('pause',()=>{if(engineMode==='native'&&selected&&!failure&&!video.ended&&!video.error)status('playback-status','In pausa');});
video.addEventListener('ended',()=>{
  if(engineMode!=='native'||!selected||failure)return;
  // Chromium accepts a Matroska source just long enough to emit ended at
  // t=0 but never fires an error. Treat that as an unsupported container and
  // finish the playback with the integrated decoders instead of a dead UI.
  const empty=video.currentTime<.05&&(!Number.isFinite(video.duration)||video.duration<.05);
  if(empty){status('playback-status','Il formato richiede i decoder integrati. Avvio…');startIntegrated(selected,playGeneration,activeProbe??{}).catch(error=>{if(!error?.name||error.name!=='AbortError')reportPlaybackError(error);});return;}
  status('playback-status','Riproduzione terminata.');
});
video.addEventListener('error',()=>{if(selected&&engineMode==='native'&&!failure)startIntegrated(selected,playGeneration,activeProbe??{}).catch(error=>reportPlaybackError(error));});
async function importStreams(data,name){const streams=parseStreams(data);if(!streams.length)throw new Error('La lista non contiene flussi riconoscibili.');groups=groups.filter(g=>g.key!=='import');groups.push({key:'import',name,streams,imported:true});renderStreams();status('import-status',`${streams.length} flussi importati per questa sessione. La lista non viene salvata.`);}
$('stream-file').onchange=async()=>{const file=$('stream-file').files[0];if(!file)return;try{if(file.size>MAX_JSON)throw new Error('File troppo grande: massimo 2 MB.');let data;try{data=JSON.parse(await file.text());}catch{throw new Error('Il file non contiene JSON valido.');}await importStreams(data,file.name);}catch(error){status('import-status',error.message,true);}finally{$('stream-file').value='';}};
$('json-url-form').onsubmit=async event=>{event.preventDefault();const button=event.submitter;try{const url=httpUrl($('json-url').value).href;button.disabled=true;if(!await grant(url))throw new Error('Accesso al sito non concesso.');await importStreams(await fetchJson(url),'Lista JSON');$('json-url').value='';}catch(error){status('import-status',error.message,true);}finally{button.disabled=false;}};
chrome.storage.onChanged.addListener(async(changes,area)=>{if(area==='local'&&Object.keys(changes).some(k=>k.startsWith(prefix))){await readAddons();await loadStreams();}});
window.addEventListener('pagehide',()=>{playGeneration++;searchAbort?.abort();stopVideo();});
async function boot(){
  enhanceSelects();tab(location.hash==='#addons'?'addons':'player');await readAddons();
  const id=Number(new URL(location.href).searchParams.get('movie'));
  if(Number.isSafeInteger(id)&&id>0){
    status('film-meta','Caricamento del film da TMDB…');
    const result=await chrome.runtime.sendMessage({type:'watch-movie',id});
    if(result?.ok){movie=result.item;$('film-title').textContent=movie.title;document.title=`${movie.title} · CineMatch`;status('film-meta',[movie.year,movie.directors.map(p=>p.name).join(', ')].filter(Boolean).join(' · '));$('film-source').href=`https://www.themoviedb.org/movie/${id}`;$('film-source').hidden=false;}
    else status('film-meta',result?.error??'Film non disponibile. Riapri la locandina o controlla TMDB nelle opzioni.',true);
  }
  await loadStreams();
}
boot().catch(()=>{status('sources-status','Impossibile caricare il player. Ricarica la scheda per riprovare.',true);});
