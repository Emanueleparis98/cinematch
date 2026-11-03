import test from 'node:test';import assert from 'node:assert/strict';
import {RangeReader} from '../extensions/cinematch/range-reader.js';
import {decoderPath} from '../extensions/cinematch/codec-player.js';
test('range reader fetches bounded blocks without HEAD, cache, referrer or credentials',async()=>{
  const requests=[],total=8*1024**3;const reader=new RangeReader('https://example.org/video',{fetcher:async(_,init)=>{requests.push(init);const [,a,b]=init.headers.Range.match(/bytes=(\d+)-(\d+)/);const start=Number(a),end=Math.min(Number(b),total-1);return new Response(new Uint8Array(end-start+1).fill(42),{status:206,headers:{'content-range':`bytes ${start}-${end}/${total}`}});}});
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
