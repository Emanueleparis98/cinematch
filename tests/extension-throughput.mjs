import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {RangeReader} from '../extensions/cinematch/range-reader.js';

// Reproduce an addon redirect and CDN with 60 ms latency each. Serve only the
// requested bytes of a virtual 50 GiB file, in memory; no account or movie.
const MiB=1024**2,total=50*1024**3,consume=32*MiB;
const slab=Buffer.alloc(65536,0x5a);let redirects=0,base;
const server=createServer((req,res)=>{
  const path=new URL(req.url,base).pathname;
  setTimeout(()=>{
    if(res.destroyed)return;
    if(path==='/addon'){redirects++;res.writeHead(302,{Location:`${base}/cdn`});return res.end();}
    const match=req.headers.range?.match(/^bytes=(\d+)-(\d+)$/);if(!match){res.writeHead(400);return res.end();}
    const start=Number(match[1]),end=Math.min(Number(match[2]),total-1);let remaining=end-start+1;
    res.writeHead(206,{'Content-Range':`bytes ${start}-${end}/${total}`,'Content-Length':remaining});
    const send=()=>{while(remaining&&!res.destroyed){const chunk=slab.subarray(0,Math.min(remaining,slab.length));remaining-=chunk.length;if(!res.write(chunk)){res.once('drain',send);return;}}if(!res.destroyed)res.end();};send();
  },60);
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));base=`http://127.0.0.1:${server.address().port}`;
async function measure(legacy){
  redirects=0;const url=`${base}/addon`,reader=new RangeReader(url,{...(legacy?{chunkSize:MiB,prefetch:false}:{}),fetcher:(target,init)=>fetch(legacy?url:target,init)});
  const began=performance.now(),out=new Uint8Array(MiB);let count=0,maxBuffered=0;
  try{
    await reader.open();
    while(count<consume){const n=await reader.read(out.subarray(0,Math.min(out.length,consume-count)));assert.ok(n>0);assert.equal(out[0],0x5a);assert.equal(out[n-1],0x5a);count+=n;maxBuffered=Math.max(maxBuffered,reader.bufferedBytes);}
    const seconds=(performance.now()-began)/1000;
    return {mode:legacy?'previous serial model':'buffered reader',seconds:Number(seconds.toFixed(3)),Mbps:Number((count*8/seconds/1e6).toFixed(1)),requests:reader.requestCount,addonRedirects:redirects,maxBufferedMiB:Number((maxBuffered/MiB).toFixed(2))};
  }finally{await reader.stop();}
}
try{
  const before=await measure(true),after=await measure(false);console.log(JSON.stringify({benchmark:'controlled 60 ms addon + 60 ms CDN latency, 32 MiB consumed',before,after},null,2));
  assert.ok(before.addonRedirects>=32);assert.equal(after.addonRedirects,1);assert.ok(after.requests<=6);assert.ok(after.maxBufferedMiB<=16);
  console.log('PASS sequential throughput: bounded read-ahead, fewer requests and no repeated addon redirects. Timing is a local benchmark, not a remote speed guarantee.');
}finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
