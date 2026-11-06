import {httpUrl} from './streams.js';
import {readUpdateBody} from './update-core.js';
export const DEBRID={rd:{name:'Real-Debrid',base:'https://api.real-debrid.com/rest/1.0/',origin:'https://api.real-debrid.com/*'},torbox:{name:'TorBox',base:'https://api.torbox.app/v1/api/',origin:'https://api.torbox.app/*'}};
export function torrentHash(stream){
  const raw=typeof stream?.infoHash==='string'?stream.infoHash:typeof stream?.magnet==='string'?new URL(stream.magnet).searchParams.get('xt')?.replace(/^urn:btih:/i,''):null;
  if(!/^[a-f0-9]{40}$/i.test(raw??''))throw new Error('Questo torrent non contiene un hash BitTorrent v1 valido.');
  return raw.toLowerCase();
}
export function chooseVideo(files,{fileIdx,season,episode}={}){
  const videos=files.filter(file=>/\.(mp4|mkv|avi|webm|m4v|mov|ts)$/i.test(file.path??file.name??'')&&!/\b(sample|trailer)\b/i.test(file.path??file.name??''));
  if(Number.isSafeInteger(fileIdx)&&fileIdx>=0){const selected=files[fileIdx];if(selected&&videos.includes(selected))return selected;throw new Error('Il file indicato dall’addon non è un video disponibile.');}
  if(Number.isSafeInteger(season)&&Number.isSafeInteger(episode)){
    const pattern=new RegExp(`(?:s0*${season}e0*${episode}(?!\\d)|(?:^|\\D)0*${season}x0*${episode}(?!\\d))`,'i');
    const matches=videos.filter(file=>pattern.test(file.path??file.name??''));
    if(matches.length===1)return matches[0];
    if(videos.length===1&&!/(?:s\d+e\d+|\d+x\d+)/i.test(videos[0].path??videos[0].name??''))return videos[0];
    throw new Error('Non posso identificare con certezza l’episodio nel torrent. Scegli un altro risultato con file indicizzato.');
  }
  if(!videos.length)throw new Error('Il torrent non contiene un file video supportato.');
  return videos.sort((a,b)=>Number(b.bytes??b.size??0)-Number(a.bytes??a.size??0))[0];
}
export function createDebridClient({provider,key,fetcher=fetch,getTask=async()=>null,setTask=async()=>{}}){
  const config=DEBRID[provider];
  if(!config||typeof key!=='string'||!key.trim()||key.length>4096||/[\s\x00-\x1f]/.test(key))throw new Error('Seleziona il servizio e inserisci una chiave valida.');
  async function api(path,{body,query}={}){
    const url=new URL(path,config.base);for(const [name,value]of Object.entries(query??{}))url.searchParams.set(name,String(value));
    let response;
    try{response=await fetcher(url.href,{method:body?'POST':'GET',headers:{Authorization:'Bearer '+key},body,credentials:'omit',referrerPolicy:'no-referrer',redirect:'error',cache:'no-store',signal:AbortSignal.timeout(20000)});}catch{throw new Error(`${config.name} non raggiungibile. Controlla rete e permessi del servizio.`);}
    if(!response.ok)throw Object.assign(new Error([401,403].includes(response.status)?`${config.name}: chiave non valida o account non abilitato.`:response.status===429?`${config.name}: limite di richieste raggiunto. Riprova più tardi.`:`${config.name}: richiesta non riuscita (${response.status}).`),{status:response.status});
    if(response.status===204)return null;
    let data;try{data=JSON.parse(new TextDecoder().decode(await readUpdateBody(response,{maxBytes:4*1024*1024})));}catch{throw new Error(`${config.name}: risposta non valida.`);}
    if(provider==='torbox'){if(data?.success!==true)throw new Error('TorBox: operazione non disponibile. Verifica account, disponibilità in cache e limiti del piano.');return data.data;}
    return data;
  }
  const direct=value=>{const url=httpUrl(value);if(url.protocol!=='https:'||url.searchParams.get('token')===key||url.href.includes(key))throw new Error('Il servizio non ha restituito un collegamento video utilizzabile.');return url.href;};
  return {
    async verify(){const data=await api(provider==='rd'?'user':'user/me');if(!data||typeof data!=='object')throw new Error('Account non riconosciuto.');return true;},
    async resolve(stream,context={}){
      const hash=torrentHash(stream),magnet='magnet:?xt=urn:btih:'+hash;
      if(provider==='rd'){
        let id=await getTask(hash);
        if(!id){const added=await api('torrents/addMagnet',{body:new URLSearchParams({magnet})});
          if(!/^[a-z0-9_-]+$/i.test(String(added?.id??'')))throw new Error('Real-Debrid: torrent non disponibile.');
          id=String(added.id);await setTask(hash,id);}
        let info;try{info=await api('torrents/info/'+id);}catch(error){if(error.status===404){await setTask(hash,null);throw new Error('Torrent rimosso da Real-Debrid. Premi Riprova torrent per aggiungerlo di nuovo.');}throw error;}
        if(!Array.isArray(info?.files)||!info.files.length)throw new Error('Real-Debrid sta acquisendo i dati del torrent. Riprova tra poco.');
        const file=chooseVideo(info.files,{...context,fileIdx:stream.fileIdx});
        if(info.status==='waiting_files_selection')await api('torrents/selectFiles/'+id,{body:new URLSearchParams({files:String(file.id)})});
        info=await api('torrents/info/'+id);
        if(info.status!=='downloaded')throw new Error('Torrent aggiunto a Real-Debrid, ma non ancora pronto. Riprova quando il download del servizio è completato.');
        const selected=info.files.filter(file=>file.selected===1),index=selected.findIndex(item=>item.id===file.id);
        if(index<0||!info.links?.[index])throw new Error('Il file richiesto non è selezionato su Real-Debrid. Scegli un altro risultato.');
        const result=await api('unrestrict/link',{body:new URLSearchParams({link:info.links[index]})});
        return {url:direct(result.download),filename:file.path};
      }
      const body=new FormData();body.set('magnet',magnet);body.set('add_only_if_cached','true');body.set('allow_zip','false');
      const added=await api('torrents/createtorrent',{body});const id=added?.torrent_id;
      if(!Number.isSafeInteger(id)||id<0)throw new Error('TorBox: torrent non disponibile in cache.');
      const data=await api('torrents/mylist',{query:{id,bypass_cache:true}}),info=Array.isArray(data)?data.find(item=>item.id===id):data;
      if(!info?.download_finished||!Array.isArray(info.files))throw new Error('TorBox: il torrent non è ancora pronto. Riprova più tardi.');
      const file=chooseVideo(info.files,{...context,fileIdx:stream.fileIdx});
      const result=await api('torrents/requestdl',{query:{token:key,torrent_id:id,file_id:file.id,redirect:false}});
      return {url:direct(result),filename:file.name};
    }
  };
}
export function enableDebridStreams(streams,enabled){return streams.map(stream=>stream.kind==='torrent'&&stream.infoHash?{...stream,playable:Boolean(enabled),reason:enabled?'Verrà risolto dal servizio collegato.':'Collega Real-Debrid o TorBox nelle impostazioni → Flussi.'}:stream);}
