import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,writeFile,mkdtemp,rm,mkdir,cp} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve,basename} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const files=new Map();for(const name of ['pattern.mp4','pattern.m3u8','pattern-hevc-ac3.mkv','pattern-tracks.mkv','pattern-mpeg4-mp3.avi','pattern.mpd','pattern-stream0.mp4','pattern-stream1.mp4','pattern-vvc.mp4',...Array.from({length:5},(_,i)=>`segment${i}.ts`)])files.set(name,await readFile(resolve('tests/fixtures/player',name)));
const calls=[],errors=[];let base,failStreams=false,passed=0,oversizedResponses=0,slowStartStarted=false;
const manifest={id:'cinematch.test',name:'Cinema di prova',version:'1.0.0',resources:['stream'],types:['movie'],idPrefixes:['tt']};
const cdn=createServer((req,res)=>{
  const file=files.get('pattern-hevc-ac3.mkv'),range=req.headers.range?.match(/bytes=(\d+)-(\d*)/);
  res.setHeader('Content-Type','application/octet-stream');
  if(range){const start=Number(range[1]),end=Math.min(Number(range[2]||file.length-1),file.length-1);res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${file.length}`);res.setHeader('Content-Length',end-start+1);return res.end(file.subarray(start,end+1));}
  res.setHeader('Content-Length',file.length);res.end(file);
});
await new Promise(r=>cdn.listen(0,'127.0.0.2',r));
const cdnBase=`http://127.0.0.2:${cdn.address().port}`;
const server=createServer((req,res)=>{
  calls.push({path:req.url,authorization:req.headers.authorization,cookie:req.headers.cookie});const path=new URL(req.url,base).pathname;res.setHeader('Access-Control-Allow-Origin','*');
  if(path==='/cdn-redirect'){res.statusCode=302;res.setHeader('Location',`${cdnBase}/private/movie.mkv?signature=private-test-token`);return res.end();}
  if(path==='/rd-large-eof'){
    const total=80138595881,range=req.headers.range.match(/bytes=(\d+)-(\d+)/),start=Number(range[1]),end=Number(range[2]);res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${total}`);res.setHeader('Content-Length',end-start+1);return res.end(Buffer.alloc(Math.min(end,total-1)-start+1,91));
  }
  if(path==='/slow-start'){
    const file=files.get('pattern-hevc-ac3.mkv');
    const send=()=>{
      if(res.destroyed)return;
      res.setHeader('Content-Type','video/x-matroska');res.setHeader('Accept-Ranges','bytes');
      const range=req.headers.range?.match(/bytes=(\d+)-(\d*)/);const start=range?Number(range[1]):0,end=range?(range[2]?Math.min(Number(range[2]),file.length-1):file.length-1):file.length-1;
      if(range){res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${file.length}`);}
      res.setHeader('Content-Length',end-start+1);res.end(file.subarray(start,end+1));
    };
    if(slowStartStarted)return send();slowStartStarted=true;return setTimeout(send,9000);
  }
  if(path==='/manifest.json'){res.setHeader('Content-Type','application/json');return res.end(JSON.stringify(manifest));}
  if(path==='/second/manifest.json'){res.setHeader('Content-Type','application/json');return res.end(JSON.stringify({...manifest,id:'cinematch.test2',name:'Cinema di prova 2'}));}
  if(path==='/bad/manifest.json'){res.setHeader('Content-Type','text/html');return res.end('<html>Not JSON</html>');}
  if(path==='/comet-like'){if(req.method==='HEAD'){res.statusCode=405;return res.end();}res.statusCode=302;res.setHeader('Location','/opaque');return res.end();}
  if(path==='/opaque'){res.setHeader('Content-Type','application/octet-stream');return res.end(files.get('pattern.m3u8'));}
  if(path==='/forbidden'){res.statusCode=403;return res.end('private token must never be shown');}
  if(path==='/stream/movie/tt0040522.json'){
    if(failStreams){res.statusCode=503;return res.end('unavailable');}
    res.setHeader('Content-Type','application/json');return res.end(JSON.stringify({streams:[{name:'MP4 di prova',title:'Video sintetico · H264 AAC · 320 × 180',url:`${base}/pattern.mp4`},{name:'HLS di prova',url:`${base}/pattern.m3u8`},{name:'HLS senza estensione',url:`${base}/play?id=test`},{name:'Comet redirect di prova',description:'HEVC AC3.mkv · etichetta prima della conversione',url:`${base}/comet-like`},{name:'Redirect CDN di prova',description:'Redirect verso un altro server',url:`${base}/cdn-redirect`},{name:'Permesso CDN di prova',url:`${cdnBase}/pattern.mp4`},{name:'MKV HEVC AC3 di prova',description:'HEVC · AC3 · 5.1',url:`${base}/pattern-hevc-ac3.mkv`},{name:'Torrent di prova',infoHash:'a'.repeat(40)},{name:'Header di prova',url:`${base}/headers.mp4`,behaviorHints:{proxyHeaders:{request:{Referer:'https://example.org'}}}},{name:'<img onerror=alert(1)>',externalUrl:'javascript:alert(1)'}]}));
  }
  const file=files.get(path==='/play'?'pattern.m3u8':path==='/oversized-range.mkv'?'pattern-hevc-ac3.mkv':basename(path));if(file){res.setHeader('Content-Type',path.endsWith('.m3u8')||path==='/play'?'application/vnd.apple.mpegurl':path.endsWith('.ts')?'video/mp2t':'video/mp4');res.setHeader('Accept-Ranges','bytes');const range=req.headers.range?.match(/bytes=(\d+)-(\d*)/);if(range){const start=Number(range[1]),end=path==='/oversized-range.mkv'?file.length-1:range[2]?Math.min(Number(range[2]),file.length-1):file.length-1;if(path==='/oversized-range.mkv'&&end>Number(range[2]))oversizedResponses++;res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${file.length}`);res.setHeader('Content-Length',end-start+1);return res.end(file.subarray(start,end+1));}res.setHeader('Content-Length',file.length);return res.end(file);}
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
  const focusedChatLabel=()=>chat.evaluate(()=>{let el=document.activeElement;while(el?.shadowRoot?.activeElement)el=el.shadowRoot.activeElement;return el?.getAttribute('aria-label')??null;});
  let player;
  await check('the explicit Flussi button opens the same in-chat source picker',async()=>{
    const button=chat.getByRole('button',{name:'Cerca i flussi per Ladri di biciclette',exact:true});
    await button.waitFor({timeout:20000});await button.click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
    assert.equal(await focusedChatLabel(),'Cerca i flussi per Ladri di biciclette');
  });
  await check('trusted click on DeepSeek movie poster opens the in-chat overlay with the extension player for the correct TMDB/IMDb film',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    assert.equal(await chat.evaluate(()=>document.documentElement.style.overflow),'hidden');
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible',timeout:10000});
    assert.ok((await iframe.getAttribute('allow')).includes('fullscreen'));
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame,'the extension player must be a real child frame');
    await frame.waitForFunction(()=>location.href.includes('cinema.html')&&new URL(location.href).searchParams.get('movie')==='5156',{},{timeout:15000});
    await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    assert.equal(await frame.locator('#movieTitle').innerText(),'Ladri di biciclette');
    assert.ok(calls.some(c=>c.path==='/stream/movie/tt0040522.json'));
    assert.equal(await dialog.locator('.cm-loading').isVisible(),false);
    const pickerWidth=await dialog.evaluate(node=>node.getBoundingClientRect().width);
    assert.ok(pickerWidth<=980,`picker dialog should stay contained, got ${pickerWidth}px`);
    await mkdir('test-results',{recursive:true});await chat.screenshot({path:'test-results/overlay-picker-v330.png'});
    await frame.locator('#playBest').click();
    await frame.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:15000});
    assert.ok(await dialog.evaluate(node=>node.getBoundingClientRect().width)>pickerWidth);
    await chat.screenshot({path:'test-results/overlay-player-v330.png'});
    await frame.locator('#fullscreen').click();
    await frame.waitForFunction(()=>document.fullscreenElement?.id==='stage',{},{timeout:5000});
    await frame.evaluate(()=>document.exitFullscreen()).catch(()=>{});
    await frame.waitForFunction(()=>!document.fullscreenElement,{},{timeout:5000});
    await chat.waitForTimeout(150);
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
    assert.equal(await chat.evaluate(()=>document.documentElement.style.overflow),'');
    assert.equal(await focusedChatLabel(),'Guarda Ladri di biciclette con CineMatch');
  });
  await check('the in-chat overlay plays HLS through the native HLS.js path',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci HLS di prova da Cinema di prova'}).waitFor({timeout:20000});
    await frame.waitForTimeout(1200);
    await frame.getByRole('button',{name:'Riproduci HLS di prova da Cinema di prova'}).click();
    await frame.waitForTimeout(1000);
    if(await frame.locator('#bigPlay').isVisible())await frame.locator('#bigPlay').click();
    await frame.waitForFunction(()=>document.querySelector('video').currentTime>.5,{},{timeout:30000}).catch(async()=>{throw new Error(await frame.locator('#playerMessage').innerText());});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('the in-chat Consenti button requests the missing media permission',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    const source=frame.getByRole('button',{name:'Riproduci Permesso CDN di prova da Cinema di prova'});
    await source.waitFor({timeout:20000});
    await frame.waitForTimeout(1200);
    await frame.evaluate(()=>{
      window.__originalPermissionRequest=chrome.permissions.request.bind(chrome.permissions);
      window.__mediaPermissionRequests=[];
      chrome.permissions.request=options=>{
        if(options.origins?.some(origin=>origin.startsWith('http://127.0.0.2/'))){window.__mediaPermissionRequests.push(options);return Promise.resolve(false);}
        return window.__originalPermissionRequest(options);
      };
    });
    try{
      await source.click();
      const consent=frame.locator('#messageAction');
      await consent.waitFor({state:'visible',timeout:15000});
      assert.equal(await consent.innerText(),'Consenti 127.0.0.2');
      await frame.evaluate(()=>{window.__mediaPermissionRequests=[];});
      await consent.evaluate(button=>button.click());
      await frame.waitForFunction(()=>window.__mediaPermissionRequests.length>0,{},{timeout:10000});
      const requests=await frame.evaluate(()=>window.__mediaPermissionRequests);
      assert.deepEqual(requests.at(-1).origins,['http://127.0.0.2/*']);
      await consent.waitFor({state:'visible',timeout:5000});
      assert.equal(await consent.innerText(),'Consenti 127.0.0.2');
    }finally{
      await frame.evaluate(()=>{if(window.__originalPermissionRequest)chrome.permissions.request=window.__originalPermissionRequest;}).catch(()=>{});
      await dialog.locator('button.cm-close').click().catch(()=>{});await dialog.waitFor({state:'hidden',timeout:5000}).catch(()=>{});
    }
  });
  await check('the source picker filters by quality, type and language',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    await frame.locator('#jsonFile').setInputFiles({name:'filtri.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[
      {name:'Filtro 4K ITA',url:`${base}/pattern.mp4?filtro=4k`},
      {name:'Filtro 1080p ENG',url:`${base}/pattern.mp4?filtro=1080p`},
      {name:'Filtro Torrent 720p ITA',infoHash:'b'.repeat(40)}
    ]}))});
    await frame.locator('#sourceFilters').waitFor({state:'visible'});
    await frame.locator('#qualityFilter').selectOption('1080p');
    assert.equal(await frame.locator('.cm-source-card').count(),1);
    assert.equal(await frame.locator('.cm-source-name').innerText(),'Filtro 1080p ENG');
    assert.match(await frame.locator('#sourcesStatus').innerText(),/1 di \d+ sorgenti/);
    assert.equal(await frame.locator('#resetFilters').isEnabled(),true);
    await frame.locator('#resetFilters').click();
    assert.equal(await frame.locator('#resetFilters').isDisabled(),true);
    await frame.locator('#typeFilter').selectOption('torrent');
    assert.ok(await frame.locator('.cm-source-card').count()>=2);
    await frame.locator('#languageFilter').selectOption('ITA');
    assert.equal(await frame.locator('.cm-source-card').count(),1);
    assert.match(await frame.locator('.cm-source-name').innerText(),/Filtro Torrent 720p ITA/);
    await frame.locator('#resetFilters').click();
    await frame.locator('#languageFilter').selectOption('ITA');
    assert.equal(await frame.locator('.cm-source-card').count(),2);
    await frame.locator('#resetFilters').click();
    await frame.locator('#qualityFilter').selectOption('720p');
    assert.equal(await frame.locator('.cm-source-card').count(),1);
    assert.match(await frame.locator('.cm-source-name').innerText(),/Filtro Torrent 720p ITA/);
    await mkdir('test-results',{recursive:true});await chat.screenshot({path:'test-results/overlay-filters.png'});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('the picker follows a dark chat theme and keeps the player dark',async()=>{
    await chat.evaluate(()=>{document.body.style.background='#14161a';document.body.style.color='#ececec';});
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    assert.equal(await frame.evaluate(()=>document.documentElement.dataset.theme),'dark');
    assert.equal(await frame.evaluate(()=>getComputedStyle(document.body).backgroundColor),'rgb(15, 20, 29)');
    await chat.evaluate(()=>{document.body.style.background='#f7f7f8';document.body.style.color='#202124';});
    await frame.waitForFunction(()=>document.documentElement.dataset.theme==='light',{},{timeout:5000});
    await chat.evaluate(()=>{document.body.style.background='#14161a';document.body.style.color='#ececec';});
    await frame.waitForFunction(()=>document.documentElement.dataset.theme==='dark',{},{timeout:5000});
    await frame.locator('#jsonFile').setInputFiles({name:'extra.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name:'Extra test',url:`${base}/pattern.mp4`}]}))});
    await frame.getByRole('button',{name:'Riproduci Extra test da extra.json'}).waitFor({timeout:5000});
    await mkdir('test-results',{recursive:true});await chat.screenshot({path:'test-results/overlay-dark-v330.png'});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('a saved session resume point is offered in the picker',async()=>{
    await manager.evaluate(()=>chrome.storage.session.set({'resume:5156':6}));
    try{
      await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
      const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
      const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
      let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
      assert.ok(frame);
      await frame.locator('#resumePlay').waitFor({state:'visible',timeout:10000});
      assert.equal(await frame.locator('#resumeTime').innerText(),'0:06');
      await frame.locator('#resumePlay').click();
      await frame.waitForFunction(()=>!document.querySelector('#playerView').hidden,{},{timeout:15000});
      await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
    }finally{await manager.evaluate(()=>chrome.storage.session.remove('resume:5156'));}
  });
  await check('the panel manages addons without leaving the chat',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    await frame.getByRole('button',{name:'Addon',exact:true}).click();
    await chat.waitForTimeout(250);
    if(!await frame.locator('#addonsView').isVisible())await frame.evaluate(()=>globalThis.CineMatchCinemaApp?.showAddons());
    await frame.locator('#addonsView').waitFor({state:'visible',timeout:5000});
    const addonList=frame.locator('#addonList');
    await addonList.getByText('Cinema di prova',{exact:true}).waitFor({timeout:5000});
    await frame.locator('#addonUrl').fill(`${base}/second/manifest.json`);
    await frame.getByRole('button',{name:'Aggiungi addon',exact:true}).click();
    await addonList.getByText('Cinema di prova 2',{exact:true}).waitFor({timeout:15000});
    assert.equal(await frame.evaluate(async()=>Object.keys(await chrome.storage.local.get(null)).filter(key=>key.startsWith('streamAddon:')).length),2);
    await mkdir('test-results',{recursive:true});await chat.screenshot({path:'test-results/overlay-addons-v340.png'});
    await addonList.getByRole('checkbox',{name:'Attiva Cinema di prova 2'}).uncheck();
    await frame.waitForFunction(async()=>{const data=await chrome.storage.local.get(null);return Object.entries(data).some(([key,value])=>key.startsWith('streamAddon:')&&value.name==='Cinema di prova 2'&&value.enabled===false);},{},{timeout:5000});
    await addonList.getByRole('button',{name:'Rimuovi Cinema di prova 2'}).click();
    await addonList.getByText('Cinema di prova 2',{exact:true}).waitFor({state:'detached',timeout:5000});
    assert.equal(await frame.evaluate(async()=>Object.keys(await chrome.storage.local.get(null)).filter(key=>key.startsWith('streamAddon:')).length),1);
    await frame.locator('#addonsBack').click();
    await frame.locator('#pickerView').waitFor({state:'visible',timeout:5000});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('returning to the picker cancels a source still waiting for its first byte',async()=>{
    const delayed=`${base}/pattern.mp4`;
    const handler=async route=>{await new Promise(resolve=>setTimeout(resolve,1500));await route.continue().catch(()=>{});};
    await context.route(delayed,handler);
    try{
      await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
      const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible'});
      const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
      let frame=null;for(let i=0;i<30&&!frame;i++){frame=await (await iframe.elementHandle()).contentFrame();if(!frame)await chat.waitForTimeout(100);}
      assert.ok(frame);
      await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).click();
      await frame.locator('#playerView').waitFor({state:'visible'});
      await frame.locator('#backToPicker').click();
      await frame.waitForTimeout(1900);
      assert.equal(await frame.locator('#pickerView').isVisible(),true);
      assert.equal(await frame.locator('#video').evaluate(video=>video.paused&&!video.getAttribute('src')),true);
      assert.equal(await frame.locator('#playerMessage').isVisible(),false);
      await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden'});
    }finally{await context.unroute(delayed,handler);}
  });
  await check('the in-chat overlay plays an integrated MKV HEVC/AC3 stream inside the same dialog',async()=>{
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci MKV HEVC AC3 di prova da Cinema di prova'}).waitFor({timeout:20000});
    await frame.getByRole('button',{name:'Riproduci MKV HEVC AC3 di prova da Cinema di prova'}).click();
    await frame.waitForFunction(()=>document.querySelector('#codecScreen').dataset.audio==='decoded'&&document.querySelector('#codecScreen').dataset.video==='decoded'&&document.querySelector('#playerLoading').hidden,{},{timeout:60000}).catch(async()=>{throw new Error(await (await frame.locator('#playerMessage').count()?frame.locator('#playerMessage').innerText():frame.locator('#playerLoading').innerText()));});
    await frame.waitForTimeout(400);
    await mkdir('test-results',{recursive:true});await chat.screenshot({path:'test-results/overlay-integrated-v330.png'});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('the in-chat overlay closes with Esc from inside the iframe and restores focus without a new tab',async()=>{
    const pagesBefore=context.pages().length;
    await chat.getByRole('link',{name:'Guarda Ladri di biciclette con CineMatch',exact:true}).click();
    const dialog=chat.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await chat.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    await frame.evaluate(()=>document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})));
    await dialog.waitFor({state:'hidden',timeout:5000});
    assert.equal(await focusedChatLabel(),'Guarda Ladri di biciclette con CineMatch');
    assert.equal(context.pages().length,pagesBefore,'overlay must not create a new tab');
  });
  await check('the standalone player remains available for addon management and full player checks',async()=>{
    player=await context.newPage();player.on('pageerror',e=>{errors.push(e.message);console.log('PAGE ERROR',e.message);});player.on('response',r=>{if(r.status()>=400&&r.url().includes('.wasm'))console.log('WASM ERROR',new URL(r.url()).pathname,r.status());});
    await player.goto(`chrome-extension://${id}/watch.html?movie=5156`);
    await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).waitFor({timeout:20000});
    assert.equal(await player.locator('#film-title').innerText(),'Ladri di biciclette');
  });
  await check('stream output is inert text, with torrent and header-only sources explicitly unavailable',async()=>{
    assert.equal(await player.locator('#stream-list img').count(),0);assert.equal(await player.locator('#stream-list button').count(),7);assert.ok((await player.locator('#stream-list').innerText()).includes('servizio che restituisca un flusso HTTP'));assert.ok((await player.locator('#stream-list').innerText()).includes('proxy configurato'));
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
  await check('real HEVC/AC3 5.1 Matroska plays through integrated decoders with pause, seek and responsive controls',async()=>{
    await player.getByRole('button',{name:'Riproduci MKV HEVC AC3 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded'&&Number(document.querySelector('#codec-seek').value)>.3,{},{timeout:30000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
    await player.locator('#codec-toggle').click();await player.waitForFunction(()=>document.querySelector('#playback-status').textContent==='In pausa');assert.equal(await player.locator('#codec-controls').isVisible(),true);
    await player.locator('#codec-seek').evaluate(e=>{e.value='2.5';e.dispatchEvent(new Event('change',{bubbles:true}));});await player.waitForFunction(()=>Number(document.querySelector('#codec-seek').value)>2);
    await player.waitForFunction(()=>document.querySelector('#codec-time').value.startsWith('0:02'));
    for(const width of [320,1180]){await player.setViewportSize({width,height:1000});assert.ok(await player.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await player.screenshot({path:`test-results/player-integrated-v310-${width}.png`,fullPage:true});}
    await player.locator('#codec-fullscreen').click();await player.waitForFunction(()=>document.fullscreenElement?.id==='playback-stage');await player.screenshot({path:'test-results/player-fullscreen-v310.png'});await player.locator('#codec-fullscreen').click();await player.waitForFunction(()=>!document.fullscreenElement);
    await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.5);await player.locator('video').evaluate(v=>v.pause());
  });
  await check('addon requests never receive AI conversations, TMDB credential, authorization headers or cookies',async()=>{
    assert.ok(calls.every(c=>!c.authorization&&!c.cookie&&!c.path.includes('eyJtest')&&!c.path.includes('Ladri')));
  });
  const importSource=async(name,file)=>{await player.locator('#stream-file').setInputFiles({name:'codec.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name,url:`${base}/${file}`}]}))});await player.getByRole('button',{name:`Riproduci ${name} da codec.json`}).click();};
  await check('RD-style virtual 80 GB file tail reads clamp ranges to EOF in the extension',async()=>{
    const result=await player.evaluate(async url=>{
      const {RangeReader}=await import('./range-reader.js'),reader=new RangeReader(url,{prefetch:false});
      try{await reader.open();const total=await reader.size();await reader.seek(total-65536n);const data=new Uint8Array(65536),count=await reader.read(data);return {total:String(total),count,first:data[0],last:data.at(-1),bytes:reader.bytesRead,eof:await reader.read(data)};}finally{await reader.stop();}
    },`${base}/rd-large-eof`);
    assert.deepEqual(result,{total:'80138595881',count:65536,first:91,last:91,bytes:131072,eof:-1048576});
  });
  await check('a first byte slower than the old probe timeout still opens the integrated decoder',async()=>{
    await importSource('Avvio lento','slow-start');
    await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:30000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
    assert.ok(!(await player.locator('#playback-status').innerText()).includes('Riproduzione terminata'));
    await player.locator('#codec-toggle').click();
  });
  await check('CDN responses larger than the requested range decode HEVC/AC3 and allow seeking',async()=>{
    const before=oversizedResponses;await importSource('CDN range HEVC AC3','oversized-range.mkv');
    await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:30000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
    assert.ok(oversizedResponses>before,'fixture must actually return more bytes than requested');
    await player.locator('#codec-toggle').click();await player.locator('#codec-seek').evaluate(e=>{e.value='2.5';e.dispatchEvent(new Event('change',{bubbles:true}));});await player.waitForFunction(()=>document.querySelector('#codec-time').value.startsWith('0:02'));assert.ok(!(await player.locator('#playback-status').innerText()).includes('intervallo video non valido'));
  });
  await check('H.266/VVC MP4 decodes with the bundled software decoder',async()=>{
    await importSource('VVC H.266','pattern-vvc.mp4');
    await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:30000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
    assert.equal(await player.locator('#playback-stage').getAttribute('data-engine'),'integrated');
    await player.locator('#codec-toggle').click();
  });
  await check('audio languages and embedded subtitles can be selected in the integrated player',async()=>{
    await importSource('Tracce','pattern-tracks.mkv');await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded',{},{timeout:20000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
    assert.equal(await player.locator('#codec-audio-wrap').isVisible(),true);assert.equal(await player.locator('#codec-subtitle-wrap').isVisible(),true);assert.equal(await player.locator('#codec-audio option').count(),2);
    const audio=await player.locator('#codec-audio option').nth(1).getAttribute('value');await player.locator('#codec-audio').selectOption(audio);const sub=await player.locator('#codec-subtitle option').nth(1).getAttribute('value');await player.locator('#codec-subtitle').selectOption(sub);
    await player.getByText('Sottotitoli di prova.',{exact:true}).waitFor({timeout:5000});await player.locator('#codec-toggle').click();await player.screenshot({path:'test-results/player-tracks-v310.png',fullPage:true});
    await player.locator('#codec-subtitle').selectOption('off');
  });
  await check('DASH plays with the integrated engine without a native app',async()=>{
    await importSource('DASH','pattern.mpd');await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:20000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});await player.locator('#codec-toggle').click();
  });
  await check('legacy AVI automatically falls back to integrated decoding and switching back stops the old player',async()=>{
    for(let attempt=0;attempt<3;attempt++){
      await importSource('AVI','pattern-mpeg4-mp3.avi');await player.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:20000}).catch(async()=>{throw new Error(await player.locator('#playback-status').innerText());});
      await player.getByRole('button',{name:'Riproduci MP4 di prova da Cinema di prova'}).click();await player.waitForFunction(()=>document.querySelector('video').currentTime>.3);assert.equal(await player.locator('#codec-controls').isVisible(),false);await player.locator('video').evaluate(v=>v.pause());
    }
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
  await check('RD download permission is requested in the trusted playback click before any network access',async()=>{
    await player.evaluate(()=>{window.originalPermissionRequest=chrome.permissions.request;chrome.permissions.request=async args=>{window.requestedPlaybackPermission={...args,activation:navigator.userActivation.isActive};return false;};});
    try{await importSource('[RD⚡] Comet di prova','pattern-hevc-ac3.mkv');await player.waitForFunction(()=>window.requestedPlaybackPermission);const request=await player.evaluate(()=>window.requestedPlaybackPermission);assert.ok(request.origins.includes('https://*.download.real-debrid.com/*'));assert.ok(request.origins.includes('http://127.0.0.1/*'));assert.equal(request.activation,true);await player.locator('#grant-media').waitFor({state:'visible'});assert.equal(await player.locator('#grant-media').innerText(),'Consenti Real-Debrid');}
    finally{await player.evaluate(()=>{chrome.permissions.request=window.originalPermissionRequest;delete window.originalPermissionRequest;});}
  });
  await check('cross-host redirect without CORS reproduces the network error and offers the exact CDN permission',async()=>{
    assert.equal(await player.evaluate(()=>chrome.permissions.contains({origins:['http://127.0.0.2/*']})),false);
    assert.equal(await player.evaluate(async url=>{try{await fetch(url,{headers:{Range:'bytes=0-65535'}});return false;}catch{return true;}},`${base}/cdn-redirect`),true,'unmodified fetch must reproduce the reported network failure');
    await importSource('Redirect MKV HEVC AC3','cdn-redirect');
    await player.locator('#grant-media').waitFor({state:'visible',timeout:12000});assert.equal(await player.locator('#grant-media').innerText(),'Consenti 127.0.0.2');assert.match(await player.locator('#playback-status').innerText(),/altro server/);assert.ok(!(await player.locator('body').innerText()).includes('private-test-token'));
  });
  await check('the same no-CORS HEVC/AC3 redirect decodes with the destination host granted',async()=>{
    // Separate disposable profile supplies the optional grant that headless
    // Chromium cannot accept through its native permission prompt.
    extensionManifest.host_permissions.push('http://127.0.0.2/*','https://*.download.real-debrid.com/*');await writeFile(join(extension,'manifest.json'),JSON.stringify(extensionManifest));
    const granted=await chromium.launchPersistentContext(join(profile,'granted'),{channel:'chromium',headless:true,args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
    try{const w=granted.serviceWorkers()[0]||await granted.waitForEvent('serviceworker'),host=new URL(w.url()).host,p=await granted.newPage();await p.goto(`chrome-extension://${host}/watch.html`);await p.locator('#stream-file').setInputFiles({name:'redirect.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name:'CDN MKV HEVC AC3',url:`${base}/cdn-redirect`}]}))});await p.getByRole('button',{name:'Riproduci CDN MKV HEVC AC3 da redirect.json'}).click();await p.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:30000}).catch(async()=>{throw new Error(await p.locator('#playback-status').innerText());});assert.equal(await p.locator('#grant-media').isVisible(),false);
      await granted.route('https://*.download.real-debrid.com/cinematch-fixture/**',async route=>{
        const file=files.get('pattern-hevc-ac3.mkv'),match=route.request().headers().range?.match(/bytes=(\d+)-(\d+)/),start=match?Number(match[1]):0,end=match?Math.min(Number(match[2]),file.length-1):file.length-1;
        await route.fulfill({status:206,headers:{'Content-Type':'application/force-download','Content-Range':`bytes ${start}-${end}/${file.length}`,'Content-Length':String(end-start+1)},body:file.subarray(start,end+1)});
      });
      await check('one download-domain grant covers two rotating RD hosts without server CORS headers',async()=>{
        for(const host of ['23-4','104-4']){
          assert.equal(await p.evaluate(host=>chrome.permissions.contains({origins:[`https://${host}.download.real-debrid.com/*`]}),host),true);
          await p.locator('#stream-file').setInputFiles({name:'rd-fixture.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify({streams:[{name:'[RD⚡] HEVC AC3 '+host,url:`https://${host}.download.real-debrid.com/cinematch-fixture/movie.mkv`}]}))});
          await p.getByRole('button',{name:`Riproduci [RD⚡] HEVC AC3 ${host} da rd-fixture.json`}).click();await p.waitForFunction(()=>document.querySelector('#codec-screen').dataset.audio==='decoded'&&document.querySelector('#codec-screen').dataset.video==='decoded',{},{timeout:30000}).catch(async()=>{throw new Error(await p.locator('#playback-status').innerText());});assert.equal(await p.locator('#grant-media').isVisible(),false);
        }
      });
    }
    finally{await granted.close();}
  });
  assert.deepEqual(errors,[]);console.log(`${passed} player/browser checks passed. No external accounts or AI calls.`);
}finally{await context.close();server.closeAllConnections();cdn.closeAllConnections();await Promise.all([new Promise(resolve=>server.close(resolve)),new Promise(resolve=>cdn.close(resolve))]);await rm(profile,{recursive:true,force:true});}
