import {httpUrl,manifestUrl,permissionOrigin,parseManifest,resourceUrl,parseStreams,fetchJson,MAX_JSON} from "./streams.js";
import {declaredCodecs,compatibility,likelyIntegrated} from "./playback.js";
import {createPlayer} from "./player-engine.js";
import {permissionLabel} from "./media-access.js";

const $=id=>document.getElementById(id);
const el=(tag,text,className)=>{const node=document.createElement(tag);if(text!==undefined)node.textContent=text;if(className)node.className=className;return node;};
const send=async message=>{const result=await chrome.runtime.sendMessage(message);if(!result?.ok)throw new Error(result?.error??"CineMatch non disponibile.");return result;};
const reducedMotion=matchMedia("(prefers-reduced-motion: reduce)");
const formatTime=seconds=>{seconds=Math.max(0,Number.isFinite(seconds)?Math.floor(seconds):0);const h=Math.floor(seconds/3600),m=Math.floor(seconds/60)%60,s=seconds%60;return h?`${h}:${String(m).padStart(2,"0")}:${String(s).padStart(2,"0")}`:`${m}:${String(s).padStart(2,"0")}`;};
const formatRuntime=minutes=>{const value=Number(minutes);if(!Number.isFinite(value)||value<1)return "";const h=Math.floor(value/60),m=value%60;return h?`${h}h ${m}m`:`${m}m`;};

const state={movie:null,addons:[],groups:[],filter:"all",selected:null,recommended:null,recommendedId:null,resume:0,view:"picker",muted:false,lastVolume:1,scrubbing:false,loading:false,generation:0,poster:null,toastTimer:null,stack:null};

/* ---------- Player ---------- */
const player=createPlayer({
  video:$("video"),
  codecContainer:$("codecScreen"),
  onStatus:text=>handleStatus(text),
  onTime:(position,duration)=>handleTime(position,duration),
  onState:name=>handlePlayerState(name),
  onTracks:tracks=>handleTracks(tracks),
  onFrame:type=>{ $("codecScreen").dataset[type]="decoded";if(type==="video")setPlayerLoading(false); },
  onError:error=>showPlayerError(error)
});

function handleStatus(text){
  if(!text)return;
  if(/Verifica|Avvio dei decoder|Preparo/.test(text)){setPlayerLoading(true);}
  if(/Audio non supportato/.test(text))toast(text);
}
function handleTime(position,duration){
  if(!state.scrubbing&&duration>0){$("seek").value=String(Math.round(Math.max(0,Math.min(1,position/duration))*1000));$("seek").parentElement.style.setProperty("--progress",`${Math.max(0,Math.min(100,position/duration*100))}%`);}
  $("time").textContent=`${formatTime(position)} / ${formatTime(duration)}`;
}
function handlePlayerState(name){
  if(name==="loading"){setPlayerLoading(true);$("bigPlay").hidden=true;}
  else if(name==="playing"){setPlayerLoading(false);$("bigPlay").hidden=true;hidePlayerMessage();$("playPause").innerHTML=PAUSE_ICON;$("playPause").setAttribute("aria-label","Pausa");$("stage").classList.add("playing");showControls();}
  else if(name==="paused"){setPlayerLoading(false);$("playPause").innerHTML=PLAY_ICON;$("playPause").setAttribute("aria-label","Riproduci");showControls();saveResume();}
  else if(name==="ended"){setPlayerLoading(false);$("playPause").innerHTML=PLAY_ICON;$("playPause").setAttribute("aria-label","Riproduci");showControls();saveResume();toast("Riproduzione terminata.");}
  else if(name==="needs-gesture"){setPlayerLoading(false);$("bigPlay").hidden=false;}
  updatePiP();
}
function handleTracks(tracks){
  const audio=tracks?.audio??[],subtitles=tracks?.subtitles??[];
  $("audioWrap").hidden=audio.length<2;
  $("audioTrack").replaceChildren(...audio.map(track=>new Option(track.label,String(track.id))));
  if(audio.length)$("audioTrack").value=String(audio.find(track=>track.id===tracks.selectedAudio)?.id??audio[0].id);
  $("subtitleWrap").hidden=!subtitles.length;
  $("subtitleTrack").replaceChildren(new Option("Disattivati","off"),...subtitles.map(track=>new Option(track.label,String(track.id))));
  $("captions").hidden=!subtitles.length;
  if(!subtitles.length){$("captions").setAttribute("aria-pressed","false");$("subtitleTrack").value="off";}
  updatePiP();
}
function showPlayerError(error){
  setPlayerLoading(false);
  $("bigPlay").hidden=true;
  const message=$("playerMessage"),text=$("messageText"),action=$("messageAction");
  message.hidden=false;
  text.textContent=error?.message??"Non riesco a riprodurre questa sorgente.";
  if(error?.mediaOrigin){
    action.hidden=false;action.textContent=`Consenti ${permissionLabel(error.mediaOrigin)}`;action.onclick=()=>{hidePlayerMessage();player.retry().catch(showPlayerError);};
  }else if(player.stream){
    action.hidden=false;action.textContent="Riprova";action.onclick=()=>{hidePlayerMessage();player.retry().catch(showPlayerError);};
  }else action.hidden=true;
}
function hidePlayerMessage(){$("playerMessage").hidden=true;$("messageAction").hidden=true;}
function setPlayerLoading(visible){$("playerLoading").hidden=!visible;}
function updatePiP(){const button=$("pip");if(!button)return;button.hidden=player.mode!=="native"||!document.pictureInPictureEnabled;}

/* ---------- Controls ---------- */
const PLAY_ICON='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z"/></svg>';
const PAUSE_ICON='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 5h4v14H7zM13 5h4v14h-4z"/></svg>';
const VOLUME_ICON='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M17 8a6 6 0 0 1 0 8"/></svg>';
const MUTE_ICON='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9h4l5-4v14l-5-4H4z"/><path d="M17 9l4 6m0-6-4 6"/></svg>';
$("playPause").innerHTML=PLAY_ICON;
$("mute").innerHTML=VOLUME_ICON;
$("mute").classList.add("stroke");
$("playPause").onclick=()=>player.paused?player.play().catch(showPlayerError):player.pause();
$("bigPlay").onclick=()=>{$("bigPlay").hidden=true;player.play().catch(showPlayerError);};
$("seek").oninput=()=>{state.scrubbing=true;const duration=player.duration;if(duration>0){const position=Number($("seek").value)/1000*duration;$("time").textContent=`${formatTime(position)} / ${formatTime(duration)}`;$("seek").parentElement.style.setProperty("--progress",`${Number($("seek").value)/10}%`);}};
$("seek").onchange=()=>{state.scrubbing=false;const duration=player.duration;if(duration>0)player.seek(Number($("seek").value)/1000*duration).catch(showPlayerError);};
$("volume").oninput=()=>{state.lastVolume=Number($("volume").value)||0;state.muted=state.lastVolume===0;player.setVolume(state.lastVolume);$("mute").innerHTML=state.muted?MUTE_ICON:VOLUME_ICON;};
$("mute").onclick=()=>{state.muted=!state.muted;if(state.muted){state.lastVolume=Number($("volume").value)||1;$("volume").value="0";player.setVolume(0);}else{$("volume").value=String(state.lastVolume||1);player.setVolume(state.lastVolume||1);}$("mute").innerHTML=state.muted?MUTE_ICON:VOLUME_ICON;};
$("audioTrack").onchange=()=>player.setAudio($("audioTrack").value).catch(()=>toast("Traccia audio non disponibile."));
$("subtitleTrack").onchange=()=>{player.setSubtitle($("subtitleTrack").value).catch(()=>toast("Sottotitoli non disponibili."));$("captions").setAttribute("aria-pressed",String($("subtitleTrack").value!=="off"));};
$("captions").onclick=()=>{const pressed=$("captions").getAttribute("aria-pressed")==="true";const next=pressed?"off":($("subtitleTrack").options[1]?.value??"off");$("subtitleTrack").value=next;$("subtitleTrack").dispatchEvent(new Event("change",{bubbles:true}));};
$("fullscreen").onclick=()=>{const operation=document.fullscreenElement?document.exitFullscreen():$("stage").requestFullscreen();operation.catch(()=>toast("Schermo intero non disponibile."));};
$("pip").onclick=async()=>{try{if(document.pictureInPictureElement)await document.exitPictureInPicture();else await $("video").requestPictureInPicture();}catch{toast("Picture-in-Picture non disponibile.");}};
$("openDrawer").onclick=openDrawer;$("openDrawerTop").onclick=openDrawer;$("closeDrawer").onclick=closeDrawer;$("drawerBackdrop").onclick=closeDrawer;
$("backToPicker").onclick=showPicker;
$("playBest").onclick=()=>{if(state.recommended)startStream(state.recommended);};
$("resumePlay").onclick=()=>{const stream=state.recommended||state.groups.flatMap(group=>group.streams).find(item=>item.playable);if(stream)startStream(stream,{resumeAt:state.resume});};
$("refreshSources").onclick=()=>loadStreams();
$("manageAddons").onclick=()=>state.view==="addons"?showPicker():showAddons();
$("addonsBack").onclick=showPicker;
$("importJson").onclick=()=>$("jsonFile").click();

/* ---------- Player visibility ---------- */
function showPlayer(stream){
  state.view="player";state.selected=stream.id;
  document.body.classList.add("player-mode");document.body.classList.remove("addons-mode");
  $("pickerView").hidden=true;$("addonsView").hidden=true;$("playerView").hidden=false;
  $("manageAddons").textContent="Addon";$("topbarTitle").textContent="Scegli come guardarlo";
  $("playerTitle").textContent=state.movie?.title??"Film";
  const presentation=present(stream);
  $("playerSource").textContent=`${stream.groupName??"Sorgente"} · ${presentation.quality}`;
  $("seek").value="0";$("seek").parentElement.style.setProperty("--progress","0%");$("time").textContent="0:00 / 0:00";
  delete $("codecScreen").dataset.audio;delete $("codecScreen").dataset.video;
  $("stage").classList.remove("hide-chrome");
  hidePlayerMessage();setPlayerLoading(true);$("bigPlay").hidden=true;
  renderDrawer();renderSources();
  $("stage").focus?.({preventScroll:true});
}
function showPicker(){
  saveResume();
  player.pause().catch(()=>{});
  state.view="picker";
  state.selected=null;
  document.body.classList.remove("player-mode","addons-mode");
  $("playerView").hidden=true;$("addonsView").hidden=true;$("pickerView").hidden=false;
  $("manageAddons").textContent="Addon";$("topbarTitle").textContent="Scegli come guardarlo";
  closeDrawer();renderSources();
  $("playBest").focus({preventScroll:true});
  updateResume();
}
function showAddons(){
  saveResume();
  player.pause().catch(()=>{});
  state.view="addons";
  document.body.classList.remove("player-mode");document.body.classList.add("addons-mode");
  $("pickerView").hidden=true;$("playerView").hidden=true;$("addonsView").hidden=false;
  $("manageAddons").textContent="Film";$("topbarTitle").textContent="Addon e player";
  renderAddons();
  $("addonsBack").focus({preventScroll:true});
}
function setAddonStatus(message,type=""){
  const status=$("addonStatus");status.textContent=message;status.className=`cm-addon-status ${type}`.trim();
}
$("addonForm").onsubmit=async event=>{
  event.preventDefault();
  const form=event.currentTarget,button=form.querySelector('button[type="submit"]');
  try{
    const url=manifestUrl($("addonUrl").value);
    button.disabled=true;
    setAddonStatus(`Collegamento a ${httpUrl(url).host}…`);
    if(!await chrome.permissions.request({origins:[permissionOrigin(url)]}))throw new Error("Accesso al sito non concesso. Riprova e consenti il collegamento.");
    if(state.addons.length>=20)throw new Error("Hai già 20 addon. Rimuovine uno prima di aggiungerne altri.");
    const addon=parseManifest(await fetchJson(url),url);
    const digest=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(url));
    const key="streamAddon:"+Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,"0")).join("");
    await chrome.storage.local.set({[key]:addon});
    $("addonUrl").value="";
    setAddonStatus(`${addon.name} collegato. Torna al film per scegliere una sorgente.`,"success");
    await loadAddons();renderAddons();await loadStreams();
  }catch(error){
    setAddonStatus(error.message??"Non è stato possibile collegare l’addon.","error");
  }finally{
    button.disabled=false;
  }
};
function renderAddons(){
  const list=$("addonList");list.replaceChildren();
  $("addonCount").textContent=state.addons.length?`${state.addons.length} collegati`:"";
  $("addonEmpty").hidden=state.addons.length>0;
  for(const addon of state.addons){
    const row=el("div",undefined,"cm-addon-row");
    const check=document.createElement("input");check.type="checkbox";check.className="cm-addon-check";check.checked=addon.enabled!==false;check.setAttribute("aria-label",`Attiva ${addon.name}`);
    check.onchange=async()=>{
      const saved=(await chrome.storage.local.get(addon.key))[addon.key];if(!saved)return;
      saved.enabled=check.checked;await chrome.storage.local.set({[addon.key]:saved});
      await loadAddons();renderAddons();await loadStreams();
    };
    const main=el("div",undefined,"main");main.append(el("strong",addon.name));
    let host="";try{host=httpUrl(addon.url).host;}catch{}
    main.append(el("span",[host,addon.version].filter(Boolean).join(" · ")));
    const remove=el("button","Rimuovi","cm-addon-remove");remove.type="button";remove.setAttribute("aria-label",`Rimuovi ${addon.name}`);
    remove.onclick=async()=>{await chrome.storage.local.remove(addon.key);await loadAddons();renderAddons();await loadStreams();};
    row.append(check,main,remove);list.append(row);
  }
}
function openDrawer(){
  $("drawer").classList.add("open");$("drawerBackdrop").classList.add("open");$("drawer").focus({preventScroll:true});
}
function closeDrawer(){ $("drawer").classList.remove("open");$("drawerBackdrop").classList.remove("open"); }

let controlsTimer;
function showControls(){ $("stage").classList.remove("hide-chrome");clearTimeout(controlsTimer);if(!player.paused&&!reducedMotion.matches)controlsTimer=setTimeout(()=>$("stage").classList.add("hide-chrome"),2600); }
$("stage").addEventListener("mousemove",showControls);
$("stage").addEventListener("mouseleave",()=>{if(!player.paused)$("stage").classList.add("hide-chrome");});
$("stage").addEventListener("click",event=>{if(event.target.closest("button,input,select,label,a"))return;if(state.view==="player")player.paused?player.play().catch(showPlayerError):player.pause();});

/* ---------- Streams ---------- */
function present(stream){
  if(!state.presented)state.presented=new Map();
  if(state.presented.has(stream))return state.presented.get(stream);
  const text=[stream.name,stream.description,stream.filename].filter(Boolean).join(" ");
  const resolution=/2160p|4k|uhd/i.test(text)?"4K":/1080p/i.test(text)?"1080p":/720p/i.test(text)?"720p":/480p|\bsd\b/i.test(text)?"SD":null;
  const hdr=/dolby.?vision|\bdv\b/i.test(text)?"Dolby Vision":/hdr10\+?/i.test(text)?"HDR10":/\bhdr\b/i.test(text)?"HDR":null;
  const codec=/hevc|[hx][. ]?265/i.test(text)?"HEVC":/\bav1\b/i.test(text)?"AV1":/avc|[hx][. ]?264/i.test(text)?"H.264":null;
  const audio=/atmos/i.test(text)?"Atmos":/truehd/i.test(text)?"TrueHD":/\bdts\b/i.test(text)?"DTS":/e[. -]?ac[. -]?3|ddp|dd\+/i.test(text)?"E-AC3":/ac[. -]?3/i.test(text)?"AC3":/aac/i.test(text)?"AAC":null;
  const languages=[];if(/\bita\b|italian|italiano/i.test(text))languages.push("ITA");if(/\beng\b|english|inglese/i.test(text))languages.push("ENG");if(/sub[. -]?ita/i.test(text))languages.push("SUB-ITA");
  const size=text.match(/(\d+(?:[.,]\d+)?)\s*(GB|MB)/i);
  const compatibilityResult=compatibility(declaredCodecs(stream),mime=>$("video").canPlayType(mime));
  const integrated=likelyIntegrated(stream);
  const base=resolution?(resolution==="1080p"?80:resolution==="720p"?60:resolution==="SD"?40:100):50;
  const score=base+(hdr?6:0)+(codec==="H.264"?4:0)+(compatibilityResult.unsupported.length?0:4)+(stream.playable?0:-1000);
  const badges=[];
  if(resolution)badges.push({text:resolution,tone:"q"});
  if(hdr)badges.push({text:hdr,tone:"warn"});
  if(codec)badges.push({text:codec,tone:codec==="H.264"?"good":"warn"});
  if(audio)badges.push({text:audio,tone:audio==="AAC"?"good":""});
  if(languages.length)badges.push({text:languages.join("/"),tone:""});
  if(size)badges.push({text:`${size[1]} ${size[2].toUpperCase()}`,tone:""});
  if(stream.kind==="hls")badges.push({text:"HLS",tone:"q"});
  if(stream.kind==="dash")badges.push({text:"DASH",tone:"q"});
  if(compatibilityResult.unsupported.length)badges.push({text:"Decoder integrato",tone:"good"});
  else if(stream.playable)badges.push({text:integrated?"Decoder integrato":"Compatibilità nativa",tone:"good"});
  const quality=resolution??({hls:"HLS",dash:"DASH",video:"Video",external:"Esterno",torrent:"Torrent"}[stream.kind]??"Sorgente");
  const presentation={quality,score,badges};
  state.presented.set(stream,presentation);return presentation;
}

function renderChips(){
  if(state.filter!=="all"&&!state.groups.some(group=>group.key===state.filter))state.filter="all";
  const container=$("addonChips");container.replaceChildren();
  if(!state.groups.length){container.hidden=true;return;}
  container.hidden=false;
  const all=el("button","Tutti · "+state.groups.reduce((total,group)=>total+group.streams.length,0),"cm-chip");all.type="button";all.setAttribute("aria-pressed",String(state.filter==="all"));all.onclick=()=>{state.filter="all";renderChips();renderSources();};container.append(all);
  for(const group of state.groups){const chip=el("button",`${group.name} · ${group.streams.length}`,"cm-chip");chip.type="button";chip.setAttribute("aria-pressed",String(state.filter===group.key));chip.onclick=()=>{state.filter=group.key;renderChips();renderSources();};container.append(chip);}
}
function renderSkeletons(){
  const list=$("sourceList");list.replaceChildren();
  for(let i=0;i<4;i++){const skeleton=el("div",undefined,"cm-skeleton");skeleton.innerHTML='<span class="sk dot"></span><div><div class="sk line"></div><div class="sk line short"></div><div class="sk line tiny"></div></div><span class="sk pill"></span>';list.append(skeleton);}
}
function renderEmptyState(title,message,actionLabel="Gestisci addon"){
  const list=$("sourceList");list.replaceChildren();
  const empty=el("div",undefined,"cm-empty");
  empty.innerHTML=`<div class="empty-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5h16v14H4z"/><path d="M9 10l6 3-6 3z"/></svg></div><h3>${escapeText(title)}</h3><p>${escapeText(message)}</p><button class="cm-primary" type="button" data-addons>${escapeText(actionLabel)}</button>`;
  empty.querySelector("[data-addons]").onclick=showAddons;list.append(empty);
  $("sourcesStatus").textContent=message;
}
const escapeText=value=>String(value).replace(/[&<>"']/g,character=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[character]));
function renderEmpty(){
  renderEmptyState("Nessun addon collegato","Collega un addon compatibile con Stremio o Nuvio per vedere qui le sorgenti di questo film. Puoi anche importare una lista JSON.","Collega il primo addon");
}
function renderNoActiveAddons(){
  renderEmptyState("Nessun addon attivo","Tutti gli addon collegati sono disattivati. Riattivane uno per vedere le sorgenti di questo film.","Gestisci addon");
}
function renderErrorCard(group){
  const card=el("div",undefined,"cm-error-card");
  const dot=el("span",undefined,"err-dot"),body=el("div"),title=el("strong",group.name),message=el("span",group.error);
  body.append(title,message);
  const action=el("button",group.permission?"Ricollega":"Riprova");action.type="button";
  action.onclick=async()=>{
    if(group.permission){try{if(!await chrome.permissions.request({origins:[permissionOrigin(group.permission)]})){toast("Accesso al sito non concesso.");return;}}catch{toast("Permesso non disponibile.");return;}}
    await loadStreams();
  };
  card.append(dot,body,action);return card;
}
function renderSourceCard(stream,compact=false){
  const presentation=present(stream);
  const card=el("article",undefined,`cm-source-card${stream.id===state.recommendedId?" recommended":""}${stream.playable?"":" unavailable"}${stream.id===state.selected?" current":""}`);
  const play=el("button",undefined,"source-play");play.type="button";
  play.setAttribute("aria-label",stream.playable?`Riproduci ${stream.name} da ${stream.groupName??"addon"}`:`${stream.name}: non disponibile`);
  play.innerHTML=stream.playable?PLAY_ICON:'<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 6v7m0 4h.01"/></svg>';
  play.disabled=!stream.playable;play.onclick=event=>{event.stopPropagation();if(stream.playable)startStream(stream);};
  const main=el("div",undefined,"cm-source-main"),name=el("div",undefined,"cm-source-name");
  name.textContent=stream.name;if(stream.id===state.recommendedId)name.append(el("span","Consigliata","rec"));main.append(name);
  if(!compact&&stream.filename){const file=el("div",stream.filename,"cm-source-file");file.title=stream.filename;main.append(file);}
  const badges=el("div",undefined,"cm-badges");
  for(const badge of presentation.badges)badges.append(el("span",badge.text,`cm-badge ${badge.tone}`.trim()));
  main.append(badges);
  if(!stream.playable&&stream.reason)main.append(el("p",stream.reason,"cm-source-reason"));
  const side=el("div",undefined,"cm-source-side");side.append(el("span",stream.groupName??"", "cm-source-addon"));
  if(stream.playable){const button=el("button",stream.id===state.selected?"In corso":"Riproduci","cm-play");button.type="button";button.onclick=event=>{event.stopPropagation();startStream(stream);};side.append(button);}
  else if(stream.kind==="external"&&stream.externalUrl){const link=el("a","Apri sul sito","cm-play");link.href=stream.externalUrl;link.target="_blank";link.rel="noopener noreferrer";side.append(link);}
  else side.append(el("span","Non disp.","cm-badge warn"));
  card.append(play,main,side);
  if(stream.playable)card.addEventListener("click",event=>{if(!event.target.closest("button,a"))startStream(stream);});
  return card;
}
function renderSources(){
  $("sourcesStatus").classList.remove("error");
  const list=$("sourceList");
  if(!state.addons.length){renderEmpty();$("playBest").disabled=true;return;}
  if(!state.addons.some(addon=>addon.enabled)){renderNoActiveAddons();$("playBest").disabled=true;return;}
  if(state.loading&&!state.groups.length){renderSkeletons();$("sourcesStatus").textContent="Interrogo gli addon collegati…";$("playBest").disabled=true;return;}
  list.replaceChildren();
  if(!state.groups.length){renderSkeletons();return;}
  const all=state.groups.flatMap(group=>group.streams.map(stream=>({stream,group})));
  all.sort((a,b)=>present(b.stream).score-present(a.stream).score);
  const best=all.find(item=>item.stream.playable);
  state.recommended=best?.stream??null;state.recommendedId=best?.stream.id??null;
  $("playBest").disabled=!state.recommended;
  const visible=all.filter(({group})=>state.filter==="all"||group.key===state.filter);
  const errors=state.groups.filter(group=>group.error&&(state.filter==="all"||state.filter===group.key));
  if(!visible.length&&!errors.length){const empty=el("div",undefined,"cm-empty");empty.innerHTML='<h3>Nessun flusso per questo filtro</h3><p>Prova un altro addon o aggiorna le sorgenti.</p>';list.append(empty);$("sourcesStatus").textContent="Nessun flusso per questo filtro.";return;}
  for(const {stream} of visible)list.append(renderSourceCard(stream));
  for(const group of errors)list.append(renderErrorCard(group));
  const playable=all.filter(item=>item.stream.playable).length;
  $("sourcesStatus").textContent=`${all.length} sorgenti · ${playable} riproducibili · aggiornate ora`;
}
function renderDrawer(){
  const list=$("drawerList");list.replaceChildren();
  const streams=state.groups.flatMap(group=>group.streams).filter(stream=>stream.playable);
  for(const stream of streams){const card=renderSourceCard(stream,true);card.addEventListener("click",event=>{if(event.target.closest("button"))return;startStream(stream);closeDrawer();});list.append(card);}
  if(!streams.length)list.append(el("p","Nessuna sorgente riproducibile.","muted"));
}
async function loadAddons(){
  const data=await chrome.storage.local.get(null);
  state.addons=Object.entries(data).filter(([key])=>key.startsWith("streamAddon:")).flatMap(([key,value])=>{
    try{const addon=parseManifest({id:value.id,name:value.name,version:value.version,description:value.description,resources:value.resources.map(resource=>({name:"stream",types:resource.types,idPrefixes:resource.prefixes})),types:["movie"]},value.url);return [{...addon,key,enabled:value.enabled!==false}];}catch{return [];}
  }).sort((a,b)=>a.name.localeCompare(b.name,"it"));
}
async function loadStreams(){
  if(!state.movie){state.loading=false;return;}
  const current=++state.generation;state.loading=true;state.groups=[];state.selected=null;state.recommended=null;state.recommendedId=null;state.presented=new Map();
  renderChips();renderSources();
  const queue=state.addons.filter(addon=>addon.enabled);
  if(!queue.length){state.loading=false;renderSources();return;}
  async function worker(){
    while(queue.length){
      const addon=queue.shift();const group={key:addon.key,name:addon.name,streams:[],error:null,permission:null};
      try{
        const url=resourceUrl(addon,state.movie);
        if(!url)group.error="Identificativo del film non supportato da questo addon.";
        else if(!await chrome.permissions.contains({origins:[permissionOrigin(url)]})){group.error="Il collegamento al sito deve essere autorizzato di nuovo.";group.permission=url;}
        else{const parsed=parseStreams(await fetchJson(url));group.streams=parsed.map((stream,index)=>({...stream,id:`${addon.key}:${index}`,groupKey:addon.key,groupName:addon.name}));if(!group.streams.length)group.error="Nessun flusso disponibile per questo film.";}
      }catch(error){group.error=error.message??"Addon non disponibile.";}
      if(current!==state.generation)return;
      state.groups.push(group);state.groups.sort((a,b)=>a.name.localeCompare(b.name,"it"));renderChips();renderSources();
    }
  }
  await Promise.all([worker(),worker(),worker()]);
  if(current!==state.generation)return;
  state.loading=false;renderChips();renderSources();
}
async function startStream(stream,{resumeAt=0}={}){
  if(!stream?.playable)return;
  const previous=state.view==="player"&&player.duration>5?Math.max(0,Math.min(1,player.currentTime/player.duration)):null;
  showPlayer(stream);
  try{
    await player.load(stream);
    if(resumeAt>5)await player.seek(resumeAt);
    else if(previous!==null){const duration=player.duration;if(duration>5)await player.seek(previous*duration);}
  }catch(error){showPlayerError(error);}
}

/* ---------- Movie and poster ---------- */
function renderMovie(){
  const movie=state.movie;
  $("movieTitle").textContent=movie.title||"Film";
  $("moviePoster").alt=movie.title?`Locandina di ${movie.title}`:"";
  $("movieOriginal").textContent=movie.originalTitle&&movie.originalTitle!==movie.title?movie.originalTitle:"";
  const meta=[movie.year,formatRuntime(movie.runtime),...(movie.genres??[])].filter(Boolean);
  $("movieMeta").replaceChildren(...meta.map(value=>el("span",value)));
  if(movie.rating){$("movieRating").hidden=false;$("movieRating").replaceChildren();const star=el("span","★","star");$("movieRating").append(star,` ${movie.rating.toLocaleString("it-IT")} `,el("small",`· ${movie.votes?.toLocaleString("it-IT")??"n.d."} voti TMDB`));}else $("movieRating").hidden=true;
  const directors=(movie.directors??[]).map(person=>person.name).slice(0,2);
  const cast=(movie.cast??[]).map(person=>person.name).filter(name=>!directors.includes(name)).slice(0,3);
  $("movieCredits").replaceChildren("Regia di ",el("strong",directors.join(", ")||"n.d."),cast.length?` · con ${cast.join(", ")}`:"");
  $("movieOverview").textContent=movie.overview??"";
  $("tmdbLink").href=`https://www.themoviedb.org/movie/${movie.id}`;$("tmdbLink").hidden=false;
  $("posterFallback").textContent=(movie.title||"CM").split(/\s+/).map(word=>word[0]).join("").slice(0,2).toUpperCase();
  document.title=`${movie.title??"Film"} · CineMatch`;
}
async function loadPoster(){
  if(!state.movie?.poster)return;
  const result=await send({type:"tmdb-image",path:state.movie.poster,size:"w342"});
  $("moviePoster").src=result.data;$("moviePoster").hidden=false;$("posterFallback").hidden=true;
  state.poster=result.data;
}
async function loadResume(){
  if(!state.movie)return;
  const key=`resume:${state.movie.id}`;
  const saved=await chrome.storage.session.get(key);
  state.resume=Number(saved[key])||0;updateResume();
}
function updateResume(){
  const visible=state.resume>5;$("resumePlay").hidden=!visible;if(visible)$("resumeTime").textContent=formatTime(state.resume);
}
function saveResume(){
  if(!state.movie)return;
  const position=player.currentTime;
  if(Number.isFinite(position)&&position>5)chrome.storage.session.set({[`resume:${state.movie.id}`]:position}).catch(()=>{});
}
window.addEventListener("pagehide",saveResume);
setInterval(()=>{if(state.view==="player"&&!player.paused)saveResume();},5000);

/* ---------- Theme and messages ---------- */
function applyTheme(theme,accent){
  document.documentElement.dataset.theme=theme==="light"?"light":"dark";
  if(typeof accent==="string"&&/^#[0-9a-f]{6}$/i.test(accent))document.documentElement.style.setProperty("--accent",accent);
}
window.addEventListener("message",event=>{
  if(event.source!==parent)return;
  if(event.data?.type==="cinematch:theme")applyTheme(event.data.theme,event.data.accent);
});
function closeOverlay(){parent.postMessage({type:"cinematch:close"},"*");}
let toastTimer;
function toast(message){const toast=$("toast");toast.textContent=message;toast.classList.add("show");clearTimeout(toastTimer);toastTimer=setTimeout(()=>toast.classList.remove("show"),2400);}

/* ---------- JSON import ---------- */
$("jsonFile").onchange=async()=>{
  const file=$("jsonFile").files?.[0];if(!file)return;
  try{
    if(file.size>MAX_JSON)throw new Error("File troppo grande: massimo 2 MB.");
    let data;try{data=JSON.parse(await file.text());}catch{throw new Error("Il file non contiene JSON valido.");}
    const streams=parseStreams(data);if(!streams.length)throw new Error("La lista non contiene flussi riconoscibili.");
    const key=`import-${Date.now()}`;const group={key,name:file.name,streams:streams.map((stream,index)=>({...stream,id:`${key}:${index}`,groupKey:key,groupName:file.name})),error:null,permission:null};
    state.groups.push(group);state.loading=false;state.filter="all";renderChips();renderSources();toast(`${streams.length} ${streams.length===1?"flusso importato":"flussi importati"} per questa sessione.`);
  }catch(error){toast(error.message);}finally{$("jsonFile").value="";}
};

/* ---------- Keyboard ---------- */
document.addEventListener("keydown",event=>{
  if(event.key==="Escape"){
    if(document.fullscreenElement)return;
    if($("drawer").classList.contains("open")){event.preventDefault();closeDrawer();return;}
    if(state.view==="addons"){event.preventDefault();showPicker();return;}
    if(state.view==="picker"){event.preventDefault();closeOverlay();return;}
    const target=event.target instanceof Element?event.target:null;
    if(target?.matches("input,select,textarea,button"))return;
    event.preventDefault();closeOverlay();return;
  }
  if(state.view!=="player")return;
  const target=event.target instanceof Element?event.target:null;
  if(target?.matches("input,select,textarea")&&event.key===" ")return;
  if(event.key===" "){event.preventDefault();player.paused?player.play().catch(showPlayerError):player.pause();}
  else if(event.key==="ArrowRight"){event.preventDefault();player.seek(player.currentTime+5).catch(()=>{});}
  else if(event.key==="ArrowLeft"){event.preventDefault();player.seek(Math.max(0,player.currentTime-5)).catch(()=>{});}
  else if(event.key.toLowerCase()==="m")$("mute").click();
  else if(event.key.toLowerCase()==="f")$("fullscreen").click();
  else if(event.key.toLowerCase()==="c")$("captions").click();
});

/* ---------- Boot ---------- */
async function boot(){
  const id=Number(new URL(location.href).searchParams.get("movie"));
  if(!Number.isSafeInteger(id)||id<1)throw new Error("Film non valido.");
  const result=await send({type:"watch-movie",id});
  state.movie=result.item;renderMovie();
  loadPoster().catch(()=>{});
  loadResume().catch(()=>{});
  await loadAddons();renderChips();renderSources();
  updateResume();
  parent.postMessage({type:"cinematch:ready"},"*");
  await loadStreams();
}
boot().catch(error=>{
  $("movieTitle").textContent="Film non disponibile";
  $("movieOriginal").textContent="";
  $("movieMeta").replaceChildren();$("movieRating").hidden=true;$("movieCredits").textContent="";$("movieOverview").textContent="";
  $("posterFallback").textContent="!";
  const status=$("sourcesStatus");status.textContent=error.message;status.classList.add("error");
  const list=$("sourceList");list.replaceChildren();
  const card=el("div",undefined,"cm-error-card");card.append(el("span",undefined,"err-dot"));
  const body=el("div");body.append(el("strong","CineMatch non disponibile"),el("span",error.message));card.append(body);
  const retry=el("button","Riprova");retry.type="button";retry.onclick=()=>location.reload();card.append(retry);list.append(card);
  parent.postMessage({type:"cinematch:ready"},"*");
});
