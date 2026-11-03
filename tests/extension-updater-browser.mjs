import assert from 'node:assert/strict';
import {readFile,writeFile,mkdtemp,rm,mkdir,cp} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const profile=await mkdtemp(join(tmpdir(),'cinematch-updater-test-'));
const extension=join(profile,'test-extension');
await cp(resolve('dist/CineMatch-Chrome'),extension,{recursive:true});
const released=JSON.parse(await readFile('extensions/cinematch/manifest.json','utf8'));
const latest=released.version;
const current='3.5.9';
const zipPath=resolve(`dist/CineMatch-Chrome-${latest}.zip`);
const zipBytes=await readFile(zipPath);
const sha256=createHash('sha256').update(zipBytes).digest('hex');
const size=zipBytes.length;
const manifest=JSON.parse(await readFile(join(extension,'manifest.json'),'utf8'));
manifest.version=current;
const githubOrigins=['https://github.com/*','https://objects.githubusercontent.com/*','https://release-assets.githubusercontent.com/*'];
manifest.host_permissions=[...new Set([...manifest.host_permissions,...githubOrigins])];
await writeFile(join(extension,'manifest.json'),JSON.stringify(manifest));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:1100,height:1000},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
const latestUrl='https://github.com/Emanueleparis98/cinematch/releases/latest/download/latest.json';
const zipUrl=`https://github.com/Emanueleparis98/cinematch/releases/download/v${latest}/CineMatch-Chrome-${latest}.zip`;
await context.route(latestUrl,route=>route.fulfill({contentType:'application/json',body:JSON.stringify({version:latest,zip:`CineMatch-Chrome-${latest}.zip`,sha256,size,url:zipUrl})}));
await context.route(zipUrl,route=>route.fulfill({contentType:'application/zip',body:zipBytes}));
let passed=0;const errors=[];const check=async(name,fn)=>{await fn();passed++;console.log('PASS',name);};
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');
  const id=new URL(worker.url()).host;
  const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  await page.goto(`chrome-extension://${id}/options.html#updates`,{waitUntil:'load'});
  await page.locator('#updates').waitFor({timeout:10000});
  await page.waitForFunction(version=>document.querySelector('#update-status')?.textContent.includes(version),latest,{timeout:15000});
  await check('the options page shows the installed version and the update channel',async()=>{
    assert.equal(await page.locator('#update-current').innerText(),current);
    assert.match(await page.locator('#update-status').innerText(),new RegExp(latest.replaceAll('.','\\.')));
  });
  await check('a newer release exposes the apply action and the release link',async()=>{
    assert.equal(await page.locator('#update-apply').isVisible(),true);
    assert.equal(await page.locator('#update-release').getAttribute('href'),`https://github.com/Emanueleparis98/cinematch/releases/tag/v${latest}`);
  });
  await check('the folder chooser and check controls are available',async()=>{
    assert.equal(await page.locator('#update-folder').isVisible(),true);
    assert.equal(await page.locator('#update-check').isVisible(),true);
  });
  await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/updater-v360.png'});
  assert.deepEqual(errors,[]);
  await page.evaluate(async version=>{
    await chrome.storage.local.set({updaterSentinel:'persist'});
    globalThis.__cineMatchUpdaterNoReload=true;
    const root=await navigator.storage.getDirectory();
    const folder=await root.getDirectoryHandle('cinematch-test',{create:true});
    const file=await folder.getFileHandle('manifest.json',{create:true});
    const writable=await file.createWritable();
    await writable.write(JSON.stringify({name:'CineMatch — Correnti del cinema',version}));
    await writable.close();
    window.showDirectoryPicker=async()=>folder;
  },current);
  await page.locator('#update-folder').click();
  await page.waitForFunction(()=>document.querySelector('#update-status')?.textContent.includes('Cartella pronta'),{},{timeout:5000});
  await page.locator('#update-apply').click();
  await page.waitForFunction(()=>document.querySelector('#update-status')?.textContent.includes('pronto'),{},{timeout:30000});
  await check('the updater writes the release into the selected folder and keeps storage',async()=>{
    assert.equal(await page.evaluate(()=>chrome.runtime.getManifest().version),current);
    const result=await page.evaluate(async()=>{
      const root=await navigator.storage.getDirectory();
      const folder=await root.getDirectoryHandle('cinematch-test');
      const manifestFile=await folder.getFileHandle('manifest.json');
      const folderManifest=JSON.parse(await (await manifestFile.getFile()).text());
      const launcher=await folder.getFileHandle('launcher.js').then(()=>true).catch(()=>false);
      const updater=await folder.getFileHandle('updater.js').then(()=>true).catch(()=>false);
      const sentinel=(await chrome.storage.local.get('updaterSentinel')).updaterSentinel;
      return {folderVersion:folderManifest.version,launcher,updater,sentinel};
    });
    assert.equal(result.folderVersion,latest);
    assert.equal(result.launcher,true);
    assert.equal(result.updater,true);
    assert.equal(result.sentinel,'persist');
  });
  console.log(`${passed} updater checks passed.`);
}finally{
  await context.close();
  await rm(profile,{recursive:true,force:true});
}
