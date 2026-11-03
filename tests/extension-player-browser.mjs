import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,writeFile,mkdtemp,rm,mkdir,cp} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve,basename} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const files=new Map();for(const name of ['pattern.mp4','pattern.m3u8','pattern-hevc-ac3.mkv',...Array.from({length:5},(_,i)=>`segment${i}.ts`)])files.set(name,await readFile(resolve('tests/fixtures/player',name)));
const calls=[],errors=[];let base,failStreams=false,passed=0;
const manifest={id:'cinematch.test',name:'Cinema di prova',version:'1.0.0',resources:['stream'],types:['movie'],idPrefixes:['tt']};
const server=createServer((req,res)=>{
  calls.push({path:req.url,authorization:req.headers.authorization,cookie:req.headers.cookie});const path=new URL(req.url,base).pathname;res.setHeader('Access-Control-Allow-Origin','*');
  if(path==='/manifest.json'){res.setHeader('Content-Type','application/json');return res.end(JSON.stringify(manifest));}
  if(path==='/bad/manifest.json'){res.setHeader('Content-Type','text/html');return res.end('<html>Not JSON</html>');}
  if(path==='/comet-like'){if(req.method==='HEAD'){res.statusCode=405;return res.end();}res.statusCode=302;res.setHeader('Location','/opaque');return res.end();}
  if(path==='/opaque'){res.setHeader('Content-Type','application/octet-stream');return res.end(files.get('pattern.m3u8'));}
  if(path==='/forbidden'){res.statusCode=403;return res.end('private token must never be shown');}
  if(path==='/stream/movie/tt0040522.json'){
    if(failStreams){res.statusCode=503;return res.end('unavailable');}
    res.setHeader('Content-Type','application/json');return res.end(JSON.stringify({streams:[{name:'MP4 di prova',title:'Video sintetico · H264 AAC · 320 × 180',url:`${base}/pattern.mp4`},{name:'HLS di prova',url:`${base}/pattern.m3u8`},{name:'HLS senza estensione',url:`${base}/play?id=test`},{name:'Comet redirect di prova',description:'HEVC AC3.mkv · etichetta prima della conversione',url:`${base}/comet-like`},{name:'MKV HEVC AC3 di prova',description:'HEVC · AC3 · 5.1',url:`${base}/pattern-hevc-ac3.mkv`},{name:'Torrent di prova',infoHash:'a'.repeat(40)},{name:'Header di prova',url:`${base}/headers.mp4`,behaviorHints:{proxyHeaders:{request:{Referer:'https://example.org'}}}},{name:'<img onerror=alert(1)>',externalUrl:'javascript:alert(1)'}]}));
  }
  const file=files.get(path==='/play'?'pattern.m3u8':basename(path));if(file){res.setHeader('Content-Type',path.endsWith('.m3u8')||path==='/play'?'application/vnd.apple.mpegurl':path.endsWith('.ts')?'video/mp2t':'video/mp4');res.setHeader('Accept-Ranges','bytes');const range=req.headers.range?.match(/bytes=(\d+)-(\d*)/);if(range){const start=Number(range[1]),end=range[2]?Math.min(Number(range[2]),file.length-1):file.length-1;res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${file.length}`);res.setHeader('Content-Length',end-start+1);return res.end(file.subarray(start,end+1));}res.setHeader('Content-Length',file.length);return res.end(file);}
  res.statusCode=404;res.end('Not found');
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));base=`http://127.0.0.1:${server.address().port}`;
const profile=await mkdtemp(join(tmpdir(),'cinematch-player-test-')),extension=join(profile,'test-extension');
// Headless Chromium cannot accept the native optional-host confirmation dialog.
// A disposable copy grants only our localhost fixture; production files stay intact.
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const extensionManifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
assert.deepEqual(extensionManifest.optional_host_permissions,['https://*/*','http://*/*']);
assert.ok(!extensionManifest.host_permissions.some(p=>p.includes('127.0.0.1')||p==='https://*/*'||p==='http://*/*'));
extensionManifest.host_permissions.push('http://127.0.0.1/*');await writeFile(join(extension,'manifest.json'),JSON.stringify(extensionManifest));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:1180,height:1000},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
const check=async(name,fn)=>{await fn();passed++;console.log(`PASS ${name}`);};
const film={id:5156,imdb_id:'tt0040522',title:'Ladri di biciclette',original_title:'Ladri di biciclette',release_date:'1948-11-24',poster_path:'/poster.jpg',credits:{crew:[{id:1,name:'Vittorio De Sica',job:'Director'}],cast:[]}};
await context.route('https://api.themoviedb.org/**',route=>route.fulfill({contentType:'application/json',body:JSON.stringify(route.request().url().includes('/search/')?{results:[film]}:film)}));
await context.route('https://image.tmdb.org/**',route=>route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')}));
await context.route('https://chat.deepseek.com/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><div class="ds-markdown"><h3>Ladri di biciclette (1948) — Regia: Vittorio De Sica</h3><p>Un film da scoprire.</p></div>'}));
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  await worker.evaluate(()=>chrome.storage.session.set({tmdbCredential:'eyJtest.payload.signature'}));
  const manager=await context.newPage();manager.on('pageerror',e=>errors.push(e.message));await manager.goto(`chrome-extension://${id}/watch.html#addons`);
  await check('addon page is dark, empty and uses scoped optional host permissions',async()=>{
    assert.equal(await manager.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),'dark');
    assert.equal(await manager.locator('#installed-empty').isVisible(),true);
    assert.equal(await manager.evaluate(()=>chrome.permissions.contains({origins:['https://unrelated.example/*']})),false);
    await mkdir('test-results',{recursive:true});await manager.evaluate(()=>document.fonts.ready);await manager.screenshot({path:'test-results/addons-empty-v30.png',fullPage:true});await manager.locator('#show-player').click();await manager.screenshot({path:'test-results/player-empty-v30.png',fullPage:true});await manager.locator('#show-addons').click();
  });
  await check('configurator links are rejected with recovery, without fetching them',async()=>{
    await manager.locator('#addon-url').fill(`${base}/configure`);await manager.getByRole('button',{name:'Aggiungi addon',exact:true}).click();await manager.waitForFunction(()=>document.querySelector('#addon-status').textContent.includes('configuratore'));assert.equal(calls.length,0);
  });
  await check('addon installs from a real JSON endpoint with the test-only localhost permission',async()=>{
    await manager.locator('#addon-url').fill(`${base}/manifest.json`);await manager.getByRole('button',{name:'Aggiungi addon',exact:true}).click();await manager.getByRole('checkbox',{name:'Attiva Cinema di prova'}).waitFor({timeout:15000}).catch(async()=>{throw new Error(await manager.locator('#addon-status').innerText());});
    assert.equal(await manager.evaluate(()=>chrome.permissions.contains({origins:['http://127.0.0.1/*']})),true);
    assert.equal((await manager.locator('#addon-status').innerText()).includes('collegato'),true);
  });
  await check('malformed manifest gives a readable error and never replaces the working addon',async()=>{
    await manager.locator('#addon-url').fill(`${base}/bad/manifest.json`);await manager.getByRole('button',{name:'Aggiungi addon',exact:true}).click();await manager.waitForFunction(()=>document.querySelector('#addon-status').classList.contains('error'));assert.equal(await manager.locator('.installed-addon').count(),1);
  });
  const chat=await context.newPage();chat.on('pageerror',e=>errors.push(e.message));await chat.goto('https://chat.deepseek.com/test');await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).waitFor({timeout:20000});
  let player;
  await check('trusted click on DeepSeek movie poster opens the extension player for the correct TMDB/IMDb film',async()=>{
    const opened=context.waitForEvent('page');await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();player=await opened;player.on('pageerror',e=>errors.push(e.message));await player.waitForURL(`chrome-extension://${id}/watch.html?movie=5156`);await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor();assert.equal(await player.locator('#film-title').innerText(),'Ladri di biciclette');assert.ok(calls.some(c=>c.path==='/stream/movie/tt0040522.json'));
  });
  await check('stream output is inert text, with torrent and header-only sources explicitly unavailable',async()=>{
    assert.equal(await player.locator('#stream-list img').count(),0);assert.equal(await player.locator('#stream-list button').count(),5);assert.ok((await player.locator('#stream-list').innerText()).includes('servizio che restituisca un flusso HTTP'));assert.ok((await player.locator('#stream-list').innerText()).includes('proxy configurato'));
  });
  await check('MP4 actually decodes and plays with controls, pause and seek',async()=>{
    await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:15000});
    assert.equal(await player.locator('video').evaluate(v=>v.videoWidth),320);assert.equal(await player.locator('video').evaluate(v=>v.controls),true);
    await player.locator('video').evaluate(v=>{v.pause();v.currentTime=2;});await player.waitForFunction(()=>Math.abs(document.querySelector('video').currentTime-2)<.2);assert.equal(await player.locator('video').evaluate(v=>v.paused),true);
  });
  await check('HLS actually loads manifest and segments and plays through local HLS.js under MV3 CSP',async()=>{
    await player.getByRole('button',{name:'Riproduci HLS di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:20000});assert.ok(calls.some(c=>c.path.endsWith('.m3u8')));assert.ok(calls.some(c=>c.path.endsWith('.ts')));await player.locator('video').evaluate(v=>v.pause());
  });
  await check('extensionless HLS proxy URL is detected from its MIME type and plays',async()=>{
    await player.getByRole('button',{name:'Riproduci HLS senza estensione da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:20000});await player.locator('video').evaluate(v=>v.pause());
  });
  await check('Comet-style redirect with HEAD rejected and wrong MIME still plays HLS, ignoring outdated filename hints',async()=>{
    await player.getByRole('button',{name:'Riproduci Comet redirect di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:20000});await player.locator('video').evaluate(v=>v.pause());
  });
  await check('real synthetic HEVC/AC3 Matroska is diagnosed from bytes, with a specific alternative action',async()=>{
    await player.getByRole('button',{name:'Riproduci MKV HEVC AC3 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('#playback-status').textContent.includes('Il file contiene codec non supportati'));
    assert.match(await player.locator('#playback-status').innerText(),/AC3/);assert.equal(await player.locator('#retry-play').innerText(),'Scegli un altro flusso');await player.waitForTimeout(200);assert.match(await player.locator('#playback-status').innerText(),/AC3/);
    for(const width of [320,1180]){await player.setViewportSize({width,height:1000});assert.ok(await player.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await player.screenshot({path:`test-results/player-codec-error-v301-${width}.png`,fullPage:true});}
    await player.locator('#retry-play').click();assert.equal(await player.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'Riproduci MP4 di prova da Cinema di prova');
    await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5);await player.locator('video').evaluate(v=>v.pause());
  });
  await check('addon requests never receive AI conversations, TMDB credential, authorization headers or cookies',async()=>{
    assert.ok(calls.every(c=>!c.authorization&&!c.cookie&&!c.path.includes('eyJtest')&&!c.path.includes('Ladri')));
  });
  await mkdir('test-results',{recursive:true});
  await check('player and addon layouts fit 320px, 600px and desktop without horizontal overflow',async()=>{
    for(const width of [320,600,1180]){
      await player.setViewportSize({width,height:1000});await player.evaluate(()=>document.fonts.ready);assert.ok(await player.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`player overflow ${width}`);await player.screenshot({path:`test-results/player-${width}.png`,fullPage:true});
      await manager.setViewportSize({width,height:1000});await manager.evaluate(()=>document.fonts.ready);assert.ok(await manager.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`addon overflow ${width}`);await manager.screenshot({path:`test-results/addons-${width}.png`,fullPage:true});
    }
  });
  await check('addon failures expose retryable source status and Refresh recovers',async()=>{
    failStreams=true;await player.locator('#refresh-streams').click();await player.getByText('Il server ha risposto con errore 503.',{exact:true}).waitFor();await player.screenshot({path:'test-results/player-error-v30.png',fullPage:true});failStreams=false;await player.locator('#refresh-streams').click();await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor();
  });
  await check('JSON file import stays in memory and handles invalid and unsafe entries',async()=>{
    await player.locator('#stream-file').setInputFiles({name:'personal.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name:'Personale',url:`${base}/pattern.mp4`}]}))});await player.getByRole('button',{name:'Riproduci Personale da personal.json'}).waitFor();
    const saved=await manager.evaluate(()=>chrome.storage.local.get(null));assert.equal(Object.keys(saved).filter(k=>k.startsWith('streamAddon:')).length,1);assert.ok(!JSON.stringify(saved).includes('pattern.mp4'));
    await player.locator('#stream-file').setInputFiles({name:'bad.json',mimeType:'application/json',buffer:Buffer.from('{bad')});await player.waitForFunction(()=>document.querySelector('#import-status').textContent.includes('JSON valido'));
  });
  await check('HTTP 403 remains a precise server error and is not overwritten by a generic codec error',async()=>{
    await player.locator('#stream-file').setInputFiles({name:'blocked.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name:'Bloccato',url:`${base}/forbidden`}]}))});await player.getByRole('button',{name:'Riproduci Bloccato da blocked.json'}).click();await player.waitForFunction(()=>document.querySelector('#playback-status').textContent.includes('(403)'));assert.ok(!(await player.locator('body').innerText()).includes('private token must never'));
  });
  await check('disabling and removing an addon updates both pages and survives a reload',async()=>{
    await manager.getByRole('checkbox',{name:'Attiva Cinema di prova'}).uncheck();await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({state:'detached'});await manager.reload();await manager.getByRole('checkbox',{name:'Attiva Cinema di prova'}).waitFor();assert.equal(await manager.getByRole('checkbox',{name:'Attiva Cinema di prova'}).isChecked(),false);
    await manager.getByRole('button',{name:'Rimuovi Cinema di prova'}).click();await manager.locator('#installed-empty').waitFor();
  });
  assert.deepEqual(errors,[]);console.log(`${passed} player/browser checks passed. No external accounts or AI calls.`);
}finally{await context.close();await new Promise(resolve=>server.close(resolve));await rm(profile,{recursive:true,force:true});}
