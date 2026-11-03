import {httpUrl} from './streams.js';
import {mediaFetch} from './media-access.js';
const EOF=-1048576,MiB=1024*1024;
const aborted=()=>new DOMException('Stopped','AbortError');
export class RangeReader {
  constructor(url,{fetcher=mediaFetch,chunkSize=8*MiB,prefetch=true,ext='mkv',onError=()=>{}}={}){
    this.url=httpUrl(url).href;this.resolvedUrl=null;this.fetcher=(...args)=>fetcher(...args);
    this.chunkSize=Math.min(8*MiB,Math.max(65536,chunkSize));this.prefetch=prefetch;this.ext=ext;this.onError=onError;
    this.position=0;this.length=0;this.bytesRead=0;this.requestCount=0;this.block=null;this.next=null;this.controller=null;this.controllers=new Set();this.closed=false;this.lastError=null;this.epoch=0;this.samples=[];
  }
  get bufferedBytes(){return (this.block?.data.length??0)+(this.next?.block?.data.length??0);}
  get downloadMbps(){const bytes=this.samples.reduce((n,s)=>n+s.bytes,0),ms=this.samples.reduce((n,s)=>n+s.ms,0);return ms?bytes*8/(ms*1000):0;}
  record(error,epoch){
    if(!this.closed&&epoch===this.epoch){this.lastError=error.name==='AbortError'?new Error('Il server video non risponde. Riprova.'):error instanceof TypeError?new Error('Connessione al server video non riuscita. Controlla accesso e disponibilità.'):error;this.onError(this.lastError);}
    return error;
  }
  async fetchBlock(start,size,controller){
    if(this.closed)throw aborted();
    this.controllers.add(controller);const timer=setTimeout(()=>controller.abort(),20000),began=performance.now();
    try{
      const init={headers:{Range:`bytes=${start}-${start+size-1}`},signal:controller.signal,credentials:'omit',cache:'no-store',referrerPolicy:'no-referrer',redirect:'follow'};
      // Keep the signed CDN URL only for this session. On expiry resolve the
      // original addon link again, once; do not retry rate limits or permissions.
      const target=this.resolvedUrl??this.url;this.requestCount++;
      let response=await this.fetcher(target,init);
      if(target!==this.url&&[401,403,404,410].includes(response.status)){
        await response.body?.cancel().catch(()=>{});this.resolvedUrl=null;this.requestCount++;response=await this.fetcher(this.url,init);
      }
      if(!response.ok)throw new Error(`Il server video ha risposto con errore ${response.status}. Aggiorna le sorgenti o riprova.`);
      const range=response.headers.get('content-range')?.match(/^\s*bytes\s+(\d+)\s*-\s*(\d+)\s*\/\s*(\d+)\s*$/i);
      const total=range?Number(range[3]):Number(response.headers.get('content-length'));
      if(response.status===206&&(!range||Number(range[1])!==start||!Number.isSafeInteger(Number(range[2]))||Number(range[2])<start||Number(range[2])>=total))throw new Error('Il server ha restituito un intervallo video non valido.');
      if(!Number.isSafeInteger(total)||total<1)throw new Error('Il server non comunica la dimensione necessaria alla lettura del video a blocchi.');
      if(response.status!==206&&(start!==0||total>size))throw new Error('Il server non consente la lettura a blocchi. Scegli una sorgente con supporto HTTP Range.');
      if(this.length&&this.length!==total)throw new Error('La dimensione del video è cambiata. Aggiorna le sorgenti.');
      // Some video CDNs honor the starting byte but send a larger/open-ended
      // range. Keep our own window bounded and cancel the surplus body.
      const expected=range?Math.min(size,Number(range[2])-start+1):total;
      const reader=response.body.getReader(),data=new Uint8Array(expected);let filled=0;
      try{while(filled<data.length){const {done,value}=await reader.read();if(done)break;const take=Math.min(value.length,data.length-filled);data.set(value.subarray(0,take),filled);filled+=take;this.bytesRead+=take;}}
      finally{await reader.cancel().catch(()=>{});}
      if(controller.signal.aborted||this.closed)throw aborted();
      if(filled!==expected)throw new Error('La lettura del blocco video si è interrotta. Riprova.');
      this.length=total;if(response.url)this.resolvedUrl=httpUrl(response.url).href;
      this.samples.push({bytes:filled,ms:Math.max(1,performance.now()-began)});if(this.samples.length>4)this.samples.shift();
      return {start,data};
    }finally{controller.abort();clearTimeout(timer);this.controllers.delete(controller);}
  }
  cancelNext(){this.next?.controller.abort();this.next=null;}
  schedule(){
    if(!this.prefetch||this.closed||this.next||!this.block)return;
    const start=this.block.start+this.block.data.length;if(start>=this.length)return;
    const job={start,size:Math.min(this.chunkSize,this.length-start),controller:new AbortController(),block:null,error:null};this.next=job;
    // Speculative errors surface only if this block is consumed. A seek or
    // switch can abandon it without a stale error or unhandled rejection.
    job.promise=this.fetchBlock(start,job.size,job.controller).then(block=>{if(this.next===job&&!this.closed)job.block=block;return block;},error=>{job.error=error;return null;});
  }
  async blockAt(start,size=this.chunkSize){
    if(this.closed)throw aborted();const epoch=this.epoch;
    try{
      const pending=this.next;let block;
      if(pending&&start>=pending.start&&start<pending.start+pending.size){
        block=await pending.promise;if(this.next===pending)this.next=null;if(pending.error)throw pending.error;
        if(block&&start>=block.start+block.data.length)block=null;
      }
      if(!block){
        this.cancelNext();this.controller?.abort();const controller=this.controller=new AbortController();
        try{block=await this.fetchBlock(start,size,controller);}finally{if(this.controller===controller)this.controller=null;}
      }
      if(this.closed||epoch!==this.epoch)throw aborted();this.block=block;return block;
    }catch(error){throw this.record(error,epoch);}
  }
  async open(){await this.blockAt(0,65536);this.position=0;return 0;}
  async read(target){
    if(this.closed||this.position>=this.length)return EOF;
    if(!this.block||this.position<this.block.start||this.position>=this.block.start+this.block.data.length)await this.blockAt(this.position);
    const offset=this.position-this.block.start,count=Math.min(target.length,this.block.data.length-offset);target.set(this.block.data.subarray(offset,offset+count));this.position+=count;this.schedule();return count;
  }
  async seek(pos){
    const position=Number(pos);if(!Number.isSafeInteger(position)||position<0||position>this.length)return -3;
    this.epoch++;this.controller?.abort();
    const cached=this.block&&position>=this.block.start&&position<this.block.start+this.block.data.length;
    const upcoming=this.next&&position>=this.next.start&&position<this.next.start+this.next.size;
    if(!cached&&!upcoming){this.cancelNext();this.block=null;}
    this.position=position;return 0;
  }
  async size(){return BigInt(this.length);}
  async stop(){this.closed=true;this.epoch++;this.cancelNext();for(const controller of this.controllers)controller.abort();this.block=null;this.resolvedUrl=null;}
}
export function loaderFor(Base,reader){return new class extends Base{
  get ext(){return reader.ext;}get name(){return 'Flusso CineMatch';}get flags(){return 1;}
  open(){return reader.open();}read(buffer){return reader.read(buffer);}seek(pos){return reader.seek(pos);}size(){return reader.size();}stop(){return reader.stop();}
}();}
