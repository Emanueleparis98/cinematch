import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {publicLiveUrl,liveKind,matchLiveSite,liveOrigins,extractLiveUrls,extractLiveFrames,uniqueLiveSources} from '../extensions/cinematch/live-core.js';
const catalog=JSON.parse(await readFile(new URL('../extensions/cinematch/data/live-sites.json',import.meta.url)));

test('catalog includes both FMHY live sections and every service host can match',()=>{
  assert.ok(catalog.sites.length>=120);
  assert.ok(catalog.sites.some(site=>site.sections.includes('tv')));
  assert.ok(catalog.sites.some(site=>site.sections.includes('sports')));
  for(const site of catalog.sites)assert.ok(matchLiveSite(site.url,catalog.sites),site.url);
  assert.equal(new Set(catalog.sites.map(site=>site.host+site.path)).size,catalog.sites.length);
  assert.equal(catalog.sites.some(site=>['discord.gg','t.me','beacons.ai'].includes(site.host)),false);
});
test('exact hosts, www aliases and shared-platform paths do not match impostor sites',()=>{
  assert.ok(matchLiveSite('https://www.dlive.sx/watch/42',catalog.sites));
  for(const url of ['https://dlive.sx.evil.test/watch','https://evil-dlive.sx/','https://www.reddit.com/r/other/','https://zhangboheng.github.io/other-project/','https://redd.it/something-else'])assert.equal(matchLiveSite(url,catalog.sites),null,url);
  assert.ok(matchLiveSite('https://zhangboheng.github.io/Easy-Web-TV-M3u8/routes/tv.html',catalog.sites));
});
test('permissions contain only explicit HTTP(S) origins',()=>{
  const origins=liveOrigins(catalog.sites);
  assert.equal(new Set(origins).size,origins.length);
  assert.ok(origins.every(origin=>/^https?:\/\/[^*]+\/\*$/.test(origin)));
});
test('unsafe URLs, credentials, private addresses and ad networks are rejected',()=>{
  for(const value of ['javascript:alert(1)','data:video/mp4,test','blob:https://dlive.sx/id','file:///movie.mp4','https://u:secret@video.test/a.mp4','http://127.0.0.1/a.mp4','http://2130706433/a.mp4','http://localhost/a.mp4','http://169.254.169.254/a.mp4','http://[::1]/a.mp4','https://foo.local/a.mp4','https://ads.doubleclick.net/a.mp4'])assert.equal(publicLiveUrl(value),null,value);
  assert.equal(publicLiveUrl('/live/master.m3u8?sig=a%2Fb#x','https://media.test/player/'),'https://media.test/live/master.m3u8?sig=a%2Fb');
});
test('media type detection ignores segments and subtitles, including MIME disguised segments',()=>{
  assert.equal(liveKind('https://cdn.test/master.m3u8?token=x'),'hls');
  assert.equal(liveKind('https://cdn.test/dynamic?id=1','application/vnd.apple.mpegurl'),'hls');
  assert.equal(liveKind('https://cdn.test/dynamic?id=2','application/dash+xml'),'dash');
  assert.equal(liveKind('https://cdn.test/watch','video/mp4'),'video');
  for(const path of ['segment.ts','chunk.m4s','sub.vtt','audio.aac','image.png'])assert.equal(liveKind('https://cdn.test/'+path,'video/mp4'),null);
  assert.equal(liveKind('https://cdn.test/player.html','text/html'),null);
});
test('extracts escaped JSON, HTML entities, relative files and protocol-relative sources without evaluating code',()=>{
  const html=String.raw`<script>evil(); const a={"file":"https:\/\/cdn.test\/master.m3u8?a=1\u0026b=2"};</script><video src="/live/event.mpd?a=1&amp;b=2"></video><source src="//cdn.test/movie.mp4"><iframe src="https://frame.test/embed?id=1&amp;lang=it"></iframe><img src="https://cdn.test/ad.png">`;
  const sources=extractLiveUrls(html,'https://site.test/event');
  assert.deepEqual(sources.map(source=>source.url).sort(),['https://cdn.test/master.m3u8?a=1&b=2','https://cdn.test/movie.mp4','https://site.test/live/event.mpd?a=1&b=2'].sort());
  assert.deepEqual(extractLiveFrames(html,'https://site.test/event'),['https://frame.test/embed?id=1&lang=it']);
});
test('deduplicates streams, preserves MIME for extensionless endpoints and caps hostile pages',()=>{
  const sources=uniqueLiveSources([{url:'https://cdn.test/dynamic?id=1',mime:'application/vnd.apple.mpegurl'},{url:'https://cdn.test/dynamic?id=1',mime:'application/vnd.apple.mpegurl'},{url:'https://cdn.test/segment.ts'}]);
  assert.equal(sources.length,1);assert.equal(sources[0].kind,'hls');
  assert.equal(uniqueLiveSources(Array.from({length:300},(_,i)=>({url:`https://cdn.test/${i}.m3u8`}))).length,40);
  assert.equal(extractLiveFrames(Array.from({length:200},(_,i)=>`<iframe src="https://frame.test/${i}"></iframe>`).join(''),'https://site.test/').length,12);
});
test('literal base64 URLs are decoded as data, without evaluating page code',()=>{
  const url='https://cdn.test/live/master.m3u8?token=x';
  assert.deepEqual(extractLiveUrls(`const file=atob('${btoa(url)}'); throw new Error('must not run');`,'https://site.test/').map(source=>source.url),[url]);
});
test('standard literal packed player configuration is decoded without eval',()=>{
  const packed="eval(function(p,a,c,k,e,r){return p}('0=\"1://2.3/4.5\"',6,6,'file|https|cdn|test|master|m3u8'.split('|'),0,{}))";
  assert.deepEqual(extractLiveUrls(packed,'https://site.test/').map(source=>source.url),['https://cdn.test/master.m3u8']);
});
