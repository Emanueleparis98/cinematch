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

const state={movie:null,kind:"movie",season:null,episode:null,episodeTitle:"",seasons:[],episodes:[],seasonGeneration:0,addons:[],groups:[],filter:"all",quality:"all",sourceType:"all",language:"all",selected:null,recommended:null,recommendedId:null,resume:0,view:"picker",muted:false,lastVolume:1,scrubbing:false,loading:false,generation:0,poster:null,toastTimer:null,stack:null,direct:false,directToken:null,directKey:null,directStream:null,directInitial:null,live:false,liveToken:null,liveResult:null,liveTried:new Set(),liveTimer:null,liveGeneration:0};
const embedded=window.parent!==window;
const mediaPhrase=()=>state.kind==="tv"?"questa serie":"questo film";
const episodeLabel=()=>state.kind==="tv"&&state.season!==null&&state.episode!==null?`S${state.season} E${state.episode}`:"";

/* ---------- Player ---------- */
const player=createPlayer({
  video:$("video"),
  codecContainer:$("codecScreen"),
  onStatus:text=>handleStatus(text),
  onTime:(position,duration)=>handleTime(position,duration),
  onState:name=>handlePlayerState(name),
  onTracks:tracks=>handleTracks(tracks),
  onFrame:type=>{ $("codecScreen").dataset[type]="decoded";if(type==="video"){setPlayerLoading(false);clearTimeout(state.liveTimer);} },
  onError:error=>showPlayerError(error)
});

function handleStatus(text){
  if(!text)return;
  if(/Verifica|Avvio dei decoder|Preparo/.test(text)){setPlayerLoading(true);}
  if(/Audio non supportato/.test(text))toast(text);
}
function handleTime(position,duration){
  if(state.live){$("time").textContent="DIRETTA";return;}
  if(!state.scrubbing&&duration>0){$("seek").value=String(Math.round(Math.max(0,Math.min(1,position/duration))*1000));$("seek").parentElement.style.setProperty("--progress",`${Math.max(0,Math.min(100,position/duration*100))}%`);}
  $("time").textContent=`${formatTime(position)} / ${formatTime(duration)}`;
}
function handlePlayerState(name){
  if(state.live&&['paused','needs-gesture','ended'].includes(name))clearTimeout(state.liveTimer);
  if(state.live&&name==="playing"){
    if(player.mode==='native')clearTimeout(state.liveTimer);
    send({type:'live-pause-original',token:state.liveToken}).catch(()=>{});
  }
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
  clearTimeout(state.liveTimer);
  if(state.live&&!error?.mediaOrigin){
    const next=state.groups.flatMap(group=>group.streams).find(stream=>!state.liveTried.has(stream.url));
    if(next&&state.liveTried.size<3){toast('Sorgente non disponibile. Provo la successiva…');startStream(next);return;}
  }
  setPlayerLoading(false);
  $("bigPlay").hidden=true;
  const message=$("playerMessage"),text=$("messageText"),action=$("messageAction");
  message.hidden=false;
  text.textContent=error?.message??"Non riesco a riprodurre questa sorgente.";
  if(error?.mediaOrigin){
    const mediaOrigin=error.mediaOrigin;
    action.hidden=false;action.textContent=`Consenti ${permissionLabel(mediaOrigin)}`;
    action.onclick=async()=>{
      hidePlayerMessage();
      setPlayerLoading(true);
      let granted=false;
      try{granted=await chrome.permissions.request({origins:[mediaOrigin]});}catch{}
      if(!granted){
        showPlayerError(Object.assign(new Error(`Accesso a ${permissionLabel(mediaOrigin)} non concesso. Premi Consenti per autorizzare il server video.`),{mediaOrigin}));
        return;
      }
      try{await player.retry();}catch(retryError){showPlayerError(retryError);}
    };
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
$("qualityFilter").onchange=()=>{state.quality=$("qualityFilter").value;renderSources();};
$("typeFilter").onchange=()=>{state.sourceType=$("typeFilter").value;renderSources();};
$("languageFilter").onchange=()=>{state.language=$("languageFilter").value;renderSources();};
$("resetFilters").onclick=()=>{state.filter="all";state.quality="all";state.sourceType="all";state.language="all";renderChips();renderSources();};
$("manageAddons").onclick=()=>state.view==="addons"?showPicker():showAddons();
$("addonsBack").onclick=showPicker;
$("importJson").onclick=()=>$("jsonFile").click();

/* ---------- Player visibility ---------- */
function showPlayer(stream){
  state.view="player";state.selected=stream.id;
  document.body.classList.add("player-mode");document.body.classList.remove("addons-mode");
  $("pickerView").hidden=true;$("addonsView").hidden=true;$("playerView").hidden=false;
  $("manageAddons").textContent="Addon";$("manageAddons").hidden=state.direct;$("topbarTitle").textContent=state.live?"Live · Flussi della pagina":state.direct?"CB01 · Scegli come guardarlo":"Scegli come guardarlo";
  $("backToPicker").textContent=state.direct?(state.kind==="tv"?"← Episodi":"← Sorgente"):state.kind==="tv"?"← Serie":"← Film";
  $("playerTitle").textContent=state.movie?.title??(state.kind==="tv"?"Serie":"Film");
  const presentation=present(stream),episode=state.kind==="tv"?`${episodeLabel()}${state.episodeTitle?` · ${state.episodeTitle}`:""}`:"";
  $("playerSource").textContent=`${episode?`${episode} · `:""}${stream.groupName??"Sorgente"} · ${presentation.quality}`;
  $("seek").value="0";$("seek").parentElement.style.setProperty("--progress","0%");$("time").textContent="0:00 / 0:00";
  delete $("codecScreen").dataset.audio;delete $("codecScreen").dataset.video;
  $("stage").classList.remove("hide-chrome");
  hidePlayerMessage();setPlayerLoading(true);$("bigPlay").hidden=true;
  renderDrawer();renderSources();notifyView();
  $("stage").focus?.({preventScroll:true});
}
function showPicker(){
  saveResume();
  player.pause().catch(()=>{});
  state.view="picker";
  state.selected=null;
  document.body.classList.remove("player-mode","addons-mode");
  $("playerView").hidden=true;$("addonsView").hidden=true;$("pickerView").hidden=false;
  $("manageAddons").textContent="Addon";$("manageAddons").hidden=state.direct;$("topbarTitle").textContent=state.live?"Live · Flussi della pagina":state.direct?"CB01 · Scegli come guardarlo":"Scegli come guardarlo";
  $("backToPicker").textContent=state.direct?(state.kind==="tv"?"← Episodi":"← Sorgente"):state.kind==="tv"?"← Serie":"← Film";
  closeDrawer();renderSources();notifyView();
  (state.kind==="tv"?$("seasonSelect"):$("playBest")).focus({preventScroll:true});
  updateResume();
}
function showAddons(){
  if(state.direct)return;
  saveResume();
  player.pause().catch(()=>{});
  state.view="addons";
  document.body.classList.remove("player-mode");document.body.classList.add("addons-mode");
  $("pickerView").hidden=true;$("playerView").hidden=true;$("addonsView").hidden=false;
  $("manageAddons").textContent=state.kind==="tv"?"Serie":"Film";$("topbarTitle").textContent="Addon e player";
  $("addonsBack").textContent=state.kind==="tv"?"← Serie":"← Film";
  renderAddons();notifyView();
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
    setAddonStatus(`${addon.name} collegato. Torna al titolo per scegliere una sorgente.`,"success");
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
    const supported=new Set(addon.resources.flatMap(resource=>resource.types));
    const kinds=[supported.has("movie")?"Film":null,supported.has("series")?"Serie TV":null].filter(Boolean).join(" · ");
    main.append(el("span",[host,addon.version,kinds].filter(Boolean).join(" · ")));
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
  const languages=[];
  if(/\bita\b|italian|italiano/i.test(text))languages.push("ITA");
  if(/\beng\b|english|inglese/i.test(text))languages.push("ENG");
  if(/\bfra\b|french|francese/i.test(text))languages.push("FRA");
  if(/\bspa\b|spanish|spagnolo/i.test(text))languages.push("SPA");
  if(/\bger\b|\bdeu\b|german|tedesco/i.test(text))languages.push("GER");
  if(/\bjpn\b|japanese|giapponese/i.test(text))languages.push("JPN");
  if(/\bkor\b|korean|coreano/i.test(text))languages.push("KOR");
  if(/sub[. -]?ita/i.test(text))languages.push("SUB-ITA");
  if(/\bmulti\b|multilingua/i.test(text))languages.push("MULTI");
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
  const presentation={quality,score,badges,resolution,languages};
  state.presented.set(stream,presentation);return presentation;
}
function streamType(stream){
  if(stream.kind==="torrent")return "torrent";
  if(stream.kind==="external")return "external";
  if(stream.kind==="unsupported")return "other";
  return "direct";
}

function renderChips(){
  if(state.direct){const container=$("addonChips");container.replaceChildren();container.hidden=true;return;}
  if(state.filter!=="all"&&!state.groups.some(group=>group.key===state.filter))state.filter="all";
  const container=$("addonChips");container.replaceChildren();
  if(!state.groups.length){container.hidden=true;return;}
  container.hidden=false;
  const all=el("button","Tutti · "+state.groups.reduce((total,group)=>total+group.streams.length,0),"cm-chip");all.type="button";all.setAttribute("aria-pressed",String(state.filter==="all"));all.onclick=()=>{state.filter="all";renderChips();renderSources();};container.append(all);
  for(const group of state.groups){const chip=el("button",`${group.name} · ${group.streams.length}`,"cm-chip");chip.type="button";chip.setAttribute("aria-pressed",String(state.filter===group.key));chip.onclick=()=>{state.filter=group.key;renderChips();renderSources();};container.append(chip);}
}
const QUALITY_FILTERS=[["4K","4K"],["1080p","1080p"],["720p","720p"],["SD","SD"],["other","Altre"]];
const TYPE_FILTERS=[["direct","Diretti"],["torrent","Torrent"],["external","Esterni"],["other","Altri"]];
const LANGUAGE_ORDER=["ITA","ENG","FRA","SPA","GER","JPN","KOR","SUB-ITA","MULTI"];
let filterSignature="";
function fillFilter(select,options,value){
  select.replaceChildren(...options.map(([optionValue,label])=>{const option=document.createElement("option");option.value=optionValue;option.textContent=label;return option;}));
  select.value=options.some(([optionValue])=>optionValue===value)?value:"all";
}
function renderFilters(){
  const bar=$("sourceFilters");
  const groups=state.groups.filter(group=>state.filter==="all"||group.key===state.filter);
  const streams=groups.flatMap(group=>group.streams);
  const totalStreams=state.groups.flatMap(group=>group.streams).length;
  bar.hidden=state.direct||totalStreams<2;
  const qualities=new Map(),types=new Map(),languages=new Map();
  for(const stream of streams){
    const presentation=present(stream);
    const quality=presentation.resolution??"other";
    qualities.set(quality,(qualities.get(quality)??0)+1);
    const type=streamType(stream);types.set(type,(types.get(type)??0)+1);
    if(presentation.languages.length)for(const language of presentation.languages)languages.set(language,(languages.get(language)??0)+1);
    else languages.set("other",(languages.get("other")??0)+1);
  }
  if(state.quality!=="all"&&!qualities.has(state.quality))state.quality="all";
  if(state.sourceType!=="all"&&!types.has(state.sourceType))state.sourceType="all";
  if(state.language!=="all"&&!languages.has(state.language))state.language="all";
  const signature=JSON.stringify([state.filter,[...qualities],[...types],[...languages]]);
  if(signature!==filterSignature){
    filterSignature=signature;
    fillFilter($("qualityFilter"),[["all",`Tutte · ${streams.length}`],...QUALITY_FILTERS.filter(([value])=>qualities.has(value)).map(([value,label])=>[value,`${label} · ${qualities.get(value)}`])],state.quality);
    fillFilter($("typeFilter"),[["all",`Tutti · ${streams.length}`],...TYPE_FILTERS.filter(([value])=>types.has(value)).map(([value,label])=>[value,`${label} · ${types.get(value)}`])],state.sourceType);
    const ordered=[...LANGUAGE_ORDER.filter(language=>languages.has(language)),...[...languages.keys()].filter(language=>language!=="other"&&!LANGUAGE_ORDER.includes(language)).sort()];
    const languageOptions=[["all",`Tutte · ${streams.length}`],...ordered.map(language=>[language,`${language} · ${languages.get(language)}`])];
    if(languages.has("other"))languageOptions.push(["other",`Non indicata · ${languages.get("other")}`]);
    fillFilter($("languageFilter"),languageOptions,state.language);
  }
  $("qualityFilter").value=state.quality;
  $("typeFilter").value=state.sourceType;
  $("languageFilter").value=state.language;
  $("resetFilters").disabled=!(state.filter!=="all"||state.quality!=="all"||state.sourceType!=="all"||state.language!=="all");
}
function matchesFilters(stream,group){
  if(state.filter!=="all"&&group.key!==state.filter)return false;
  const presentation=present(stream);
  if(state.quality!=="all"&&(presentation.resolution??"other")!==state.quality)return false;
  if(state.sourceType!=="all"&&streamType(stream)!==state.sourceType)return false;
  if(state.language!=="all"){
    if(state.language==="other"){if(presentation.languages.length)return false;}
    else if(!presentation.languages.includes(state.language))return false;
  }
  return true;
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
  renderEmptyState("Nessun addon collegato",`Collega un addon compatibile con Stremio o Nuvio per vedere qui le sorgenti di ${mediaPhrase()}. Puoi anche importare una lista JSON.`,"Collega il primo addon");
}
function renderNoActiveAddons(){
  renderEmptyState("Nessun addon attivo",`Tutti gli addon collegati sono disattivati. Riattivane uno per vedere le sorgenti di ${mediaPhrase()}.`,"Gestisci addon");
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
function renderLiveSources(){
  $('sourceFilters').hidden=true;$('addonChips').hidden=true;
  const list=$('sourceList');list.replaceChildren();
  if(state.loading){renderSkeletons();$('sourcesStatus').textContent='Rilevo le sorgenti della pagina e dei player autorizzati…';$('playBest').disabled=true;return;}
  const streams=state.groups.flatMap(group=>group.streams);
  state.recommended=streams[0]??null;state.recommendedId=state.recommended?.id??null;
  $('playBest').disabled=!streams.length;
  $('sourcesStatus').textContent=streams.length?`${streams.length} flussi rilevati · disponibilità verificata all’avvio`:'Nessun flusso diretto rilevato.';
  for(const stream of streams)list.append(renderSourceCard(stream));
  const pending=state.liveResult?.pendingOrigins??[];
  if(pending.length){
    const card=el('div',undefined,'cm-error-card'),body=el('div');
    body.append(el('strong','Il player usa un altro sito'),el('p','Consenti la lettura del player incorporato: '+pending.map(origin=>new URL(origin).hostname).join(', ')));
    const grant=el('button','Consenti e cerca');grant.type='button';
    grant.onclick=async()=>{
      try{
        if(!await chrome.permissions.request({origins:pending})){toast('Accesso non concesso.');return;}
        await loadLiveSources({refresh:true,initial:!streams.length});
      }catch(error){toast(error.message);}
    };
    card.append(body,grant);list.append(card);
  }
  if(!streams.length){
    const empty=el('div',undefined,'cm-empty');
    empty.append(el('h3','Apri il canale o il player dell’evento'),el('p','Il sito può caricare la sorgente soltanto dopo una scelta nel suo player. Torna alla pagina, seleziona il canale e premi di nuovo Flussi. Login, DRM e CAPTCHA richiedono il servizio originale.'));
    list.append(empty);
  }
  list.append(el('p','CineMatch non aggiunge annunci. La pubblicità incorporata nel video dipende dalla sorgente.','muted'));
}
async function loadLiveSources({refresh=false,initial=false}={}){
  const generation=++state.generation;state.loading=true;renderLiveSources();
  try{
    const result=await send({type:refresh?'live-refresh':'live-playback',token:state.liveToken});
    if(generation!==state.generation)return;
    state.liveResult=result;state.movie={title:result.title??'Diretta',key:result.page,genres:['Live TV / Sport']};
    state.groups=[{key:'live',name:result.site??'Live',streams:result.streams.map((stream,index)=>({...stream,live:true,id:`live:${index}`,groupKey:'live',groupName:result.site??'Live'}))}];
    state.liveTried.clear();renderMedia();
    state.loading=false;renderLiveSources();
    if(initial&&state.recommended)await startStream(state.recommended);
  }catch(error){
    if(generation!==state.generation)return;
    state.loading=false;renderLiveSources();$('sourcesStatus').textContent=error.message;$('sourcesStatus').classList.add('error');
  }
}
function renderDirectSources(){
  const list=$("sourceList");list.replaceChildren();
  const stream=state.directStream;
  if(state.loading&&!stream){renderSkeletons();$("sourcesStatus").textContent=state.kind==="tv"?"Cerco la sorgente dell’episodio…":"Cerco la sorgente CB01…";$("playBest").disabled=true;return;}
  if(!stream){
    const empty=el("div",undefined,"cm-empty");
    empty.innerHTML='<h3>Nessuna sorgente pronta</h3><p>Scegli un episodio o riprova dalla pagina CB01.</p>';
    list.append(empty);$("sourcesStatus").textContent="Nessuna sorgente CB01 pronta.";$("playBest").disabled=true;return;
  }
  state.groups=[{key:"cb01",name:"CB01",streams:[stream],error:null,permission:null}];
  state.recommended=stream;state.recommendedId=stream.id;
  list.append(renderSourceCard(stream));
  $("playBest").disabled=!stream.playable;
  $("sourcesStatus").textContent=`CB01 · ${stream.name}${state.kind==="tv"&&episodeLabel()?` · ${episodeLabel()}`:""} · senza pubblicità`;
}
function renderSources(){
  $("sourcesStatus").classList.remove("error");
  if(state.live){renderLiveSources();return;}
  if(state.direct){$("sourceFilters").hidden=true;$("addonChips").hidden=true;renderDirectSources();return;}
  const list=$("sourceList");
  if(!state.addons.length){$("sourceFilters").hidden=true;renderEmpty();$("playBest").disabled=true;return;}
  if(!state.addons.some(addon=>addon.enabled)){$("sourceFilters").hidden=true;renderNoActiveAddons();$("playBest").disabled=true;return;}
  if(state.loading&&!state.groups.length){$("sourceFilters").hidden=true;renderSkeletons();$("sourcesStatus").textContent="Interrogo gli addon collegati…";$("playBest").disabled=true;return;}
  renderFilters();
  list.replaceChildren();
  if(!state.groups.length){$("sourceFilters").hidden=true;renderSkeletons();return;}
  const all=state.groups.flatMap(group=>group.streams.map(stream=>({stream,group})));
  all.sort((a,b)=>present(b.stream).score-present(a.stream).score);
  const best=all.find(item=>item.stream.playable);
  state.recommended=best?.stream??null;state.recommendedId=best?.stream.id??null;
  $("playBest").disabled=!state.recommended;
  const visible=all.filter(({stream,group})=>matchesFilters(stream,group));
  const errors=state.groups.filter(group=>group.error&&(state.filter==="all"||state.filter===group.key));
  if(!visible.length&&!errors.length){const empty=el("div",undefined,"cm-empty");empty.innerHTML='<h3>Nessun flusso per questo filtro</h3><p>Prova un altro addon o aggiorna le sorgenti.</p>';list.append(empty);$("sourcesStatus").textContent="Nessun flusso per questo filtro.";return;}
  for(const {stream} of visible)list.append(renderSourceCard(stream));
  for(const group of errors)list.append(renderErrorCard(group));
  const playable=visible.filter(item=>item.stream.playable).length,episode=state.kind==="tv"?` · ${episodeLabel()}`:"";
  const active=state.filter!=="all"||state.quality!=="all"||state.sourceType!=="all"||state.language!=="all";
  $("sourcesStatus").textContent=active
    ? `${visible.length} di ${all.length} sorgenti · ${playable} riproducibili${episode} · filtri attivi`
    : `${all.length} sorgenti · ${playable} riproducibili${episode} · aggiornate ora`;
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
    try{const addon=parseManifest({id:value.id,name:value.name,version:value.version,description:value.description,resources:value.resources.map(resource=>({name:"stream",types:resource.types,idPrefixes:resource.prefixes})),types:["movie","series"]},value.url);return [{...addon,key,enabled:value.enabled!==false}];}catch{return [];}
  }).sort((a,b)=>a.name.localeCompare(b.name,"it"));
}
async function loadStreams(){
  if(state.live){await loadLiveSources();return;}
  if(state.direct){state.loading=false;renderSources();return;}
  if(!state.movie){state.loading=false;return;}
  const series=state.kind==="tv";
  if(series&&(state.season===null||state.episode===null)){state.loading=false;return;}
  const current=++state.generation;state.loading=true;state.groups=[];state.selected=null;state.recommended=null;state.recommendedId=null;state.presented=new Map();
  renderChips();renderSources();
  const queue=state.addons.filter(addon=>addon.enabled);
  if(!queue.length){state.loading=false;renderSources();return;}
  async function worker(){
    while(queue.length){
      const addon=queue.shift();const group={key:addon.key,name:addon.name,streams:[],error:null,permission:null};
      try{
        if(!addon.resources.some(resource=>resource.types.includes(series?"series":"movie")))throw new Error(series?"Questo addon non supporta le serie TV.":"Questo addon non supporta i film.");
        const url=resourceUrl(addon,state.movie,series?{season:state.season,episode:state.episode}:null);
        if(!url)group.error=series?"Identificativo della serie o dell’episodio non supportato da questo addon.":"Identificativo del film non supportato da questo addon.";
        else if(!await chrome.permissions.contains({origins:[permissionOrigin(url)]})){group.error="Il collegamento al sito deve essere autorizzato di nuovo.";group.permission=url;}
        else{const parsed=parseStreams(await fetchJson(url));group.streams=parsed.map((stream,index)=>({...stream,id:`${addon.key}:${index}`,groupKey:addon.key,groupName:addon.name}));if(!group.streams.length)group.error=series?`Nessun flusso disponibile per ${episodeLabel()}.`:"Nessun flusso disponibile per questo film.";}
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
  const previous=!state.live&&state.view==="player"&&player.duration>5?Math.max(0,Math.min(1,player.currentTime/player.duration)):null;
  if(state.live){
    clearTimeout(state.liveTimer);state.liveTried.add(stream.url);
    const attempt=++state.liveGeneration;
    state.liveTimer=setTimeout(()=>{if(attempt===state.liveGeneration&&state.view==='player')showPlayerError(new Error('La sorgente non ha avviato il video. Riprova o scegli un altro flusso.'));},40000);
  }
  showPlayer(stream);
  try{
    await player.load(stream);
    if(resumeAt>5)await player.seek(resumeAt);
    else if(previous!==null){const duration=player.duration;if(duration>5)await player.seek(previous*duration);}
  }catch(error){showPlayerError(error);}
}

/* ---------- Media, episodes and poster ---------- */
function renderMedia(){
  const item=state.movie,tv=state.kind==="tv";
  if(state.direct){
    const seasons=item.seasons??[];
    $("movieTitle").textContent=item.title||(tv?"Serie":"Film");
    $("moviePoster").alt=item.title?`Locandina di ${item.title}`:"";
    $("movieOriginal").textContent=item.originalTitle&&item.originalTitle!==item.title?item.originalTitle:"";
    const meta=tv?[item.year,seasons.length?`${seasons.length} stagion${seasons.length===1?"e":"i"}`:null,...(item.genres??[])]:[item.year,...(item.genres??[])];
    $("movieMeta").replaceChildren(...meta.filter(Boolean).map(value=>el("span",value)));
    $("movieRating").hidden=true;
    $("movieCredits").replaceChildren(state.live?`${state.liveResult?.site??'Live'} · player CineMatch`:"CB01 · pubblicità bloccata");
    $("movieOverview").textContent="";
    $("tmdbLink").hidden=true;
    $("manageAddons").hidden=true;
    $("posterFallback").textContent=(item.title||"CM").split(/\s+/).map(word=>word[0]).join("").slice(0,2).toUpperCase();
    $("backToPicker").textContent=tv?"← Episodi":"← Sorgente";
    $("episodeControls").hidden=!tv;
    document.title=`${item.title??(tv?"Serie":"Film")} · CineMatch`;
    return;
  }
  $("movieTitle").textContent=item.title||(tv?"Serie":"Film");
  $("moviePoster").alt=item.title?`Locandina di ${item.title}`:"";
  $("movieOriginal").textContent=item.originalTitle&&item.originalTitle!==item.title?item.originalTitle:"";
  const meta=tv?[item.year,item.numberOfSeasons?`${item.numberOfSeasons} stagion${item.numberOfSeasons===1?"e":"i"}`:null,...(item.genres??[])]:[item.year,formatRuntime(item.runtime),...(item.genres??[])];
  $("movieMeta").replaceChildren(...meta.filter(Boolean).map(value=>el("span",value)));
  if(item.rating){$("movieRating").hidden=false;$("movieRating").replaceChildren();const star=el("span","★","star");$("movieRating").append(star,` ${item.rating.toLocaleString("it-IT")} `,el("small",`· ${item.votes?.toLocaleString("it-IT")??"n.d."} voti TMDB`));}else $("movieRating").hidden=true;
  const directors=(item.directors??[]).map(person=>person.name).slice(0,2);
  const cast=(item.cast??[]).map(person=>person.name).filter(name=>!directors.includes(name)).slice(0,3);
  $("movieCredits").replaceChildren(tv?"Creata da ":"Regia di ",el("strong",directors.join(", ")||"n.d."),cast.length?` · con ${cast.join(", ")}`:"");
  $("movieOverview").textContent=item.overview??"";
  $("tmdbLink").href=`https://www.themoviedb.org/${tv?"tv":"movie"}/${item.id}`;$("tmdbLink").hidden=false;
  $("manageAddons").hidden=false;
  $("posterFallback").textContent=(item.title||"CM").split(/\s+/).map(word=>word[0]).join("").slice(0,2).toUpperCase();
  $("backToPicker").textContent=tv?"← Serie":"← Film";
  $("episodeControls").hidden=!tv;
  document.title=`${item.title??(tv?"Serie":"Film")} · CineMatch`;
}
async function loadPoster(){
  if(!state.movie?.poster)return;
  if(state.direct){
    $("moviePoster").src=state.movie.poster;$("moviePoster").hidden=false;$("posterFallback").hidden=true;
    state.poster=state.movie.poster;return;
  }
  const result=await send({type:"tmdb-image",path:state.movie.poster,size:"w342"});
  $("moviePoster").src=result.data;$("moviePoster").hidden=false;$("posterFallback").hidden=true;
  state.poster=result.data;
}
function resumeKey(){
  if(state.live)return null;
  if(!state.movie)return null;
  if(state.direct){
    const key=encodeURIComponent(state.directKey??state.movie.title??"cb01");
    if(state.kind==="tv")return state.season===null||state.episode===null?null:`resume:cb01:${key}:${state.season}:${state.episode}`;
    return `resume:cb01:${key}`;
  }
  if(state.kind==="tv")return state.season===null||state.episode===null?null:`resume:tv:${state.movie.id}:${state.season}:${state.episode}`;
  return `resume:${state.movie.id}`;
}
async function loadResume(){
  const key=resumeKey();if(!key){state.resume=0;updateResume();return;}
  const saved=await chrome.storage.session.get(key);
  state.resume=Number(saved[key])||0;updateResume();
}
function updateResume(){
  const visible=state.resume>5;$("resumePlay").hidden=!visible;if(visible)$("resumeTime").textContent=formatTime(state.resume);
}
function saveResume(){
  const key=resumeKey();if(!key)return;
  const position=player.currentTime;
  if(Number.isFinite(position)&&position>5)chrome.storage.session.set({[key]:position}).catch(()=>{});
}
function seasonLabel(season){return season.season===0?"Speciali":`Stagione ${season.season}`;}
async function setupEpisodes(preferredSeason,preferredEpisode){
  if(state.direct)return setupDirectEpisodes(preferredSeason,preferredEpisode);
  const seasons=(state.movie.seasons??[]).filter(season=>season.episodes>0);
  if(!seasons.length)throw new Error("Nessun episodio disponibile per questa serie.");
  state.seasons=seasons;
  const seasonSelect=$("seasonSelect");
  seasonSelect.replaceChildren(...seasons.map(season=>new Option(`${seasonLabel(season)}${season.episodes?` · ${season.episodes} ep.`:""}`,String(season.season))));
  seasonSelect.disabled=false;
  seasonSelect.onchange=()=>loadSeason(Number(seasonSelect.value),null);
  $("episodeSelect").onchange=()=>setEpisode(Number($("episodeSelect").value));
  const selected=seasons.find(season=>season.season===preferredSeason)??seasons.find(season=>season.season===1)??seasons[0];
  seasonSelect.value=String(selected.season);
  await loadSeason(selected.season,preferredEpisode);
}
async function setupDirectEpisodes(preferredSeason,preferredEpisode){
  const seasons=(state.movie.seasons??[]).filter(season=>season.episodes?.length);
  if(!seasons.length)throw new Error("Questa serie non contiene episodi disponibili.");
  state.seasons=seasons;
  const seasonSelect=$("seasonSelect");
  seasonSelect.replaceChildren(...seasons.map(season=>new Option(`${seasonLabel(season)} · ${season.episodes.length} ep.`,String(season.season))));
  seasonSelect.disabled=false;
  seasonSelect.onchange=()=>loadDirectSeason(Number(seasonSelect.value),null);
  $("episodeSelect").onchange=()=>setDirectEpisode(Number($("episodeSelect").value));
  const selected=seasons.find(season=>season.season===preferredSeason)??[...seasons].sort((a,b)=>b.season-a.season)[0];
  seasonSelect.value=String(selected.season);
  await loadDirectSeason(selected.season,preferredEpisode);
}
async function loadDirectSeason(seasonNumber,preferredEpisode){
  if(!state.direct||!state.movie)return;
  const season=(state.movie.seasons??[]).find(item=>item.season===seasonNumber);
  if(!season)return;
  const current=++state.seasonGeneration;
  state.season=seasonNumber;state.episode=null;state.episodeTitle="";state.episodes=season.episodes;state.resume=0;
  $("seasonSelect").value=String(seasonNumber);
  const select=$("episodeSelect");select.disabled=false;
  select.replaceChildren(...season.episodes.map(episode=>new Option(`Episodio ${episode.episode}${episode.title?` · ${episode.title}`:""}`,String(episode.episode))));
  const selected=season.episodes.find(episode=>episode.episode===preferredEpisode)??season.episodes[0];
  select.value=String(selected.episode);
  if(current!==state.seasonGeneration)return;
  await setDirectEpisode(selected.episode);
}
async function setDirectEpisode(episodeNumber){
  const episode=state.episodes.find(item=>item.episode===episodeNumber);
  if(!episode)return;
  state.episode=episode.episode;state.episodeTitle=episode.title??"";
  $("episodeSelect").value=String(episode.episode);
  $("episodeStatus").hidden=true;$("episodeStatus").textContent="";
  state.loading=true;state.directStream=null;state.groups=[];renderSources();
  try{
    let stream=null;
    if(state.directInitial&&state.directInitial.season===state.season&&state.directInitial.episode===state.episode&&state.directInitial.stream){
      stream=state.directInitial.stream;state.directInitial=null;
    }else{
      const result=await send({type:"cb01-episode",token:state.directToken,season:state.season,episode:state.episode});
      stream=result.stream;
    }
    const normalized={...stream,id:"cb01:0",groupKey:"cb01",groupName:"CB01"};
    state.loading=false;state.directStream=normalized;
    state.groups=[{key:"cb01",name:"CB01",streams:[normalized],error:null,permission:null}];
    document.title=`${state.movie.title} · S${state.season} E${state.episode} · CineMatch`;
    renderSources();
    try{await loadResume();}catch{}
    updateResume();
    await startStream(normalized);
  }catch(error){
    state.loading=false;state.directStream=null;state.groups=[];
    $("sourcesStatus").textContent=error.message;
    $("sourcesStatus").classList.add("error");
    $("sourceList").replaceChildren();$("playBest").disabled=true;
    toast(error.message);
  }
}
async function loadSeason(seasonNumber,preferredEpisode){
  if(!state.movie||state.kind!=="tv")return;
  const current=++state.seasonGeneration;
  state.season=seasonNumber;state.episode=null;state.episodeTitle="";state.episodes=[];state.resume=0;updateResume();
  $("seasonSelect").value=String(seasonNumber);$("episodeSelect").disabled=true;
  const episodeStatus=$("episodeStatus");episodeStatus.hidden=false;episodeStatus.textContent="Carico gli episodi…";
  try{
    const result=await send({type:"watch-season",id:state.movie.id,season:seasonNumber});
    if(current!==state.seasonGeneration)return;
    const episodes=result.item.episodes??[];
    if(!episodes.length)throw new Error("Questa stagione non contiene episodi.");
    state.episodes=episodes;
    const select=$("episodeSelect");
    select.replaceChildren(...episodes.map(episode=>new Option(`Episodio ${episode.episode}${episode.title?` · ${episode.title}`:""}`,String(episode.episode))));
    select.disabled=false;
    const selected=episodes.find(episode=>episode.episode===preferredEpisode)??episodes[0];
    select.value=String(selected.episode);
    await setEpisode(selected.episode);
  }catch(error){
    if(current!==state.seasonGeneration)return;
    episodeStatus.hidden=false;episodeStatus.textContent=error.message;
    const status=$("sourcesStatus");status.textContent=error.message;status.classList.add("error");
    $("sourceList").replaceChildren();$("playBest").disabled=true;
  }
}
async function setEpisode(episodeNumber){
  const episode=state.episodes.find(item=>item.episode===episodeNumber);
  if(!episode)return;
  state.episode=episode.episode;state.episodeTitle=episode.title??"";
  $("episodeSelect").value=String(episode.episode);
  $("episodeStatus").hidden=true;$("episodeStatus").textContent="";
  document.title=`${state.movie.title} · S${state.season} E${state.episode} · CineMatch`;
  await loadResume().catch(()=>{});
  await loadStreams();
}
window.addEventListener("pagehide",saveResume);
setInterval(()=>{if(state.view==="player"&&!player.paused)saveResume();},5000);

/* ---------- Theme and messages ---------- */
function applyTheme(theme,accent){
  document.documentElement.dataset.theme=theme==="light"?"light":"dark";
  if(typeof accent==="string"&&/^#[0-9a-f]{6}$/i.test(accent))document.documentElement.style.setProperty("--accent",accent);
}
let directStart=null,pendingDirect=null;
window.addEventListener("message",event=>{
  if(event.source!==parent)return;
  if(event.data?.type==="cinematch:theme")applyTheme(event.data.theme,event.data.accent);
  else if(event.data?.type==="cinematch:direct"){
    if(directStart){const start=directStart;directStart=null;start(event.data);}
    else pendingDirect=event.data;
  }
});
function waitForDirect(){
  if(pendingDirect){const data=pendingDirect;pendingDirect=null;return Promise.resolve(data);}
  return new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>{if(directStart===done){directStart=null;reject(new Error("La pagina non ha inviato la sorgente CB01. Riprova."));}},20000);
    const done=data=>{clearTimeout(timer);directStart=null;resolve(data);};
    directStart=done;
  });
}
function normalizeDirectStream(stream){
  return {...stream,id:"cb01:0",groupKey:"cb01",groupName:"CB01"};
}
async function startDirect(result){
  if(!result?.media||!result?.stream?.url)throw new Error("Sorgente CB01 non valida.");
  state.direct=true;
  state.directToken=result.token??state.directToken;
  state.directKey=result.media.key??result.media.title??"cb01";
  state.kind=result.media.kind==="tv"?"tv":"movie";
  state.movie={...result.media,seasons:result.media.seasons??[]};
  state.directInitial=result.episode?{...result.episode,stream:result.stream}:null;
  state.directStream=null;state.groups=[];state.loading=true;
  renderMedia();
  loadPoster().catch(()=>{});
  if(state.kind==="tv"){
    await setupEpisodes(result.episode?.season??null,result.episode?.episode??null);
  }else{
    const stream=normalizeDirectStream(result.stream);
    state.directStream=stream;state.groups=[{key:"cb01",name:"CB01",streams:[stream],error:null,permission:null}];state.loading=false;
    try{await loadResume();}catch{}
    updateResume();
    await startStream(stream);
  }
}
function notifyView(){if(embedded)parent.postMessage({type:"cinematch:view",view:state.view},"*");}
function closeOverlay(){if(embedded)parent.postMessage({type:"cinematch:close"},"*");else window.close();}
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
  const params=new URL(location.href).searchParams;
  if(params.get('liveToken')){
    state.live=true;state.direct=true;state.liveToken=params.get('liveToken');
    $('manageAddons').hidden=true;$('importJson').hidden=true;
    $('topbarTitle').textContent='Live · Flussi della pagina';
    $('movieTitle').textContent='Cerco i flussi della pagina…';
    $('seek').parentElement.hidden=true;
    const liveButton=el('button','Torna in diretta');liveButton.type='button';liveButton.id='goLive';
    liveButton.onclick=()=>player.goLive().catch(()=>toast('Ritorno in diretta non disponibile per questo formato.'));
    $('time').after(liveButton);
    if(embedded)parent.postMessage({type:'cinematch:ready'},'*');
    await loadLiveSources({initial:true});return;
  }
  if(params.get("direct")==="1"){
    if(embedded)parent.postMessage({type:"cinematch:ready"},"*");
    await startDirect(await waitForDirect());
    return;
  }
  if(params.get("directToken")){
    const token=params.get("directToken");
    const result=await send({type:"cb01-playback",token});
    await startDirect({...result,token});
    return;
  }
  const kind=params.get("kind")==="tv"?"tv":"movie";
  const id=Number(kind==="tv"?params.get("id"):params.get("movie")??params.get("id"));
  if(!Number.isSafeInteger(id)||id<1)throw new Error("Titolo non valido.");
  const preferredSeason=kind==="tv"&&params.get("season")!==null&&Number.isSafeInteger(Number(params.get("season")))&&Number(params.get("season"))>=0?Number(params.get("season")):null;
  const preferredEpisode=kind==="tv"&&params.get("episode")!==null&&Number.isSafeInteger(Number(params.get("episode")))&&Number(params.get("episode"))>0?Number(params.get("episode")):null;
  const result=await send({type:"watch-media",kind,id});
  state.kind=kind;state.movie=result.item;renderMedia();
  loadPoster().catch(()=>{});
  await loadAddons();renderChips();
  if(kind==="tv"){
    state.loading=true;$("sourcesStatus").textContent="Carico gli episodi…";renderSkeletons();
    parent.postMessage({type:"cinematch:ready"},"*");
    await setupEpisodes(preferredSeason,preferredEpisode);
  }else{
    await loadResume().catch(()=>{});
    updateResume();
    parent.postMessage({type:"cinematch:ready"},"*");
    await loadStreams();
  }
}
globalThis.CineMatchCinemaApp={showAddons,showPicker,state};
boot().catch(error=>{
  $("movieTitle").textContent="Titolo non disponibile";
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
