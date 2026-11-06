import assert from 'node:assert/strict';
import {cp,mkdtemp,readFile,writeFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const profile=await mkdtemp(join(tmpdir(),'cinematch-debrid-')),extension=join(profile,'extension');
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
assert.equal(manifest.host_permissions.some(host=>/real-debrid|torbox/.test(host)),false);
manifest.host_permissions.push('https://addon.test/*','https://api.real-debrid.com/*','https://api.torbox.app/*','https://cdn.test/*');await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:420,height:850},args:[`--load-extension=${extension}`,`--disable-extensions-except=${extension}`]});
let holdOld=false,releaseOld=null;
const calls=[];const hash='a'.repeat(40);
const media=await readFile('tests/fixtures/player/pattern.mp4');
await context.route('https://cdn.test/**',route=>{assert.equal(route.request().headers().authorization,undefined);return route.fulfill({contentType:'video/mp4',headers:{'Access-Control-Allow-Origin':'*'},body:media});});
await context.route('https://api.themoviedb.org/**',route=>route.fulfill({contentType:'application/json',body:JSON.stringify({id:5156,title:'Film di prova',imdb_id:'tt0040522',credits:{cast:[],crew:[]}})}));
await context.route('https://addon.test/**',route=>{
 assert.equal(route.request().headers().authorization,undefined);assert.equal(route.request().url().includes('personal-key'),false);
 if(route.request().url().includes('/stream/'))return route.fulfill({contentType:'application/json',body:JSON.stringify({streams:[{name:'Torrent di prova',infoHash:hash}]})});
 return route.fulfill({contentType:'application/json',body:JSON.stringify({id:'fixture',name:'Addon di prova',version:'1',types:['movie'],resources:['stream']})});
});
await context.route('https://api.real-debrid.com/**',async route=>{
 const request=route.request(),path=new URL(request.url()).pathname;calls.push({provider:'rd',path,auth:request.headers().authorization});
 if(holdOld&&path.endsWith('/unrestrict/link')&&request.headers().authorization==='Bearer personal-key-rd')await new Promise(resolve=>{releaseOld=resolve;});
 let body=path.endsWith('/user')?{username:'fixture'}:path.endsWith('addMagnet')?{id:'torrent'}:path.includes('/info/')?{status:'downloaded',files:[{id:1,path:'movie.mkv',selected:1}],links:['https://host.test/link']}:{download:'https://cdn.test/movie.mkv'};
 return route.fulfill({contentType:'application/json',body:JSON.stringify(body)});
});
await context.route('https://api.torbox.app/**',route=>{
 const request=route.request(),u=new URL(request.url());calls.push({provider:'torbox',path:u.pathname,auth:request.headers().authorization});
 let data=u.pathname.endsWith('/me')?{id:1}:u.pathname.endsWith('/createtorrent')?{torrent_id:2}:u.pathname.endsWith('/mylist')?[{id:2,download_finished:true,files:[{id:0,name:'movie.mp4'}]}]:'https://cdn.test/movie.mp4';
 return route.fulfill({contentType:'application/json',body:JSON.stringify({success:true,data})});
});
let count=0;const check=async(name,fn)=>{await fn();console.log('PASS '+name);count++;};
try{
 const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
 const page=await context.newPage();const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto(`chrome-extension://${id}/settings-popup.html?page=1`);await page.locator('#tab-streams').click();
 await check('Flussi tab manages addon manifests without TMDB',async()=>{
  await page.locator('#stream-addon-url').fill('https://addon.test/manifest.json');await page.getByRole('button',{name:'Aggiungi addon',exact:true}).click();
  await page.getByRole('checkbox',{name:'Attiva Addon di prova'}).waitFor();
  await page.getByRole('checkbox',{name:'Attiva Addon di prova'}).uncheck();
  assert.equal(await page.evaluate(async()=>Object.values(await chrome.storage.local.get(null)).find(value=>value?.id==='fixture').enabled),false);
 });
 await check('a global Real-Debrid key is verified and never returned by status',async()=>{
  await page.locator('#debrid-key').fill('personal-key-rd');await page.locator('#debrid-form button[type=submit]').click();
  await page.waitForFunction(()=>document.querySelector('#debrid-status').textContent.startsWith('Real-Debrid collegato'));
  assert.equal(await page.locator('#debrid-key').inputValue(),'');
  const status=await page.evaluate(()=>chrome.runtime.sendMessage({type:'debrid-status'}));assert.equal(status.enabled,true);assert.equal(JSON.stringify(status).includes('personal-key'),false);
 });
 await check('settings cannot invoke torrent resolution',async()=>{
  const result=await page.evaluate(infoHash=>chrome.runtime.sendMessage({type:'debrid-resolve',stream:{infoHash}}),hash);assert.equal(result.ok,false);
 });
 const player=await context.newPage();await player.goto(`chrome-extension://${id}/cinema.html`);
 await check('player resolves hashes with the saved RD key without adding addon credentials',async()=>{
  const result=await player.evaluate(infoHash=>chrome.runtime.sendMessage({type:'debrid-resolve',stream:{infoHash}}),hash);
  assert.equal(result.ok,true,JSON.stringify(result));assert.equal(result.url,'https://cdn.test/movie.mkv');assert.equal(calls.filter(call=>call.provider==='rd').every(call=>call.auth==='Bearer personal-key-rd'),true);
 });
 await check('rotating the key isolates concurrent requests and rejects the old account result',async()=>{
  holdOld=true;const old=player.evaluate(infoHash=>chrome.runtime.sendMessage({type:'debrid-resolve',stream:{infoHash}}),'b'.repeat(40));
  for(let i=0;i<100&&!releaseOld;i++)await new Promise(resolve=>setTimeout(resolve,50));
  assert.ok(releaseOld);
  try{
   await page.locator('#debrid-key').fill('personal-key-rd2');await page.locator('#debrid-form button[type=submit]').click();
   await page.waitForFunction(()=>document.querySelector('#debrid-key').value==='');
   const result=await Promise.race([player.evaluate(infoHash=>chrome.runtime.sendMessage({type:'debrid-resolve',stream:{infoHash}}),'b'.repeat(40)),new Promise((_,reject)=>setTimeout(()=>reject(new Error('New credential reused the old request')),5000))]);
   assert.equal(result.ok,true);
  }finally{holdOld=false;releaseOld();}
  assert.equal((await old).ok,false);
 });
 await check('switching to TorBox uses the new global key and cached-only torrent resolution',async()=>{
  await page.locator('#debrid-provider').selectOption('torbox');await page.locator('#debrid-key').fill('personal-key-torbox');await page.locator('#debrid-form button[type=submit]').click();
  await page.waitForFunction(()=>document.querySelector('#debrid-status').textContent.startsWith('TorBox collegato'));
  const result=await player.evaluate(infoHash=>chrome.runtime.sendMessage({type:'debrid-resolve',stream:{infoHash}}),hash);assert.equal(result.url,'https://cdn.test/movie.mp4');
  assert.equal(calls.filter(call=>call.provider==='torbox').every(call=>call.auth==='Bearer personal-key-torbox'),true);
 });
 await check('an addon torrent reaches the real CineMatch video player using the global key',async()=>{
  await page.getByRole('checkbox',{name:'Attiva Addon di prova'}).check();
  await worker.evaluate(()=>chrome.storage.session.set({tmdbCredential:'eyJtest.payload.signature'}));
  await player.goto(`chrome-extension://${id}/cinema.html?movie=5156`);
  await player.getByRole('button',{name:'Riproduci Torrent di prova da Addon di prova'}).click();
  await player.waitForFunction(()=>document.querySelector('#video').currentTime>.1,{},{timeout:25000});
  assert.match(await player.locator('#video').getAttribute('src'),/cdn.test/);
 });
 await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/stream-settings.png'});
 await check('disconnect removes the saved key and addon removal is shared with the player',async()=>{
  await page.locator('#debrid-remove').click();await page.waitForFunction(()=>document.querySelector('#debrid-status').textContent==='Nessun servizio collegato.');
  assert.equal(await page.evaluate(async()=>(await chrome.storage.local.get('debridKey')).debridKey),undefined);
  await page.getByRole('button',{name:'Rimuovi Addon di prova'}).click();await page.getByRole('checkbox',{name:'Attiva Addon di prova'}).waitFor({state:'detached'});
 });
 assert.deepEqual(errors,[]);console.log(`${count} debrid/browser checks passed with API fixtures; no personal account or real torrent used.`);
}finally{await context.close();await rm(profile,{recursive:true,force:true});}
