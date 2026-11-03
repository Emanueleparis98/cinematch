import test from 'node:test';
import assert from 'node:assert/strict';
import {declaredCodecs,compatibility,inspectPrefix,probePlayback,nativePlaybackError} from '../extensions/cinematch/playback.js';
import {mediaFetch} from '../extensions/cinematch/media-access.js';
import {RangeReader} from '../extensions/cinematch/range-reader.js';
const text=value=>new TextEncoder().encode(value);
test('Comet release labels identify HEVC/AC3 and do not confuse EAC3 with AC3',()=>{
  assert.deepEqual(declaredCodecs({description:'Pacific Rim (2013) 1080p H265 ita eng AC3 5.1 sub ita eng Licdom.mkv · hevc'}),{video:'hevc',audio:'ac3'});
  assert.equal(declaredCodecs({filename:'Test.x264.E-AC3.mkv'}).audio,'eac3');assert.equal(declaredCodecs({name:'Test x264 DD+ 5.1'}).audio,'eac3');
  assert.deepEqual(declaredCodecs({filename:'test.H.264.AAC.mp4'}),{video:'h264',audio:'aac'});
});
test('codec checks query the current browser instead of assuming HEVC always fails',()=>{
  const supported=m=>m.includes('hvc1')||m.includes('avc1')||m.includes('mp4a')?'probably':'';
  const result=compatibility({video:'hevc',audio:'ac3'},supported);assert.deepEqual(result.unsupported,['ac3']);assert.match(result.warning,/AC3/);assert.equal(compatibility({video:'h264',audio:'aac'},supported).score,2);assert.deepEqual(compatibility({},()=>'' ).unsupported,[]);
  assert.deepEqual(compatibility({video:'h264',audio:'ac3',audios:['ac3','aac']},supported).unsupported,[]);
});
test('HLS is sniffed from bytes with wrong MIME, and HTML/DASH are distinguished',()=>{
  assert.equal(inspectPrefix(text('\ufeff#EXTM3U\n#EXTINF:1\na.ts'),'application/octet-stream').kind,'hls');
  assert.equal(inspectPrefix(text('<html>Denied</html>'),'text/html').kind,'page');assert.equal(inspectPrefix(text('<MPD></MPD>'),'application/dash+xml').kind,'dash');
});
test('Matroska codec IDs are read only from EBML CodecID elements',()=>{
  const bytes=new Uint8Array([0x1a,0x45,0xdf,0xa3,0x86,0x90,...text('V_MPEGH/ISO/HEVC'),0x86,0x85,...text('A_AC3')]);
  // HEVC ID is 16 bytes; malformed or arbitrary release text is not enough.
  bytes[5]=0x80+text('V_MPEGH/ISO/HEVC').length;
  assert.deepEqual(inspectPrefix(bytes).codecs,{video:'hevc',audio:'ac3'});assert.deepEqual(inspectPrefix(text('V_MPEGH/ISO/HEVC A_AC3')).codecs,{});
});
test('playback probe uses bounded GET/Range, follows playback redirects and cancels oversized bodies',async()=>{
  let init,cancelled=false;const result=await probePlayback('https://example.org/private/play',{fetcher:async(_,opts)=>{init=opts;return new Response(new ReadableStream({start(c){c.enqueue(text('#EXTM3U\n'+ 'a'.repeat(100000)));},cancel(){cancelled=true;}}));}});
  assert.equal(result.kind,'hls');assert.equal(init.method,'GET');assert.equal(init.headers.Range,'bytes=0-65535');assert.equal(init.redirect,'follow');assert.equal(init.credentials,'omit');assert.equal(cancelled,true);
});
test('HTTP failures stay specific and never echo signed URLs or private server bodies',async()=>{
  for(const code of [401,403,404,429,503])await assert.rejects(probePlayback('https://example.org/private?token=secret',{fetcher:async()=>new Response('secret',{status:code})}),error=>!error.message.includes('secret')&&error.playback===true);
  assert.match(nativePlaybackError(3),/decodificare/);assert.match(nativePlaybackError(2),/connessione/);
});
test('network/CORS failure remains uncertain instead of being falsely called a codec failure',async()=>{
  assert.equal((await probePlayback('https://example.org/private',{fetcher:async()=>{throw new TypeError('private-url');}})).kind,'unknown');
  const controller=new AbortController();controller.abort();await assert.rejects(probePlayback('https://example.org',{signal:controller.signal,fetcher:async()=>{throw new TypeError('aborted');}}),error=>error.name==='AbortError');
});

test('redirect permission recovery is scoped to this request and never reveals signed paths',async()=>{
  let listener,removed=false;
  const api={runtime:{getURL:()=> 'chrome-extension://test/'},webRequest:{onBeforeRedirect:{addListener:fn=>listener=fn,removeListener:fn=>{assert.equal(fn,listener);removed=true;}}},permissions:{contains:async({origins})=>origins[0]==='https://addon.test/*'}};
  await assert.rejects(mediaFetch('https://addon.test/play?key=private',{}, {api,fetcher:async()=>{
    listener({initiator:'https://other.test',url:'https://addon.test/play?key=private',requestId:'other',redirectUrl:'https://unrelated.test/private'});
    listener({initiator:'chrome-extension://test',url:'https://addon.test/play?key=private',requestId:'ours',redirectUrl:'https://cdn.test/private?signature=secret'});
    throw new TypeError('Failed to fetch');
  }}),e=>e.mediaOrigin==='https://cdn.test/*'&&!/private|signature|secret/.test(e.message));
  assert.equal(removed,true);
});

test('probe propagates destination permissions and range reader preserves recovery metadata',async()=>{
  const error=Object.assign(new Error('Consenti il server'),{mediaOrigin:'https://cdn.test/*'});
  await assert.rejects(probePlayback('https://addon.test/play',{fetcher:async()=>{throw error;}}),e=>e===error);
  const reader=new RangeReader('https://addon.test/play',{fetcher:async()=>{throw error;}});await assert.rejects(reader.open(),e=>e===error);assert.equal(reader.lastError,error);
});

test('browser network codes are sanitized, specific and all request listeners are removed',async()=>{
  let failureListener;const active=new Set();const event={addListener:fn=>active.add(fn),removeListener:fn=>active.delete(fn)};
  const api={runtime:{getURL:()=> 'chrome-extension://test/'},permissions:{contains:async()=>true},webRequest:{onBeforeRedirect:event,onErrorOccurred:{addListener:fn=>{failureListener=fn;active.add(fn);},removeListener:fn=>active.delete(fn)}}};
  await assert.rejects(mediaFetch('https://cdn.test/private?secret=key',{}, {api,fetcher:async()=>{failureListener({initiator:'chrome-extension://test',url:'https://cdn.test/private?secret=key',requestId:'1',error:'net::ERR_CONNECTION_REFUSED'});throw new TypeError('private URL');}}),e=>e.networkCode==='ERR_CONNECTION_REFUSED'&&e.message.includes('rifiuta')&&!/secret|private/.test(e.message));assert.equal(active.size,0);
});

test('permitted redirects pass through and abort never asks for a new host',async()=>{
  const listeners=new Set(),api={runtime:{getURL:()=> 'chrome-extension://test/'},webRequest:{onBeforeRedirect:{addListener:fn=>listeners.add(fn),removeListener:fn=>listeners.delete(fn)}},permissions:{contains:async()=>true}};
  const response=await mediaFetch('https://addon.test/play',{}, {api,fetcher:async()=>new Response('ok')});assert.equal(await response.text(),'ok');assert.equal(listeners.size,0);
  const abort=new AbortController();abort.abort();await assert.rejects(mediaFetch('https://addon.test/play',{signal:abort.signal},{api,fetcher:async()=>{throw new DOMException('Stopped','AbortError');}}),e=>e.name==='AbortError');assert.equal(listeners.size,0);
});
