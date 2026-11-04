import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {cp,mkdtemp,readFile,rm,writeFile,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');

const calls=[];
let base='';
const server=createServer((req,res)=>{
  calls.push(req.url);
  res.setHeader('Access-Control-Allow-Origin','*');
  if(req.url==='/manifest.json'){
    res.setHeader('Content-Type','application/json');
    return res.end(JSON.stringify({id:'tv.test',name:'TV Test',version:'1.0.0',resources:['stream'],types:['movie','series'],idPrefixes:['tt']}));
  }
  if(req.url.startsWith('/stream/series/')){
    const episode=decodeURIComponent(req.url).match(/:(\d+):(\d+)\.json$/);
    res.setHeader('Content-Type','application/json');
    return res.end(JSON.stringify({streams:[{name:`TV ${episode?.[1]??'?'}x${episode?.[2]??'?'}`,url:`${base}/pattern.mp4`}]}));
  }
  res.statusCode=404;res.end('not found');
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
base=`http://127.0.0.1:${server.address().port}`;

const profile=await mkdtemp(join(tmpdir(),'cinematch-tv-browser-')),extension=join(profile,'test-extension');
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
const productionManifest=JSON.parse(await readFile('extensions/cinematch/manifest.json','utf8'));
assert.ok(!productionManifest.host_permissions.includes('https://www.themoviedb.org/*'));
assert.ok(productionManifest.optional_host_permissions.includes('https://*/*'));
manifest.host_permissions.push('http://127.0.0.1/*','https://www.themoviedb.org/*');
await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));

const series={id:1399,name:'Il Trono di Spade',original_name:'Game of Thrones',first_air_date:'2011-04-17',poster_path:'/series.jpg',external_ids:{imdb_id:'tt0944947'},created_by:[{id:1,name:'David Benioff',profile_path:'/creator.jpg'}],credits:{cast:[{id:2,name:'Emilia Clarke',character:'Daenerys',profile_path:'/cast.jpg'}]},genres:[{name:'Dramma'}],number_of_seasons:1,number_of_episodes:2,vote_average:8.4,vote_count:1200,overview:'Una serie di prova.',seasons:[{season_number:1,name:'Stagione 1',episode_count:2,air_date:'2011-04-17'}]};
const episodes={episodes:[{episode_number:1,name:'Winter Is Coming',air_date:'2011-04-17',runtime:62,overview:'Pilota'},{episode_number:2,name:'The Kingsroad',air_date:'2011-04-24',runtime:56,overview:'Strada'}]};
const card=(kind,id,title)=>`<div class="relative w-full card"><a class="flex w-full" href="/${kind}/${id}-test?language=it-IT"><img class="poster" alt="${title}" src="https://media.themoviedb.org/t/p/w220_and_h330_face/poster.jpg"></a><div><a href="/${kind}/${id}-test">${title}</a></div></div>`;
const fixture=`<!doctype html><html lang="it"><head><meta charset="utf-8"><style>body{margin:0;background:#0b1220;color:#fff;font:14px system-ui}.card{display:inline-block;width:240px;margin:18px;position:relative}.poster{display:block;width:240px;height:360px;object-fit:cover}.card a{color:#fff}</style></head><body><h1>TMDB test</h1>${card('movie',5156,'Ladri di biciclette')}${card('tv',1399,'Il Trono di Spade')}<div class="card"><a href="/person/1"><img class="poster" alt="Persona" src="https://media.themoviedb.org/t/p/w220_and_h330_face/person.jpg"></a><div>Persona</div></div></body></html>`;
const mainFixture='<!doctype html><html lang="it"><head><meta charset="utf-8"><style>body{margin:0;background:#0b1220;color:#fff;font:14px system-ui}.images.inner{display:block;width:300px;margin:20px}.poster{position:relative;width:300px;height:450px}.poster img{display:block;width:300px;height:450px;object-fit:cover}</style></head><body><section class="images inner"><div class="poster_wrapper"><div class="poster"><div class="image_content"><img class="poster" alt="Wildwood - I segreti del bosco proibito" src="https://media.themoviedb.org/t/p/w300_and_h450_face/poster.jpg"></div></div></div></section></body></html>';

const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:1280,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
await context.route('https://www.themoviedb.org/**',route=>{
  const path=new URL(route.request().url()).pathname;
  return route.fulfill({contentType:'text/html',body:path==='/movie/791523-wildwood'?mainFixture:fixture});
});
await context.route('https://media.themoviedb.org/**',route=>route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')}));
await context.route('https://api.themoviedb.org/**',route=>{
  const url=route.request().url();
  const body=url.includes('/season/')?episodes:series;
  return route.fulfill({contentType:'application/json',body:JSON.stringify(body)});
});
await context.route('https://image.tmdb.org/**',route=>route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')}));

let passed=0;const errors=[];const check=async(name,fn)=>{await fn();passed++;console.log(`PASS ${name}`);};
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  await worker.evaluate(async ({url})=>{
    await chrome.storage.session.set({tmdbCredential:'eyJtest.payload.signature'});
    await chrome.storage.local.set({'streamAddon:tvtest':{id:'tv.test',name:'TV Test',version:'1.0.0',description:'',url,resources:[{types:['movie','series'],prefixes:['tt']}],enabled:true}});
  },{url:`${base}/manifest.json`});
  const tmdb=await context.newPage();tmdb.on('pageerror',error=>errors.push(error.message));await tmdb.goto('https://www.themoviedb.org/movie/now-playing?language=it-IT');
  const options=await context.newPage();options.on('pageerror',error=>errors.push(error.message));await options.goto(`chrome-extension://${id}/options.html`);await options.waitForTimeout(600);
  await check('TMDB site access is optional in production and the TMDB button is off by default',async()=>{
    assert.equal(await options.locator('#tmdb-site-toggle').isChecked(),false);
    assert.equal(await options.locator('#tmdb-site-toggle').isEnabled(),true);
    assert.equal(await tmdb.locator('.cm-tmdb-stream-button').count(),0);
  });
  await check('enabling the optional-looking toggle registers the site button in open TMDB tabs',async()=>{
    await options.locator('#tmdb-site-toggle').click();
    await options.waitForFunction(()=>document.querySelector('#tmdb-site-status').textContent.startsWith('Attivo'),{},{timeout:10000});
    await tmdb.locator('.cm-tmdb-stream-button').first().waitFor({timeout:10000});
    assert.equal(await tmdb.locator('.cm-tmdb-stream-button').count(),2);
    assert.equal(await tmdb.getByRole('button',{name:'Cerca i flussi per Ladri di biciclette',exact:true}).count(),1);
    assert.equal(await tmdb.getByRole('button',{name:'Cerca i flussi per Il Trono di Spade',exact:true}).count(),1);
    assert.equal(await tmdb.getByRole('button',{name:'Cerca i flussi per Persona',exact:true}).count(),0);
    assert.equal(await tmdb.locator('.cm-tmdb-stream-button').evaluateAll(buttons=>buttons.every(button=>!button.closest('a'))),true);
    await mkdir('test-results',{recursive:true});await tmdb.screenshot({path:'test-results/tmdb-site-buttons.png'});
  });
  await check('the TV button opens the series picker with season/episode selection and series addon requests',async()=>{
    await tmdb.getByRole('button',{name:'Cerca i flussi per Il Trono di Spade',exact:true}).click();
    const dialog=tmdb.locator('dialog.cm-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const iframe=dialog.locator('iframe.cm-frame');await iframe.waitFor({state:'visible'});
    let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await tmdb.waitForTimeout(150);}
    assert.ok(frame);
    await frame.getByRole('button',{name:'Riproduci TV 1x1 da TV Test'}).waitFor({timeout:20000});
    assert.equal(await frame.locator('#movieTitle').innerText(),'Il Trono di Spade');
    assert.equal(await frame.locator('#seasonSelect').inputValue(),'1');
    assert.deepEqual(await frame.locator('#episodeSelect option').allTextContents(),['Episodio 1 · Winter Is Coming','Episodio 2 · The Kingsroad']);
    assert.ok(calls.some(path=>decodeURIComponent(path)==='/stream/series/tt0944947:1:1.json'));
    await frame.locator('#episodeSelect').selectOption('2');
    await frame.getByRole('button',{name:'Riproduci TV 1x2 da TV Test'}).waitFor({timeout:20000});
    assert.ok(calls.some(path=>decodeURIComponent(path)==='/stream/series/tt0944947:1:2.json'));
    assert.match(await frame.locator('#sourcesStatus').innerText(),/S1 E2/);
    await mkdir('test-results',{recursive:true});await tmdb.screenshot({path:'test-results/tmdb-site-tv-picker.png'});
    await dialog.locator('button.cm-close').click();await dialog.waitFor({state:'hidden',timeout:5000});
    assert.equal(await tmdb.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'Cerca i flussi per Il Trono di Spade');
  });
  await check('the main movie poster gets a visible button on its container, never on the image',async()=>{
    const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
    await page.goto('https://www.themoviedb.org/movie/791523-wildwood');
    const button=page.locator('.cm-tmdb-stream-button');await button.waitFor({timeout:10000});
    assert.equal(await button.count(),1);
    const geometry=await button.evaluate(element=>({width:element.getBoundingClientRect().width,parent:element.parentElement?.tagName,parentClass:element.parentElement?.className}));
    assert.ok(geometry.width>20);assert.equal(geometry.parent,'DIV');assert.match(geometry.parentClass,/cm-tmdb-poster-host/);
    await page.close();
  });
  await check('disabling the toggle removes live buttons from TMDB and unregisters the site script',async()=>{
    await options.bringToFront();
    await options.locator('#tmdb-site-toggle').click();
    await options.waitForFunction(()=>document.querySelector('#tmdb-site-status').textContent.startsWith('Disattivo'),{},{timeout:10000});
    await tmdb.waitForFunction(()=>!document.querySelector('.cm-tmdb-stream-button'),{},{timeout:10000});
    assert.equal(await tmdb.locator('.cm-tmdb-stream-button').count(),0);
    let registered=true;
    for(let i=0;i<30&&registered;i++){registered=await worker.evaluate(async()=>{const scripts=await chrome.scripting.getRegisteredContentScripts();return scripts.some(script=>script.id==='cinematch-tmdb-site');});if(registered)await options.waitForTimeout(100);}
    assert.equal(registered,false);
  });
  assert.deepEqual(errors,[]);
  console.log(`${passed} TV/TMDB browser checks passed. No real TMDB requests, accounts or AI calls.`);
}finally{
  await context.close();
  await rm(profile,{recursive:true,force:true});
  server.close();
}
