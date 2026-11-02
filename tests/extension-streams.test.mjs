import test from 'node:test';
import assert from 'node:assert/strict';
import {httpUrl,manifestUrl,permissionOrigin,parseManifest,resourceUrl,parseStreams,fetchJson,MAX_JSON} from '../extensions/cinematch/streams.js';
const manifest={id:'test.addon',name:'Test',version:'1.0.0',resources:['stream'],types:['movie'],idPrefixes:['tt']};
test('manifest links normalize stremio and keep personal path and query parameters',()=>{
  assert.equal(manifestUrl('stremio://example.org/config/manifest.json?token=private'),'https://example.org/config/manifest.json?token=private');
  assert.equal(manifestUrl('https://example.org'),'https://example.org/manifest.json');
  for(const url of ['https://example.org/configure','https://example.org/app/configure','https://example.org/streams.json','javascript:alert(1)','file:///tmp/a','https://user:pass@example.org/manifest.json'])assert.throws(()=>manifestUrl(url));
  assert.equal(permissionOrigin('https://example.org/private/path?token=x'),'https://example.org/*');
});
test('manifest supports simple and advanced stream declarations, rejects catalogs and setup-required',()=>{
  assert.equal(parseManifest(manifest,'https://example.org/manifest.json').resources[0].prefixes[0],'tt');
  assert.equal(parseManifest({...manifest,resources:[{name:'stream',types:['movie'],idPrefixes:['tmdb:']}]},'https://example.org/manifest.json').resources[0].prefixes[0],'tmdb:');
  for(const data of [null,{}, {...manifest,resources:['catalog']},{...manifest,types:['series']},{...manifest,behaviorHints:{configurationRequired:true}}])assert.throws(()=>parseManifest(data,'https://example.org/manifest.json'));
});
test('movie requests use verified IMDb or explicitly supported TMDB IDs and preserve configuration',()=>{
  const addon=parseManifest(manifest,'https://example.org/private/manifest.json?lang=ita');
  assert.equal(resourceUrl(addon,{id:5,imdbId:'tt1234567'}),'https://example.org/private/stream/movie/tt1234567.json?lang=ita');
  assert.equal(resourceUrl(addon,{id:5,imdbId:null}),null);
  addon.resources[0].prefixes=['tmdb:'];assert.equal(resourceUrl(addon,{id:5,imdbId:'tt1234567'}),'https://example.org/private/stream/movie/tmdb%3A5.json?lang=ita');
});
test('stream schema distinguishes browser video, HLS, torrent, proxy, DASH and external pages',()=>{
  const streams=parseStreams({streams:[{name:'MP4',url:'https://media.example/a.mp4'},{url:'https://media.example/a.m3u8?key=x'},{infoHash:'abc',fileIdx:1},{url:'https://media.example/a',behaviorHints:{proxyHeaders:{request:{Referer:'https://example.org'}}}},{url:'https://media.example/a.mpd'},{externalUrl:'https://example.org/watch'},{ytId:'abc'}]});
  assert.deepEqual(streams.map(s=>s.kind),['video','hls','torrent','headers','dash','external','unsupported']);assert.equal(streams.filter(s=>s.playable).length,2);
});
test('stream lists deduplicate URLs and retain text as inert text, blocking executable URLs',()=>{
  const data=parseStreams([{name:'<img onerror=alert(1)>',url:'https://example.org/a',subtitles:[{url:'https://example.org/sub.vtt',lang:'it'},{url:'javascript:alert(1)'}]},{url:'https://example.org/a'},{url:'javascript:alert(1)',externalUrl:'data:text/html,x'}]);
  assert.equal(data.length,2);assert.equal(data[0].name,'<img onerror=alert(1)>');assert.equal(data[0].subtitles.length,1);assert.equal(data[1].playable,false);assert.equal(data[1].url,null);
  assert.throws(()=>parseStreams({}));assert.throws(()=>parseStreams(Array(501).fill({})));
});
test('JSON fetch omits credentials, refuses redirects, bounds size and does not expose private URLs',async()=>{
  let init;assert.deepEqual(await fetchJson('https://example.org/private?key=secret',{fetcher:async(_,opts)=>{init=opts;return new Response('{"streams":[]}');}}),{streams:[]});
  assert.equal(init.credentials,'omit');assert.equal(init.redirect,'error');assert.equal(init.cache,'no-store');assert.equal(init.referrerPolicy,'no-referrer');
  await assert.rejects(fetchJson('https://example.org/private?key=secret',{fetcher:async()=>new Response('<html>')}),/JSON valido/);
  await assert.rejects(fetchJson('https://example.org',{fetcher:async()=>new Response('x',{headers:{'content-length':String(MAX_JSON+1)}})}),/troppo grande/);
  await assert.rejects(fetchJson('https://example.org',{fetcher:async()=>new Response('x'.repeat(MAX_JSON+1))}),/troppo grande/);
  await assert.rejects(fetchJson('https://example.org/private?key=secret',{fetcher:async()=>{throw new TypeError('secret');}}),error=>!error.message.includes('secret'));
});
test('HTTP URL validation strips fragment and rejects unsafe protocols without reflecting input',()=>{
  assert.equal(httpUrl('https://example.org/a#x').href,'https://example.org/a');
  for(const bad of ['javascript:foo','data:foo','ftp://example.org/a','https://user:pw@example.org'])assert.throws(()=>httpUrl(bad));
});
