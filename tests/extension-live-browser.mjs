import assert from 'node:assert/strict';
import {cp,mkdtemp,readFile,rm,writeFile,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const {liveOrigins}=await import('../extensions/cinematch/live-core.js');
const data=JSON.parse(await readFile('extensions/cinematch/data/live-sites.json','utf8'));
const profile=await mkdtemp(join(tmpdir(),'cinematch-live-')),extension=join(profile,'extension');
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
assert.equal(manifest.host_permissions.some(origin=>origin.includes('dlive.sx')),false,'live access must remain optional in production');
manifest.host_permissions.push(...liveOrigins(data.sites),'https://media.cinematch.test/*','https://frame.cinematch.test/*');
await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));
const mp4=await readFile('tests/fixtures/player/pattern.mp4');
const hls=await readFile('tests/fixtures/player/pattern.m3u8','utf8');
const errors=[];let passed=0,adRequests=0;
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:1280,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
context.on('page',page=>page.on('pageerror',error=>errors.push(error.message)));
await context.route(/^https?:\/\//,async route=>{
  const url=new URL(route.request().url()),cors={'Access-Control-Allow-Origin':'*','Access-Control-Allow-Headers':'*'};
  if(url.hostname==='media.cinematch.test'){
    if(url.pathname==='/broken.mp4')return route.fulfill({status:404,body:'unavailable',headers:cors});
    if(url.pathname==='/live.m3u8')return route.fulfill({contentType:'application/vnd.apple.mpegurl',body:hls,headers:cors});
    if(/segment\d+\.ts$/.test(url.pathname))return route.fulfill({contentType:'video/mp2t',body:await readFile('tests/fixtures/player/'+url.pathname.slice(1)),headers:cors});
    return route.fulfill({contentType:'video/mp4',body:mp4,headers:cors});
  }
  if(url.hostname==='frame.cinematch.test')return route.fulfill({contentType:'text/html',body:'<video controls src="https://media.cinematch.test/pattern.mp4"></video>',headers:cors});
  if(url.hostname.includes('doubleclick.net')){adRequests++;return route.fulfill({body:'window.adLoaded=true;'});}
  let body='<h1>Canale di prova</h1>';
  if(url.pathname.includes('watch'))body+='<script>const sources=["https://media.cinematch.test/pattern.mp4"];</script>';
  if(url.pathname.includes('fallback'))body+='<script>const sources=["https://media.cinematch.test/broken.mp4","https://media.cinematch.test/pattern.mp4"];</script>';
  if(url.pathname.includes('hls'))body+='<script>const file="https://media.cinematch.test/live.m3u8";</script>';
  if(url.pathname.includes('embed'))body+='<iframe src="https://frame.cinematch.test/player"></iframe>';
  if(url.pathname.includes('pending'))body+='<iframe src="https://not-granted.cinematch.test/player"></iframe>';
  if(url.pathname.includes('observed'))body+='<video preload="auto" src="https://media.cinematch.test/opaque-endpoint?id=42"></video>';
  return route.fulfill({contentType:'text/html',body:`<!doctype html><title>Diretta di prova</title><style>body{background:#101827;color:white;font:18px system-ui;padding:24px}</style>${body}`,headers:cors});
});
const check=async(name,fn)=>{await fn();passed++;console.log('PASS '+name);};
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  const settings=await context.newPage();await settings.goto(`chrome-extension://${id}/settings-popup.html?page=1`);await settings.locator('#tab-functions').click();
  await check('Live mode is off by default and can be enabled without TMDB',async()=>{
    assert.equal(await settings.locator('#live-toggle').isChecked(),false);
    await settings.locator('#live-toggle').click();
    await settings.waitForFunction(()=>document.querySelector('#live-status').textContent.startsWith('Attivo'),{},{timeout:30000});
  });
  const page=await context.newPage();
  await check(`Flussi launcher is injected on all ${data.sites.length} catalog service hosts (synthetic pages)`,async()=>{
    for(const site of data.sites){await page.goto(site.url);await page.locator('#cinematch-live-button').waitFor({timeout:8000});}
  });
  await check('unlisted hosts get no launcher',async()=>{
    await page.goto('https://unlisted.cinematch.test/');assert.equal(await page.locator('#cinematch-live-button').count(),0);
  });
  const open=async path=>{
    await page.goto('https://dlive.sx/'+path);
    const host=page.locator('#cinematch-live-button');await host.waitFor();
    // The launcher deliberately has a closed shadow root. Click its visible box.
    await host.click();
    const dialog=page.locator('#cinematch-cinema-host').locator('dialog');await dialog.waitFor({state:'visible'});
    const iframe=dialog.locator('iframe');await iframe.waitFor({state:'visible'});
    return page.frameLocator('#cinematch-cinema-host iframe');
  };
  await check('Flussi starts an extracted MP4 in the existing CineMatch player',async()=>{
    const frame=await open('watch');
    await frame.locator('#playerView').waitFor({state:'visible',timeout:20000});
    await frame.locator('#video').evaluate(video=>new Promise((resolve,reject)=>{if(video.currentTime>.1)return resolve();const timer=setTimeout(()=>reject(new Error('No decoded video')),15000);video.addEventListener('timeupdate',()=>{if(video.currentTime>.1){clearTimeout(timer);resolve();}});}));
    assert.equal(await frame.locator('#playerTitle').textContent(),'Diretta di prova');
    await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/live-player.png'});
    await page.setViewportSize({width:320,height:780});
    assert.equal(await frame.locator('.cm-controls-row').evaluate(row=>row.scrollWidth<=row.clientWidth+1),true,'live controls must fit 320px');
    await page.setViewportSize({width:1280,height:900});
  });
  await check('a failed source falls back to the next source with a bounded attempt count',async()=>{
    const frame=await open('fallback');
    await frame.locator('#video').evaluate(video=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('Fallback did not play')),30000);const check=()=>{if(video.currentTime>.1){clearTimeout(timer);video.removeEventListener('timeupdate',check);resolve();}};video.addEventListener('timeupdate',check);check();}));
    assert.match(await frame.locator('#video').getAttribute('src'),/pattern\.mp4/);
  });
  await check('authorized iframe pages are resolved without embedding their original player',async()=>{
    const frame=await open('embed');await frame.locator('#playerView').waitFor({state:'visible',timeout:20000});
    assert.equal(await frame.locator('iframe[src*="frame.cinematch.test"]').count(),0);
  });
  await check('an unauthorized iframe domain is presented as an explicit permission request',async()=>{
    const frame=await open('pending');
    await frame.getByRole('button',{name:'Consenti e cerca',exact:true}).waitFor({timeout:20000});
    assert.match(await frame.locator('#sourceList').textContent(),/not-granted\.cinematch\.test/);
    await page.screenshot({path:'test-results/live-permission.png'});
  });
  await check('HLS plays through the existing Hls.js engine',async()=>{
    const frame=await open('hls');await frame.locator('#playerView').waitFor({state:'visible',timeout:20000});
    await frame.locator('#video').evaluate(video=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('HLS did not decode')),20000);const check=()=>{if(video.currentTime>.1){clearTimeout(timer);video.removeEventListener('timeupdate',check);resolve();}};video.addEventListener('timeupdate',check);check();}));
  });
  await check('network MIME detection recovers extensionless media URLs',async()=>{
    await page.goto('https://dlive.sx/observed');await page.locator('video').waitFor();
    await page.waitForFunction(()=>document.querySelector('video').readyState>=2);
    await page.waitForTimeout(300);
    await page.locator('#cinematch-live-button').click();
    const frame=page.frameLocator('#cinematch-cinema-host iframe');
    await frame.locator('#playerView').waitFor({state:'visible',timeout:20000});
    assert.match(await frame.locator('#video').getAttribute('src'),/opaque-endpoint/);
  });
  await check('playback tokens cannot be read by the settings page',async()=>{
    const url=await page.locator('#cinematch-cinema-host iframe').getAttribute('src'),token=new URL(url).searchParams.get('liveToken');
    const result=await settings.evaluate(token=>chrome.runtime.sendMessage({type:'live-playback',token}),token);
    assert.equal(result.ok,false);assert.match(result.error,/non autorizzata/);
  });
  await check('changing the event hash invalidates a previously opened playback token',async()=>{
    const url=await page.locator('#cinematch-cinema-host iframe').getAttribute('src');
    await page.goto('https://dlive.sx/observed#another-event');
    const standalone=await context.newPage();await standalone.goto(url);
    await standalone.waitForFunction(()=>document.querySelector('#sourcesStatus').textContent.includes('pagina è cambiata'));
    assert.equal(await standalone.locator('#playerView').isVisible(),false);
    await standalone.close();
  });
  await check('known ad networks and blank popunders are blocked only while Live mode is enabled',async()=>{
    await page.goto('https://dlive.sx/');await page.locator('#cinematch-live-button').waitFor();
    assert.equal(await page.evaluate(()=>window.open('about:blank')),null);
    await page.evaluate(()=>{const script=document.createElement('script');script.src='https://ads.doubleclick.net/popup.js';document.body.append(script);});
    await page.waitForTimeout(300);assert.equal(adRequests,0);
  });
  await check('disable removes launchers, scripts and live ad rules and restores window.open',async()=>{
    await settings.bringToFront();await settings.locator('#live-toggle').click();
    await settings.waitForFunction(()=>document.querySelector('#live-status').textContent.startsWith('Disattivo'),{},{timeout:30000});
    await page.locator('#cinematch-live-button').waitFor({state:'detached'});
    const scripts=await worker.evaluate(()=>chrome.scripting.getRegisteredContentScripts());assert.equal(scripts.some(script=>script.id.startsWith('cinematch-live-')),false);
    const rules=await worker.evaluate(()=>chrome.declarativeNetRequest.getSessionRules());assert.equal(rules.some(rule=>rule.id===19001),false);
    assert.equal(await page.evaluate(()=>window.open.toString().includes('[native code]')),true);
  });
  await check('re-enabling restores the launcher in an already-open page without a reload',async()=>{
    await settings.locator('#live-toggle').click();
    await settings.waitForFunction(()=>document.querySelector('#live-status').textContent.startsWith('Attivo'),{},{timeout:30000});
    await page.locator('#cinematch-live-button').waitFor();
  });
  assert.deepEqual(errors,[]);
  console.log(`${passed} Live browser checks passed. Catalog checks use fixtures; they do not certify external streams.`);
}catch(error){for(const page of context.pages()){console.log('DEBUG',page.url());for(const frame of page.frames()){if(frame.url().includes('cinema.html'))console.log(await frame.evaluate(()=>({body:document.body.innerText,video:{src:document.querySelector('video')?.src,time:document.querySelector('video')?.currentTime,paused:document.querySelector('video')?.paused,error:document.querySelector('video')?.error?.message}})));}}throw error;}finally{await context.close();await rm(profile,{recursive:true,force:true});}
