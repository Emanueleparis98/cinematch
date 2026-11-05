/* Update I/O shared by the popup and its persistent window. No remote code. */
export const MAX_UPDATE_BYTES=50*1024*1024;
export function validUpdate(data){
  const version=typeof data?.version==='string'?data.version:'';
  const sha256=typeof data?.sha256==='string'?data.sha256.toLowerCase():'';
  const size=Number(data?.size);
  if(!/^\d+\.\d+\.\d+$/.test(version)||version.split('.').some(part=>!Number.isSafeInteger(Number(part))))throw new Error('Il canale di aggiornamento contiene una versione non valida.');
  if(!/^[a-f0-9]{64}$/.test(sha256))throw new Error('Il canale di aggiornamento non contiene un hash SHA-256 valido.');
  if(!Number.isSafeInteger(size)||size<1||size>MAX_UPDATE_BYTES)throw new Error('Il canale di aggiornamento contiene una dimensione non valida.');
  return {version,sha256,size};
}
export function versionIsNewer(candidate,current){
  const a=candidate.split('.').map(Number),b=current.split('.').map(Number);
  for(let i=0;i<3;i++)if(a[i]!==b[i])return a[i]>b[i];
  return false;
}
export async function readUpdateBody(response,{maxBytes,expectedSize,onProgress=()=>{}}){
  const declared=Number(response.headers.get('content-length'));
  if(declared>maxBytes){await response.body?.cancel();throw new Error('Il download supera la dimensione massima consentita.');}
  if(!response.body)throw new Error('GitHub ha restituito un download vuoto.');
  const reader=response.body.getReader(),chunks=[];let size=0,last=-1;
  try{
    for(;;){
      const {done,value}=await reader.read();if(done)break;
      size+=value.byteLength;
      if(size>maxBytes)throw new Error('Il download supera la dimensione prevista.');
      chunks.push(value);
      const progress=Math.min(100,Math.round(size/(expectedSize||maxBytes)*100));
      if(progress!==last){last=progress;onProgress(progress);}
    }
  }finally{await reader.cancel().catch(()=>{});}
  if(expectedSize!==undefined&&size!==expectedSize)throw new Error('La dimensione del pacchetto non corrisponde al manifesto.');
  const result=new Uint8Array(size);let offset=0;
  for(const chunk of chunks){result.set(chunk,offset);offset+=chunk.length;}
  return result;
}
async function parentFor(root,path,create=false){
  const parts=path.split('/'),name=parts.pop();let directory=root;
  for(const part of parts)directory=await directory.getDirectoryHandle(part,{create});
  return {directory,name};
}
export async function writeUpdateFile(root,path,data){
  const {directory,name}=await parentFor(root,path,true);
  const file=await directory.getFileHandle(name,{create:true}),writable=await file.createWritable();
  try{await writable.write(data);await writable.close();}
  catch(error){await writable.abort?.().catch(()=>{});throw error;}
}
export async function verifyLoadedFolder(root,extensionUrl,{fetcher=fetch}={}){
  const token=crypto.randomUUID(),name=`cinematch-update-${token}.json`;
  await writeUpdateFile(root,name,JSON.stringify({token}));
  try{
    const response=await fetcher(extensionUrl(name),{cache:'no-store',credentials:'omit',signal:AbortSignal.timeout(5000)});
    if(!response.ok||JSON.parse(new TextDecoder().decode(await readUpdateBody(response,{maxBytes:1024}))).token!==token)throw new Error('Folder mismatch');
  }catch{
    throw new Error('Questa cartella non è quella caricata da Chrome. Usa Cambia cartella e seleziona quella indicata nei dettagli dell’estensione.');
  }finally{await root.removeEntry(name).catch(()=>{});}
}
export async function installUpdate(root,files,{onProgress=()=>{},maxBackupBytes=256*1024*1024}={}){
  const ordered=[...files.filter(file=>file.path!=='manifest.json'),...files.filter(file=>file.path==='manifest.json')];
  const changes=[];let backedUp=0;
  // Read and compare before changing anything. Unchanged decoders/fonts need
  // no write; a failed read is never mistaken for a missing file.
  for(const file of ordered){
    let previous=null;
    try{
      const {directory,name}=await parentFor(root,file.path);
      const existing=await (await directory.getFileHandle(name)).getFile();
      backedUp+=existing.size;
      if(backedUp>maxBackupBytes)throw new Error('Spazio di ripristino insufficiente: aggiornamento annullato senza modificare i file.');
      previous=new Uint8Array(await existing.arrayBuffer());
    }catch(error){if(error.name!=='NotFoundError')throw error;}
    if(previous&&previous.length===file.data.length&&previous.every((byte,i)=>byte===file.data[i]))continue;
    changes.push({...file,previous});
  }
  const attempted=[];
  try{
    for(const file of changes){attempted.push(file);await writeUpdateFile(root,file.path,file.data);onProgress(attempted.length,changes.length);}
  }catch(error){
    let restored=true;
    for(const file of attempted.reverse()){
      try{
        if(file.previous!==null)await writeUpdateFile(root,file.path,file.previous);
        else{const {directory,name}=await parentFor(root,file.path);await directory.removeEntry(name);}
      }catch{restored=false;}
    }
    throw new Error(restored?'Aggiornamento interrotto: i file precedenti sono stati ripristinati. Riprova.':'Aggiornamento interrotto e ripristino incompleto. Controlla l’accesso alla cartella prima di ricaricare l’estensione.');
  }
  return {written:changes.length,unchanged:files.length-changes.length};
}
