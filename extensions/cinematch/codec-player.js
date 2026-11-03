import {RangeReader,loaderFor} from './range-reader.js';
const codecFiles={1:'mpeg2video',2:'mpeg2video',12:'mpeg4',27:'h264',30:'theora',173:'hevc',225:'av1',139:'vp8',167:'vp9',17:'wmv',18:'wmv',71:'wmv',86017:'mp3',86018:'aac',86019:'ac3',86020:'dca',86021:'vorbis',86028:'flac',86051:'speex',86056:'eac3',86076:'opus',86023:'wma',86024:'wma',86052:'wma',86053:'wma',86054:'wma'};
export function decoderPath(type,id){
  if(type==='resampler')return 'resample/resample.wasm';if(type==='stretchpitcher')return 'stretchpitch/stretchpitch.wasm';
  const name=id>=65536&&id<=65572?'pcm':id>=69632&&id<=69683?'adpcm':codecFiles[id];if(!name)throw new Error('Questo codec non è incluso nei decoder integrati.');return `decode/${name}.wasm`;
}
let library;
export async function createCodecPlayer({container,url,kind='video',ext='mkv',onState=()=>{},onTime=()=>{},onTracks=()=>{},onFrame=()=>{},onError=()=>{},forceSoftware=false}){
  library??=import('./vendor/libmedia/avplayer.js');const {default:AVPlayer}=await library;AVPlayer.setLogLevel(6);
  const reader=['hls','dash'].includes(kind)?null:new RangeReader(url,{ext});
  const player=new AVPlayer({container,enableWorker:false,enableAudioWorklet:false,enableWebGPU:false,enableWebCodecs:!forceSoftware,enableHardware:!forceSoftware,checkUseMSE:()=>false,preLoadTime:3,enableJitterBuffer:false,getWasm:(type,id)=>{try{return chrome.runtime.getURL('vendor/libmedia/'+decoderPath(type,id));}catch{return null;}}});
  let destroyed=false,loaded=false,playing=false,timer=null,pausedPosition=null;
  const duration=()=>Number(player.getDuration())/1000,currentTime=()=>pausedPosition??Number(player.currentTime)/1000;
  const tracks=()=>{const streams=player.getStreams();return {selectedAudio:player.getSelectedAudioStreamId(),audio:streams.filter(s=>s.mediaType==='audio'||s.codecparProxy?.codecType===1).map(s=>({id:s.id,label:s.metadata?.title||s.metadata?.language||`Audio ${s.id+1}`})),subtitles:streams.filter(s=>s.mediaType==='subtitle'||s.codecparProxy?.codecType===3).map(s=>({id:s.id,label:s.metadata?.title||s.metadata?.language||`Sottotitoli ${s.id+1}`}))};};
  for(const [event,state]of [['playing','loading'],['played','playing'],['paused','paused'],['ended','ended']])player.on(event,()=>{if(destroyed)return;playing=state==='playing';if(playing)pausedPosition=null;onState(state);});
  player.on('firstAudioRendered',()=>{if(!destroyed)onFrame('audio');});player.on('firstVideoRendered',()=>{if(!destroyed)onFrame('video');});
  player.on('error',()=>{if(!destroyed)onError(reader?.lastError??new Error('Il motore integrato non riesce a riprodurre questa sorgente. Prova un altro flusso.'));});
  const adapter={reader,player,get playing(){return playing;},get currentTime(){return currentTime();},get duration(){return duration();},
    async start(){try{
      await player.load(reader?loaderFor(AVPlayer.IOLoader.CustomIOLoader,reader):url,{ext:kind==='dash'?'mpd':kind==='hls'?'m3u8':ext,http:{credentials:'omit',referrerPolicy:'no-referrer'},ioLoaderOptions:{preload:1024*1024,retryCount:1}});
      if(destroyed)return;loaded=true;onTracks(tracks());onTime(currentTime(),duration());timer=setInterval(()=>{if(!destroyed)onTime(currentTime(),duration());},250);await player.play();player.setSubtitleEnable(false);
    }catch(error){if(!destroyed)throw reader?.lastError??new Error(error.message?.includes('decoder integrati')?error.message:'Il motore integrato non riesce ad aprire la sorgente. Controlla accesso, formato e disponibilità.');}},
    async play(){if(loaded&&!destroyed){pausedPosition=null;await player.play();}},async pause(){if(loaded&&!destroyed){await player.pause();pausedPosition=Number(player.currentTime)/1000;}},
    async seek(seconds){if(loaded&&!destroyed){const position=Math.max(0,Math.min(duration(),seconds));await player.seek(BigInt(Math.round(position*1000)));if(!playing)pausedPosition=position;onTime(position,duration());}},
    volume(value){if(!destroyed)player.setVolume(Math.max(0,Math.min(1,value)));},async audio(id){if(!destroyed)await player.selectAudio(Number(id));},
    async subtitle(id){if(destroyed)return;if(id==='off')player.setSubtitleEnable(false);else{await player.selectSubtitle(Number(id));player.setSubtitleEnable(true);}},
    async destroy(){if(destroyed)return;destroyed=true;clearInterval(timer);await reader?.stop();await player.destroy().catch(()=>{});container.replaceChildren();}
  };return adapter;
}
