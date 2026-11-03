import test from 'node:test';import assert from 'node:assert/strict';
import {RangeReader} from '../extensions/cinematch/range-reader.js';
import {decoderPath} from '../extensions/cinematch/codec-player.js';
test('range reader fetches bounded blocks without HEAD, cache, referrer or credentials',async()=>{
  const requests=[],total=8*1024**3;const reader=new RangeReader('https://example.org/video',{prefetch:false,chunkSize:1024**2,fetcher:async(_,init)=>{requests.push(init);const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/);const start=Number(a),end=Math.min(Number(b),total-1);return new Response(new Uint8Array(end-start+1).fill(42),{status:206,headers:{'content-range':`bytes ${start}-${end}/${total}`}});}});
  await reader.open();assert.equal(await reader.size(),BigInt(total));const out=new Uint8Array(4096);assert.equal(await reader.read(out),4096);assert.equal(out[0],42);assert.equal(requests.length,1);
  await reader.seek(BigInt(6*1024**3));await reader.read(out);assert.ok(reader.block.data.length<=1024**2);assert.ok(reader.bytesRead<2*1024**2);
  assert.ok(requests.every(r=>r.credentials==='omit'&&r.cache==='no-store'&&r.referrerPolicy==='no-referrer'&&r.method!=='HEAD'));await reader.stop();assert.equal(reader.block,null);assert.equal(await reader.read(out),-1048576);
});
test('non-range large responses are cancelled instead of downloading an entire movie',async()=>{
  const reader=new RangeReader('https://example.org',{fetcher:async()=>new Response('small',{headers:{'content-length':String(7*1024**3)}})});await assert.rejects(reader.open(),/non consente la lettura a blocchi/);
});
test('invalid offsets and wrong Content-Range are rejected',async()=>{
  const reader=new RangeReader('https://example.org',{fetcher:async()=>new Response('data',{status:206,headers:{'content-range':'bytes 100-103/1000'}})});await assert.rejects(reader.open(),/intervallo video non valido/);assert.equal(await reader.seek(-1n),-3);
});
test('small complete responses work and EOF is explicit',async()=>{
  const reader=new RangeReader('https://example.org',{fetcher:async()=>new Response('test',{headers:{'content-length':'4'}})});await reader.open();const output=new Uint8Array(32);assert.equal(await reader.read(output),4);assert.equal(await reader.read(output),-1048576);await reader.stop();
});
test('decoder resources use local allowlisted codec IDs only',()=>{
  assert.equal(decoderPath('decoder',173),'decode/hevc.wasm');assert.equal(decoderPath('decoder',86019),'decode/ac3.wasm');assert.equal(decoderPath('decoder',86056),'decode/eac3.wasm');assert.equal(decoderPath('decoder',86020),'decode/dca.wasm');assert.throws(()=>decoderPath('decoder',999999));
});

const MiB=1024**2;
function bytesResponse(start,end,total,url){
  const response=new Response(new Uint8Array(end-start+1).fill(Math.floor(start/65536)%251),{status:206,headers:{'content-range':`bytes ${start}-${end}/${total}`}});
  if(url)Object.defineProperty(response,'url',{value:url});return response;
}
test('sequential reads overlap the next request, preserve bytes and keep only two bounded blocks',async()=>{
  const calls=[],total=50*1024**3;
  const reader=new RangeReader('https://addon.test/play',{chunkSize:8*MiB,fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Math.min(Number(b),total-1);calls.push({start,end});return bytesResponse(start,end,total);
  }});
  await reader.open();const out=new Uint8Array(65536);assert.equal(await reader.read(out),65536);assert.equal(calls.length,2,'next request starts before the current cache misses');
  await reader.next.promise;assert.ok(reader.bufferedBytes<=16*MiB);assert.ok(reader.next.block.data.length===8*MiB);
  await reader.read(out);assert.equal(out[0],1);assert.equal(reader.position,131072);assert.equal(calls.length,3,'another block is read ahead while the active one is consumed');
  await reader.stop();assert.equal(reader.bufferedBytes,0);assert.equal(reader.next,null);
});

test('seek cancels speculative I/O without reporting a playback error or using the wrong block',async()=>{
  let cancelled=0;const errors=[],total=64*MiB;
  const reader=new RangeReader('https://video.test/movie',{onError:e=>errors.push(e),fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Math.min(Number(b),total-1);
    if(start===65536)return new Promise((_,reject)=>init.signal.addEventListener('abort',()=>{cancelled++;reject(new DOMException('seek','AbortError'));},{once:true}));
    return bytesResponse(start,end,total);
  }});
  await reader.open();await reader.read(new Uint8Array(32));await reader.seek(32*MiB);assert.equal(cancelled,1);const out=new Uint8Array(32);await reader.read(out);assert.equal(out[0],Math.floor(32*MiB/65536)%251);assert.deepEqual(errors,[]);await reader.stop();
});

test('signed CDN URLs bypass repeated addon redirects, refresh on expiry and never persist after stop',async()=>{
  const seen=[],total=64*MiB;let expired=false;
  const reader=new RangeReader('https://addon.test/play',{prefetch:false,fetcher:async(url,init)=>{
    seen.push(url);if(url==='https://cdn.test/old'&&expired)return new Response('expired',{status:403});
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Math.min(Number(b),total-1);return bytesResponse(start,end,total,expired?'https://cdn.test/new':'https://cdn.test/old');
  }});
  await reader.open();await reader.seek(MiB);await reader.read(new Uint8Array(1));expired=true;await reader.seek(32*MiB);await reader.read(new Uint8Array(1));
  assert.deepEqual(seen,['https://addon.test/play','https://cdn.test/old','https://cdn.test/old','https://addon.test/play']);assert.equal(reader.resolvedUrl,'https://cdn.test/new');await reader.stop();assert.equal(reader.resolvedUrl,null);
});

test('speculative failures are reported only when consumed and stop cancels pending requests',async()=>{
  const errors=[],total=64*MiB;
  const reader=new RangeReader('https://video.test/movie',{onError:e=>errors.push(e),fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Math.min(Number(b),total-1);
    if(start)return new Response('unavailable',{status:503});return bytesResponse(start,end,total);
  }});
  await reader.open();await reader.read(new Uint8Array(65536));await reader.next.promise;assert.equal(errors.length,0);await assert.rejects(reader.read(new Uint8Array(1)),/503/);assert.equal(errors.length,1);await reader.stop();
});

test('servers that cap a range below the requested window still allow seeking into that window',async()=>{
  const total=64*MiB;
  const reader=new RangeReader('https://video.test/movie',{fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Math.min(Number(b),start+MiB-1,total-1);return bytesResponse(start,end,total);
  }});
  await reader.open();await reader.read(new Uint8Array(1));await reader.next.promise;await reader.seek(5*MiB);const out=new Uint8Array(1);assert.equal(await reader.read(out),1);assert.equal(out[0],Math.floor(5*MiB/65536)%251);await reader.stop();
});

test('a truncated server body fails instead of silently skipping file bytes',async()=>{
  const reader=new RangeReader('https://video.test/movie',{fetcher:async()=>new Response(new Uint8Array(8),{status:206,headers:{'content-range':'bytes 0-63/1000'}})});
  await assert.rejects(reader.open(),/interrotta/);await reader.stop();
});
