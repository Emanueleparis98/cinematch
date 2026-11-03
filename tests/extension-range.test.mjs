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
  assert.equal(decoderPath('decoder',173),'decode/hevc.wasm');assert.equal(decoderPath('decoder',196),'decode/vvc.wasm');assert.equal(decoderPath('decoder',86019),'decode/ac3.wasm');assert.equal(decoderPath('decoder',86056),'decode/eac3.wasm');assert.equal(decoderPath('decoder',86020),'decode/dca.wasm');assert.throws(()=>decoderPath('decoder',999999));
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

test('CDNs returning an open-ended range are read only up to the requested window',async()=>{
  const total=50*1024**3;let cancelled=0,delivered=0;
  const reader=new RangeReader('https://cdn.test/movie',{prefetch:false,fetcher:async(_,init)=>{
    const start=Number(init.headers.Range.match(/bytes=(\d+)-/)[1]);
    return new Response(new ReadableStream({pull(c){const data=new Uint8Array(65536).fill(Math.floor(start/65536)%251);delivered+=data.length;c.enqueue(data);},cancel(){cancelled++;}}),{status:206,headers:{'content-range':`bytes ${start}-${total-1}/${total}`,'content-length':String(total-start)}});
  }});
  await reader.open();assert.equal(reader.block.data.length,65536);assert.equal(await reader.size(),BigInt(total));assert.ok(delivered<=3*65536);assert.equal(cancelled,1);
  await reader.seek(6*1024**3);const out=new Uint8Array(32);assert.equal(await reader.read(out),32);assert.equal(out[0],Math.floor(6*1024**3/65536)%251);assert.equal(reader.block.data.length,8*MiB);assert.ok(delivered<9*MiB);assert.equal(cancelled,2);await reader.stop();
});

test('range syntax tolerates surrounding whitespace while impossible intervals remain rejected',async()=>{
  const reader=new RangeReader('https://cdn.test/movie',{prefetch:false,fetcher:async()=>new Response(new Uint8Array(4),{status:206,headers:{'content-range':'BYTES  0 - 3 / 4'}})});await reader.open();assert.equal(await reader.size(),4n);await reader.stop();
  for(const range of ['bytes 100-101/100','bytes 4-3/100','bytes 0-9007199254740992/9007199254740993']){
    const broken=new RangeReader('https://cdn.test/movie',{fetcher:async()=>new Response('x',{status:206,headers:{'content-range':range}})});await assert.rejects(broken.open(),/intervallo video non valido|dimensione/);await broken.stop();
  }
});

test('RD-style ranges ending beyond EOF are clipped to the file, including an oversized initial probe',async()=>{
  const reader=new RangeReader('https://cdn.test/movie',{prefetch:false,fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Number(b);
    // This is the real CDN behavior: it echoes the requested end and length
    // even though the body physically ends at the resource size.
    return new Response(new Uint8Array(100-start).fill(73),{status:206,headers:{'content-range':`bytes ${start}-${end}/100`,'content-length':String(end-start+1)}});
  }});
  await reader.open();assert.equal(await reader.size(),100n);assert.equal(reader.block.data.length,100);assert.equal(reader.bytesRead,100);const out=new Uint8Array(256);assert.equal(await reader.read(out),100);assert.equal(out[99],73);assert.equal(await reader.read(out),-1048576);await reader.stop();
});

test('large MKV tail seeks clamp outgoing ranges to EOF instead of requesting an 8 MiB overrun',async()=>{
  const total=80138595881,requests=[];
  const reader=new RangeReader('https://cdn.test/movie',{prefetch:false,fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Number(b);requests.push({start,end});
    return new Response(new Uint8Array(Math.min(end,total-1)-start+1).fill(91),{status:206,headers:{'content-range':`bytes ${start}-${end}/${total}`,'content-length':String(end-start+1)}});
  }});
  await reader.open();await reader.seek(BigInt(total-65536));const out=new Uint8Array(65536);assert.equal(await reader.read(out),65536);assert.equal(out[0],91);assert.equal(out[65535],91);assert.equal(requests.at(-1).end,total-1);assert.equal(await reader.read(out),-1048576);
  await reader.seek(BigInt(total-1));assert.equal(await reader.read(out),1);assert.equal(requests.at(-1).end,total-1);await reader.stop();
});

test('a timed-out block retries a smaller window once and reuses it for the session',async()=>{
  const total=8*MiB,requests=[];
  const reader=new RangeReader('https://slow.test/movie',{timeout:300,fetcher:async(_,init)=>{
    const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/),start=Number(a),end=Number(b);requests.push({start,end,size:end-start+1});
    if(requests.length===1)return new Promise((_,reject)=>init.signal.addEventListener('abort',()=>reject(new DOMException('Stopped','AbortError')),{once:true}));
    return bytesResponse(start,Math.min(start+MiB-1,total-1),total);
  }});
  try{
    await reader.open();assert.equal(requests.length,2);assert.equal(requests[1].size,65536);assert.equal(reader.chunkSize,MiB);assert.equal(reader.block.data.length,65536);
    const out=new Uint8Array(65536);assert.equal(await reader.read(out),65536);await reader.next?.promise;assert.equal(requests.at(-1).size,MiB,'the reduced window is used for read-ahead');
  }finally{await reader.stop();}
});
test('missing CORS-exposed range headers have a distinct diagnostic from invalid byte offsets',async()=>{
  const reader=new RangeReader('https://cdn.test/movie',{fetcher:async()=>new Response('data',{status:206,headers:{'content-length':'4'}})});
  await assert.rejects(reader.open(),e=>e.rangeCode==='RANGE_HEADER_MISSING'&&e.message.includes('Content-Range')&&!e.message.includes('intervallo video non valido'));await reader.stop();
});
