/* Playback engine shared by the in-chat cinema app. It keeps the existing
   native/HLS/libmedia strategy but exposes a small, UI-agnostic API. */
import {permissionOrigin} from "./streams.js";
import {probePlayback,declaredCodecs,compatibility,containerExtension,likelyIntegrated,nativePlaybackError} from "./playback.js";
import {createCodecPlayer} from "./codec-player.js";
import {playbackPermissions,permissionLabel} from "./media-access.js";

export function createPlayer({video,codecContainer,onStatus=()=>{},onTime=()=>{},onState=()=>{},onTracks=()=>{},onFrame=()=>{},onError=()=>{},probeMedia=probePlayback,codecFactory=createCodecPlayer}){
  let hls=null,integrated=null,mode="native",generation=0,lastStream=null,probe=null,abort=null,engineCleanup=Promise.resolve(),volumeValue=1;
  const codecCache=new Map();
  const checkCodecs=codecs=>compatibility(codecs,mime=>{if(!codecCache.has(mime))codecCache.set(mime,video.canPlayType(mime));return codecCache.get(mime);});

  async function reset(){
    mode="idle";
    if(hls){try{hls.destroy();}catch{}hls=null;}
    const previous=integrated;integrated=null;
    if(previous)await previous.destroy().catch(()=>{});
    try{video.pause();video.removeAttribute("src");video.load();}catch{}
    video.hidden=false;
    codecContainer.hidden=true;
    codecContainer.replaceChildren();
    onTracks({audio:[],subtitles:[]});
  }
  function queueReset(){engineCleanup=engineCleanup.then(reset,reset);return engineCleanup;}

  function nativeTracks(){
    const subtitles=[];
    try{for(let i=0;i<video.textTracks.length;i++){const track=video.textTracks[i];subtitles.push({id:String(i),label:track.label||`Sottotitoli ${i+1}`});}}catch{}
    onTracks({audio:[],subtitles});
  }

  async function startNative(stream,isHls,current){
    if(current!==generation)return;
    mode="native";
    video.hidden=false;codecContainer.hidden=true;
    // CB01 serves a hotlink-protected MP4 with an ACAO header for its own
    // player. The direct stream does not need CORS here, so leave the element
    // in no-cors mode; normal addon streams keep the existing behavior.
    if(stream.crossOrigin===false)video.removeAttribute("crossorigin");
    else video.setAttribute("crossorigin","anonymous");
    video.querySelectorAll("track").forEach(track=>track.remove());
    for(const sub of stream.subtitles){if(/\.vtt(?:\?|$)/i.test(sub.url)){const track=document.createElement("track");track.kind="subtitles";track.label=sub.lang;track.srclang=/^[a-z]{2,3}(?:-[a-z]{2})?$/i.test(sub.lang)?sub.lang:"und";track.src=sub.url;video.append(track);}}
    nativeTracks();
    if(isHls&&!video.canPlayType("application/vnd.apple.mpegurl")){
      if(!globalThis.Hls?.isSupported())throw new Error("HLS non è supportato da questo browser. Scegli un’altra sorgente.");
      hls=new Hls({enableWorker:true,maxBufferLength:30,backBufferLength:15,xhrSetup:xhr=>{xhr.withCredentials=false;}});
      hls.on(Hls.Events.ERROR,async(_,data)=>{
        if(current!==generation||mode!=="native"||!data.fatal)return;
        const failed=data.context?.url??data.response?.url??data.url;
        if(failed){try{if(!await chrome.permissions.contains({origins:[permissionOrigin(failed)]})){if(current===generation&&mode==="native")onError(Object.assign(new Error(`Il flusso usa un altro server (${new URL(failed).hostname}). Autorizzalo per continuare.`),{mediaOrigin:permissionOrigin(failed)}));return;}}catch{}}
        if(current!==generation||mode!=="native")return;
        hls?.stopLoad();
        startIntegrated(stream,{kind:"hls"},current).catch(error=>{if(current===generation)onError(error);});
      });
      hls.attachMedia(video);hls.loadSource(stream.url);
    }else video.src=stream.url;
    try{await video.play();}catch(error){
      if(error.name==="NotAllowedError"){onState("needs-gesture");onStatus("Sorgente pronta. Premi Riproduci.");}
      else throw error;
    }
  }

  async function startIntegrated(stream,probeResult,current){
    if(current!==generation)return;
    mode="integrated";
    onState("loading");onStatus("Avvio dei decoder integrati…");
    try{video.pause();video.removeAttribute("src");video.load();}catch{}
    video.hidden=true;codecContainer.hidden=false;codecContainer.replaceChildren();
    // An obsolete async decoder must never clear the new decoder's DOM.
    const surface=codecContainer.ownerDocument.createElement('div');
    surface.style.cssText='position:absolute;inset:0;width:100%;height:100%';
    codecContainer.append(surface);
    const ext=containerExtension(stream,probeResult)||(probeResult.kind==="video"?"mp4":"mkv");
    const candidate=await codecFactory({
      container:surface,url:stream.url,
      kind:probeResult.kind==="dash"||stream.kind==="dash"?"dash":probeResult.kind==="hls"||stream.kind==="hls"?"hls":"video",
      ext,
      onState:state=>{if(current===generation&&mode==="integrated")onState(state);},
      onTime:(position,duration)=>{if(current===generation)onTime(position,duration);},
      onTracks:tracks=>{if(current===generation)onTracks(tracks);},
      onFrame:type=>{if(current===generation)onFrame(type);},
      onAudioUnavailable:()=>{if(current===generation)onStatus("Audio non supportato: prova un’altra traccia o sorgente.");},
      onError:error=>{if(current===generation)onError(error);}
    });
    if(current!==generation){await candidate.destroy();surface.remove();return;}
    integrated=candidate;
    candidate.volume(volumeValue);
    await candidate.start();
    if(current!==generation){await candidate.destroy();surface.remove();if(integrated===candidate)integrated=null;return;}
    onState("playing");onStatus("In riproduzione · decoder integrati");
  }

  async function load(stream){
    if(!stream?.url)throw new Error("Sorgente non valida.");
    const current=++generation;
    const previousMode=mode;mode="idle";
    if(previousMode==="integrated")integrated?.pause().catch(()=>{});
    else try{video.pause();}catch{}
    lastStream=stream;
    abort?.abort();abort=new AbortController();
    const origins=playbackPermissions(stream);
    let granted=true;
    try{granted=await chrome.permissions.request({origins});}catch{granted=false;}
    if(!granted){
      const missing=[];
      for(const origin of origins)if(!await chrome.permissions.contains({origins:[origin]}))missing.push(origin);
      if(missing.length){const error=new Error(`Accesso a ${permissionLabel(missing[0])} non concesso. Premi Consenti per autorizzare il server video.`);error.mediaOrigin=missing[0];throw error;}
    }
    if(current!==generation)return;
    await queueReset();
    if(current!==generation)return;
    onState("loading");onStatus("Verifica della sorgente e del formato…");
    const probed=await probeMedia(stream.url,{signal:abort.signal,timeout:20000});
    if(current!==generation)return;
    probe=probed;
    if(probed.kind==="page")throw new Error("La sorgente restituisce una pagina o un messaggio del servizio, non un video. Aggiorna le sorgenti o scegli un altro flusso.");
    const actual=checkCodecs(probed.codecs),declared=checkCodecs(declaredCodecs(stream));
    const isHls=probed.kind==="hls"||(probed.kind==="unknown"&&stream.kind==="hls");
    if(probed.kind==="matroska"||probed.kind==="dash"||stream.kind==="dash"||actual.unsupported.length||(!isHls&&declared.unsupported.length)||(!isHls&&likelyIntegrated(stream,probed)))return startIntegrated(stream,probed,current);
    return startNative(stream,isHls,current);
  }

  async function retry(){if(lastStream)return load(lastStream);}

  video.addEventListener("playing",()=>{if(mode==="native")onState("playing");});
  video.addEventListener("pause",()=>{if(mode==="native"&&!video.ended)onState("paused");});
  video.addEventListener("timeupdate",()=>{if(mode==="native")onTime(video.currentTime,video.duration||0);});
  video.addEventListener("waiting",()=>{if(mode==="native")onState("loading");});
  video.addEventListener("ended",()=>{
    if(mode!=="native"||!lastStream)return;
    const empty=video.currentTime<.05&&(!Number.isFinite(video.duration)||video.duration<.05);
    if(empty){const current=generation;onState("loading");onStatus("Il formato richiede i decoder integrati. Avvio…");startIntegrated(lastStream,probe??{},current).catch(error=>{if(current===generation)onError(error);});}
    else onState("ended");
  });
  video.addEventListener("error",()=>{
    if(mode!=="native"||!lastStream)return;
    const code=video.error?.code;
    const current=generation;
    onState("loading");onStatus("Il browser non riesce a decodificare questa sorgente. Provo i decoder integrati…");
    startIntegrated(lastStream,probe??{},current).catch(()=>{if(current===generation)onError(new Error(nativePlaybackError(code)));});
  });

  return {
    load,retry,
    async play(){if(mode==="integrated")return integrated?.play();return video.play();},
    async pause(){if(mode==="integrated"){await integrated?.pause();return;}video.pause();},
    async seek(seconds){if(mode==="integrated")return integrated?.seek(seconds);if(Number.isFinite(video.duration))video.currentTime=Math.max(0,Math.min(video.duration,seconds));},
    setVolume(value){volumeValue=Math.max(0,Math.min(1,Number(value)||0));if(mode==="integrated")integrated?.volume(volumeValue);else video.volume=volumeValue;},
    async setAudio(id){if(mode==="integrated")await integrated?.audio(id);},
    async setSubtitle(id){
      if(mode==="integrated")return integrated?.subtitle(id);
      try{for(let i=0;i<video.textTracks.length;i++)video.textTracks[i].mode=String(i)===String(id)?"showing":"disabled";}catch{}
    },
    get mode(){return mode;},
    get currentTime(){return mode==="integrated"?(integrated?.currentTime??0):(video.currentTime||0);},
    get duration(){return mode==="integrated"?(integrated?.duration??0):(Number.isFinite(video.duration)?video.duration:0);},
    get paused(){return mode==="integrated"?!(integrated?.playing):video.paused;},
    get stream(){return lastStream;},
    async destroy(){generation++;abort?.abort();await queueReset();}
  };
}
