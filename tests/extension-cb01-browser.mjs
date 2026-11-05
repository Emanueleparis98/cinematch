import assert from 'node:assert/strict';
import {cp,mkdtemp,readFile,rm,writeFile,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');

const CB01_ORIGINS=[
  'https://cb01one.org/*','https://www.cb01one.org/*','https://stayonline.pro/*','https://uprot.net/*',
  'https://*.mxdrop.click/*','https://*.m1xdrop.net/*','https://*.mixdrop.co/*','https://*.mixdrop.ag/*',
  'https://*.mxdrop.to/*','https://*.mixdrop.to/*','https://*.mxcontent.net/*'
];
const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64');
const mp4=await readFile(resolve('tests/fixtures/player/pattern.mp4'));

const profile=await mkdtemp(join(tmpdir(),'cinematch-cb01-browser-')),extension=join(profile,'test-extension');
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
const productionManifest=JSON.parse(await readFile('extensions/cinematch/manifest.json','utf8'));
assert.ok(!productionManifest.host_permissions.some(origin=>origin.includes('cb01one.org')));
assert.ok(productionManifest.permissions.includes('declarativeNetRequest'));
assert.equal(productionManifest.declarative_net_request.rule_resources[0].id,'cb01_ads');
manifest.host_permissions.push(...CB01_ORIGINS);
await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));

const home=`<!doctype html><html><head><meta charset="utf-8"><title>CB01 test</title></head><body><div class="post"><a class="wa_chpcs_post_link" href="/film-di-prova-2026/"><img alt="Film di Prova [HD] (2026)" src="https://cb01one.org/poster.jpg" width="200" height="300"></a></div><script>window.__popup=window.open('https://adsco.re/popup','_blank');</script><script src="https://adsco.re/ad.js"></script></body></html>`;
const movie=`<!doctype html><html><head><meta charset="utf-8"><title>Film di Prova [HD] (2026) Streaming - FILM GRATIS by CB01 OFFICIAL</title></head><body class="single"><article>
<div class="sequex-featured-img"><img class="responsive-locandina" alt="Film di Prova [HD] (2026)" src="https://cb01one.org/poster.jpg"></div>
<div class="ignore-css"><table class="cbtable"><tr><td class="tableinside"><a href="https://stayonline.pro/e/MOVIE1/">Mixdrop</a></td></tr></table>
<table class="cbtable"><tr><td>Guarda il Film Completo in HD:</td></tr></table>
<div id="fancy-tabs"><ul><li><a class="link-catch-all">Mixdrop HD</a></li></ul></div>
<div class="tabs-catch-all" data-src="https://stayonline.pro/e/MOVIE1/"><iframe src="https://stayonline.pro/e/MOVIE1/" width="640" height="360"></iframe></div>
</div></article></body></html>`;
const tv=`<!doctype html><html><head><meta charset="utf-8"><title>Serie di Prova – 2×01 – ITA Streaming - SERIE TV GRATIS by CB01 OFFICIAL</title></head><body class="single"><article>
<div class="sequex-featured-img"><img class="responsive-locandina" alt="Serie di Prova – 2×01 – ITA" src="https://cb01one.org/poster.jpg"></div>
<div class="ignore-css"><table class="cbtable"><tr><td>
<div class="sp-wrap sp-wrap-default"><div class="sp-head">STAGIONE 1 - ITA - HD</div><div class="sp-body">
<p>1×01 – <a href="https://stayonline.pro/l/S1E1M/">Maxstream</a> – <a href="https://stayonline.pro/l/S1E1X/">Mixdrop</a></p>
<p>1×02 – <a href="https://stayonline.pro/l/S1E2M/">Maxstream</a> – <a href="https://stayonline.pro/l/S1E2X/">Mixdrop</a></p>
</div></div>
<div class="sp-wrap sp-wrap-default"><div class="sp-head">STAGIONE 2 - ITA - HD</div><div class="sp-body">
<p>2×01 – <a href="https://stayonline.pro/l/S2E1M/">Maxstream</a> – <a href="https://stayonline.pro/l/S2E1X/">Mixdrop</a></p>
<p>2×02 – <a href="https://stayonline.pro/l/S2E2M/">Maxstream</a> – <a href="https://stayonline.pro/l/S2E2X/">Mixdrop</a></p>
</div></div>
</td></tr></table></div></article></body></html>`;

let adRequests=0;
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:1280,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
await context.route('https://**/*',async route=>{
  const request=route.request(),url=new URL(request.url());
  if(url.hostname==='adsco.re'){adRequests++;return route.fulfill({contentType:'application/javascript',body:'window.__adLoaded=true;'});}
  if(url.hostname==='cb01one.org'&&url.pathname==='/')return route.fulfill({contentType:'text/html',body:home});
  if(url.hostname==='cb01one.org'&&url.pathname==='/film-di-prova-2026/')return route.fulfill({contentType:'text/html',body:movie});
  if(url.hostname==='cb01one.org'&&url.pathname==='/serietv/serie-di-prova/')return route.fulfill({contentType:'text/html',body:tv});
  if(url.hostname==='cb01one.org')return route.fulfill({contentType:'image/png',body:png});
  if(url.hostname==='stayonline.pro'&&url.pathname==='/ajax/linkEmbedView.php'){
    const id=new URLSearchParams(request.postData()||'').get('id')||'unknown';
    return route.fulfill({contentType:'application/json',body:JSON.stringify({status:'success',data:{value:`https://m1xdrop.net/e/${id}`}})});
  }
  if(url.hostname==='m1xdrop.net'){
    const id=url.pathname.split('/').pop()||'episode';
    return route.fulfill({contentType:'text/html',body:`<!doctype html><html><body><script>MDCore.wurl="//test.mxcontent.net/${id}.mp4?s=1";MDCore.vfile="${id}.mp4";</script></body></html>`});
  }
  if(url.hostname==='test.mxcontent.net'){
    const range=request.headers()['range'];
    if(range){
      const match=range.match(/bytes=(\d+)-(\d*)/),start=Number(match[1]),end=Math.min(Number(match[2]||mp4.length-1),mp4.length-1);
      return route.fulfill({status:206,headers:{'Content-Type':'video/mp4','Accept-Ranges':'bytes','Content-Range':`bytes ${start}-${end}/${mp4.length}`,'Access-Control-Allow-Origin':'*'},body:mp4.subarray(start,end+1)});
    }
    return route.fulfill({status:200,headers:{'Content-Type':'video/mp4','Accept-Ranges':'bytes','Content-Length':String(mp4.length),'Access-Control-Allow-Origin':'*'},body:mp4});
  }
  return route.fulfill({status:404,body:'not found'});
});

let passed=0;const errors=[];
const check=async(name,fn)=>{await fn();passed++;console.log(`PASS ${name}`);};
const iframeFrame=async(page,dialog)=>{const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});let frame=null;for(let i=0;i<40&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await page.waitForTimeout(100);}return frame;};
const closeDialog=async(page,dialog)=>{await dialog.getByRole('button',{name:'Chiudi il player'}).click();await dialog.waitFor({state:'hidden',timeout:5000});};

try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  const options=await context.newPage();options.on('pageerror',error=>errors.push(error.message));
  await options.goto(`chrome-extension://${id}/settings-popup.html?page=1`);await options.waitForTimeout(500);await options.locator('#tab-functions').click();

  await check('CB01 mode is optional, disabled by default and uses a scoped ruleset',async()=>{
    assert.equal(await options.locator('#cb01-toggle').isChecked(),false);
    assert.equal(await options.locator('#cb01-toggle').isEnabled(),true);
    assert.deepEqual(await worker.evaluate(async()=>await chrome.declarativeNetRequest.getEnabledRulesets()),[]);
    assert.equal(await worker.evaluate(async()=>{const scripts=await chrome.scripting.getRegisteredContentScripts();return scripts.some(script=>script.id.startsWith('cinematch-cb01'));}),false);
  });

  await check('enabling the toggle registers the CB01 scripts and network filters',async()=>{
    await options.locator('#cb01-toggle').click();
    await options.waitForFunction(()=>document.querySelector('#cb01-status').textContent.startsWith('Attivo'),{},{timeout:15000});
    const scripts=await worker.evaluate(async()=>await chrome.scripting.getRegisteredContentScripts());
    assert.ok(scripts.some(script=>script.id==='cinematch-cb01-site'));
    assert.ok(scripts.some(script=>script.id==='cinematch-cb01-main'&&script.world==='MAIN'));
    assert.deepEqual(await worker.evaluate(async()=>await chrome.declarativeNetRequest.getEnabledRulesets()),['cb01_ads']);
    const ad=await worker.evaluate(async()=>await chrome.declarativeNetRequest.testMatchOutcome({url:'https://adsco.re/ad.js',initiator:'https://cb01one.org/',type:'script'}));
    const legitimate=await worker.evaluate(async()=>await chrome.declarativeNetRequest.testMatchOutcome({url:'https://cb01one.org/wp-content/theme.js',initiator:'https://cb01one.org/',type:'script'}));
    const unrelated=await worker.evaluate(async()=>await chrome.declarativeNetRequest.testMatchOutcome({url:'https://adsco.re/ad.js',initiator:'https://example.org/',type:'script'}));
    assert.equal(ad.matchedRules.length,1);
    assert.equal(legitimate.matchedRules.length,0);
    assert.equal(unrelated.matchedRules.length,0);
  });

  await check('a movie card opens the CineMatch overlay, resolves Mixdrop and plays without navigation',async()=>{
    const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
    await page.goto('https://cb01one.org/');
    await page.locator('.cm-cb01-card-link img').waitFor({timeout:10000});
    assert.equal(await page.evaluate(()=>window.__popup),null);
    const before=page.url();
    await page.locator('.cm-cb01-card-link img').click();
    const dialog=page.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const frame=await iframeFrame(page,dialog);assert.ok(frame);
    await frame.waitForFunction(()=>document.querySelector('#movieTitle')?.textContent.includes('Film di Prova'),{},{timeout:15000});
    await frame.waitForFunction(()=>document.querySelector('video')?.currentTime>0.5,{},{timeout:20000});
    assert.equal(page.url(),before);
    assert.match(await frame.locator('#sourcesStatus').innerText(),/CB01 · Mixdrop HD/);
    assert.equal(await frame.locator('#moviePoster').evaluate(image=>image.complete&&image.naturalWidth>0),true);
    assert.equal(adRequests,0);
    await closeDialog(page,dialog);
    assert.equal(await page.evaluate(()=>document.activeElement?.closest('.cm-cb01-card-link')!==null),true);
    await page.close();
  });

  await check('a movie article replaces the ad player and its poster opens the direct player',async()=>{
    const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
    await page.goto('https://cb01one.org/film-di-prova-2026/');
    await page.locator('.cm-cb01-poster-play').waitFor({timeout:10000});
    assert.equal(await page.locator('.cm-cb01-card').count(),1);
    assert.equal(await page.locator('#fancy-tabs').evaluate(node=>node.classList.contains('cm-cb01-original-hidden')),true);
    assert.equal(await page.locator('.tabs-catch-all iframe').getAttribute('src'),'about:blank');
    await page.locator('.cm-cb01-poster-play').click();
    const dialog=page.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const frame=await iframeFrame(page,dialog);assert.ok(frame);
    await frame.waitForFunction(()=>document.querySelector('video')?.currentTime>0.5,{},{timeout:20000});
    assert.match(await frame.locator('#sourcesStatus').innerText(),/CB01 · Mixdrop HD/);
    await closeDialog(page,dialog);
    await page.close();
  });

  await check('a resolved CB01 token opens the standalone player in a tab',async()=>{
    const page=await context.newPage();await page.goto('https://cb01one.org/film-di-prova-2026/');await page.waitForTimeout(300);
    const newPage=context.waitForEvent('page');
    const result=await worker.evaluate(async()=>{
      const tabs=await chrome.tabs.query({url:'https://cb01one.org/*'});
      const tab=tabs.find(item=>item.url.includes('/film-di-prova-2026/'))||tabs[0];
      const executions=await chrome.scripting.executeScript({target:{tabId:tab.id},func:async()=>{
        const resolved=await chrome.runtime.sendMessage({type:'cb01-resolve',articleUrl:location.href});
        if(!resolved?.ok)return resolved;
        return chrome.runtime.sendMessage({type:'cb01-open',token:resolved.token});
      }});
      return executions[0]?.result;
    });
    assert.equal(result?.ok,true);
    const player=await newPage;await player.waitForLoadState('domcontentloaded');
    assert.match(player.url(),/cinema\.html\?directToken=/);
    await player.waitForFunction(()=>document.querySelector('video')?.currentTime>0.5,{},{timeout:20000});
    assert.match(await player.locator('#sourcesStatus').innerText(),/CB01 · Mixdrop HD/);
    await player.close();await page.close();
  });

  await check('a TV poster opens a season/episode picker and episode changes resolve a new stream',async()=>{
    const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
    await page.goto('https://cb01one.org/serietv/serie-di-prova/');
    await page.locator('.cm-cb01-card').waitFor({timeout:10000});
    assert.equal(await page.locator('.cm-cb01-episode-play').count(),4);
    await page.locator('.cm-cb01-poster-play').click();
    const dialog=page.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const frame=await iframeFrame(page,dialog);assert.ok(frame);
    await frame.waitForFunction(()=>document.querySelectorAll('#seasonSelect option').length===2,{},{timeout:15000});
    assert.deepEqual(await frame.locator('#seasonSelect option').allTextContents(),['Stagione 1 · 2 ep.','Stagione 2 · 2 ep.']);
    assert.equal(await frame.locator('#seasonSelect').inputValue(),'2');
    await frame.waitForFunction(()=>document.querySelector('video')?.currentTime>0.5,{},{timeout:20000});
    assert.match(await frame.locator('#sourcesStatus').innerText(),/S2 E1/);
    await frame.getByRole('button',{name:'← Episodi'}).click();
    await frame.locator('#episodeSelect').selectOption('2');
    await frame.waitForFunction(()=>document.querySelector('#sourcesStatus')?.textContent.includes('S2 E2'),{},{timeout:20000});
    assert.match(await frame.locator('#sourcesStatus').innerText(),/S2 E2/);
    await closeDialog(page,dialog);
    await page.close();
  });

  await check('disabling CB01 removes live buttons, registrations and rules',async()=>{
    await options.bringToFront();
    await options.locator('#cb01-toggle').click();
    await options.waitForFunction(()=>document.querySelector('#cb01-status').textContent.startsWith('Disattivo'),{},{timeout:15000});
    assert.deepEqual(await worker.evaluate(async()=>await chrome.declarativeNetRequest.getEnabledRulesets()),[]);
    let remaining=true;
    for(let i=0;i<30&&remaining;i++){
      remaining=await worker.evaluate(async()=>{const scripts=await chrome.scripting.getRegisteredContentScripts();return scripts.some(script=>script.id.startsWith('cinematch-cb01'));});
      if(remaining)await options.waitForTimeout(100);
    }
    assert.equal(remaining,false);
    const page=await context.newPage();await page.goto('https://cb01one.org/');await page.waitForTimeout(700);
    assert.equal(await page.locator('.cm-cb01-card-link').count(),0);
    await page.close();
  });

  await mkdir('test-results',{recursive:true});
  assert.deepEqual(errors,[]);
  console.log(`${passed} CB01 browser checks passed. No real CB01 requests, accounts or ads.`);
}finally{
  await context.close();
  await rm(profile,{recursive:true,force:true});
}
