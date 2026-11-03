import {httpUrl} from './streams.js';
import {mediaFetch} from './media-access.js';
const EOF=-1048576;
export class RangeReader {
  constructor(url,{fetcher=mediaFetch,chunkSize=1024*1024,ext='mkv',onError=()=>{}}={}){
    this.url=httpUrl(url).href;this.fetcher=(...args)=>fetcher(...args);this.chunkSize=Math.min(1024*1024,Math.max(65536,chunkSize));this.ext=ext;this.onError=onError;this.position=0;this.length=0;this.bytesRead=0;this.block=null;this.controller=null;this.closed=false;this.lastError=null;
  }
  async blockAt(start,size=this.chunkSize){
    if(this.closed)throw new DOMException('Stopped','AbortError');
    this.controller?.abort();const controller=this.controller=new AbortController(),timer=setTimeout(()=>controller.abort(),20000);
    try{
      const response=await this.fetcher(this.url,{headers:{Range:`bytes=${start}-${start+size-1}`},signal:controller.signal,credentials:'omit',cache:'no-store',referrerPolicy:'no-referrer',redirect:'follow'});
      if(!response.ok)throw new Error(`Il server video ha risposto con errore ${response.status}. Aggiorna le sorgenti o riprova.`);
      const range=response.headers.get('content-range')?.match(/^bytes (\d+)-(\d+)\/(\d+)$/i);
      if(response.status===206&&(!range||Number(range[1])!==start))throw new Error('Il server ha restituito un intervallo video non valido.');
      const total=range?Number(range[3]):Number(response.headers.get('content-length'));
      if(!Number.isSafeInteger(total)||total<1)throw new Error('Il server non comunica la dimensione necessaria alla lettura del video a blocchi.');
      if(response.status!==206&&(start!==0||total>size))throw new Error('Il server non consente la lettura a blocchi. Scegli una sorgente con supporto HTTP Range.');
      this.length=total;
      const reader=response.body.getReader(),data=new Uint8Array(Math.min(size,total-start));let filled=0;
      try{while(filled<data.length){const {done,value}=await reader.read();if(done)break;const take=Math.min(value.length,data.length-filled);data.set(value.subarray(0,take),filled);filled+=take;this.bytesRead+=take;}}
      finally{controller.abort();await reader.cancel().catch(()=>{});}
      if(!filled)throw new Error('Il server ha restituito un blocco video vuoto.');
      this.block={start,data:data.subarray(0,filled)};return this.block;
    }catch(error){if(!this.closed){this.lastError=error.name==='AbortError'?new Error('Il server video non risponde. Riprova.'):error instanceof TypeError?new Error('Connessione al server video non riuscita. Controlla accesso e disponibilità.'):error;this.onError(this.lastError);}throw error;}
    finally{controller.abort();clearTimeout(timer);}
  }
  async open(){await this.blockAt(0,65536);this.position=0;return 0;}
  async read(target){
    if(this.closed||this.position>=this.length)return EOF;
    if(!this.block||this.position<this.block.start||this.position>=this.block.start+this.block.data.length)await this.blockAt(this.position);
    const offset=this.position-this.block.start,count=Math.min(target.length,this.block.data.length-offset);target.set(this.block.data.subarray(offset,offset+count));this.position+=count;return count;
  }
  async seek(pos){const position=Number(pos);if(!Number.isSafeInteger(position)||position<0||position>this.length)return -3;this.position=position;return 0;}
  async size(){return BigInt(this.length);}
  async stop(){this.closed=true;this.controller?.abort();this.block=null;}
}
export function loaderFor(Base,reader){return new class extends Base{
  get ext(){return reader.ext;}get name(){return 'Flusso CineMatch';}get flags(){return 1;}
  open(){return reader.open();}read(buffer){return reader.read(buffer);}seek(pos){return reader.seek(pos);}size(){return reader.size();}stop(){return reader.stop();}
}();}
