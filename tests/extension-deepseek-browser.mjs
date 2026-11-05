import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const extension=resolve('dist/CineMatch-Chrome');
const profile=await mkdtemp(join(tmpdir(),'cinematch-deepseek-test-'));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:1280,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});

const deepseekFixture=`<!doctype html><html lang="it"><head><meta charset="utf-8"><style>
body{margin:0;min-height:100vh;background:#16181c;color:#ececec;font:14px system-ui}
main{max-width:760px;margin:0 auto;padding:70px 24px}
.composer{position:fixed;left:50%;bottom:30px;transform:translateX(-50%);width:min(680px,85vw);background:#202124;border:1px solid #333;border-radius:18px;padding:12px}
.ds-toggle-row{display:flex;gap:8px;align-items:center;margin-bottom:10px}
.ds-toggle-button{display:inline-flex;align-items:center;gap:6px;height:34px;padding:0 12px;border:1px solid #414247;border-radius:18px;background:#292a2e;color:#e9e9e9;font:13px/1 system-ui;cursor:pointer}
.ds-toggle-button:hover{background:#34353a}
.ds-toggle-button--selected{border-color:#7e9bff;color:#b9c9ff}
.ds-toggle-button__icon{width:14px;height:14px;display:grid;place-items:center}
.ds-toggle-button__icon svg{width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:1.8}
textarea{width:100%;height:56px;background:#242527;color:#fff;border:0;outline:0;resize:none;font:14px system-ui;box-sizing:border-box}
</style></head><body><main><h1>DeepSeek di prova</h1><p>Composer con Pensiero Profondo e Ricerca.</p></main>
<form class="composer" id="composer"><div class="ds-toggle-row" id="toggle-row">
<div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><div class="ds-toggle-button__icon"><svg viewBox="0 0 24 24"><path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/></svg></div><span>Pensiero Profondo</span></div>
<div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><div class="ds-toggle-button__icon"><svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="6"/><path d="M16 16l4 4"/></svg></div><span>Ricerca</span></div>
</div><textarea id="chat-input" placeholder="Scrivi un messaggio…"></textarea><button type="submit">Invia</button></form>
<script>window.submits=0;document.getElementById('composer').addEventListener('submit',event=>{event.preventDefault();window.submits++});</script></body></html>`;
const otherFixture='<!doctype html><html><body><main><h1>Altra chat</h1></main><textarea></textarea></body></html>';
await context.route('https://chat.deepseek.com/**',route=>route.fulfill({contentType:'text/html',body:deepseekFixture}));
await context.route('https://chatgpt.com/**',route=>route.fulfill({contentType:'text/html',body:otherFixture}));
await context.route('https://example.org/**',route=>route.fulfill({contentType:'text/html',body:otherFixture}));

let passed=0;const errors=[];const check=async(name,fn)=>{await fn();passed++;console.log(`PASS ${name}`);};
const compactFrame=async(page)=>{const popover=page.locator('.cm-currents-popover');await popover.waitFor({state:'visible',timeout:10000});const iframe=popover.locator('iframe.cm-currents-frame');await iframe.waitFor({state:'visible'});let frame=null;for(let i=0;i<40&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await page.waitForTimeout(100);}return frame;};

try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  const manager=await context.newPage();manager.on('pageerror',error=>errors.push(error.message));
  await manager.goto(`chrome-extension://${id}/options.html`);await manager.waitForTimeout(400);
  const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
  await page.goto('https://chat.deepseek.com/test',{waitUntil:'load'});await page.waitForTimeout(500);

  await check('production manifest has no side panel and a DeepSeek-only launcher',async()=>{
    const manifest=JSON.parse(await readFile('extensions/cinematch/manifest.json','utf8'));
    assert.equal(manifest.side_panel,undefined);
    assert.equal(manifest.permissions.includes('sidePanel'),false);
    assert.ok(manifest.content_scripts.some(script=>script.js.includes('launcher.js')&&script.matches.length===1&&script.matches[0]==='https://chat.deepseek.com/*'));
    assert.ok(!manifest.content_scripts.find(script=>script.js.includes('references.js')).js.includes('launcher.js'));
    assert.match(await worker.evaluate(async()=>await chrome.action.getPopup({})),/settings-popup\.html$/);
    assert.equal(await worker.evaluate(()=>chrome.action.onClicked.hasListeners()),false);
  });

  await check('the compact currents button is off by default and no floating launcher exists',async()=>{
    assert.equal(await page.locator('.cm-deepseek-currents').count(),0);
    assert.equal(await page.locator('.cm-fab').count(),0);
    assert.equal(await manager.locator('#deepseek-currents-toggle').isChecked(),false);
  });

  await check('enabling the setting inserts one native-looking button after DeepThink and Search',async()=>{
    await manager.locator('#deepseek-currents-toggle').check();
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===1,{},{timeout:10000});
    const info=await page.evaluate(()=>{
      const button=document.querySelector('.cm-deepseek-currents');
      const native=[...document.querySelectorAll('#toggle-row .ds-toggle-button')];
      const second=native[1];
      return {count:document.querySelectorAll('.cm-deepseek-currents').length,text:button?.innerText.trim(),classes:button?.className,role:button?.getAttribute('role'),expanded:button?.getAttribute('aria-expanded'),afterSecond:button?.previousElementSibling===second,style:button?{height:getComputedStyle(button).height,display:getComputedStyle(button).display,borderRadius:getComputedStyle(button).borderRadius}:null,nativeStyle:second?{height:getComputedStyle(second).height,display:getComputedStyle(second).display,borderRadius:getComputedStyle(second).borderRadius}:null};
    });
    assert.equal(info.count,1);assert.equal(info.text,'Correnti');assert.match(info.classes,/ds-toggle-button/);assert.equal(info.role,'button');assert.equal(info.afterSecond,true);assert.deepEqual(info.style,info.nativeStyle);
  });

  await check('the enabled button survives a DeepSeek reload without touching the setting',async()=>{
    await page.reload();await page.waitForTimeout(300);
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===1,{},{timeout:10000});
    assert.equal(await page.locator('.cm-deepseek-currents').count(),1);
    assert.equal(await manager.locator('#deepseek-currents-toggle').isChecked(),true);
  });

  await check('the compact popover lists 74 currents and inserts a generated prompt',async()=>{
    await page.bringToFront();await page.locator('.cm-deepseek-currents').click();
    const frame=await compactFrame(page);assert.ok(frame);
    await frame.locator('#search').waitFor({timeout:20000});
    assert.equal(await frame.locator('.cc-row').count(),74);
    assert.equal(await frame.locator('.cc-tabs [aria-selected="true"]').innerText(),'Correnti');
    await frame.locator('.cc-row').first().click();
    assert.equal(await frame.locator('.cc-chip').count(),1);
    await frame.locator('#generate').click();
    await frame.locator('#prompt-preview').waitFor({state:'visible',timeout:10000});
    const prompt=await frame.locator('#prompt-preview').inputValue();assert.ok(prompt.length>200);
    await frame.locator('#insert').click();
    await page.locator('.cm-currents-popover').waitFor({state:'hidden',timeout:10000});
    await page.waitForFunction(()=>document.querySelector('#chat-input').value.length>200,{},{timeout:10000});
    assert.ok((await page.locator('#chat-input').inputValue()).includes(prompt.trim().slice(0,80)));
    assert.equal(await page.evaluate(()=>window.submits),0);
    await page.waitForTimeout(200);
    assert.equal(await page.evaluate(()=>document.activeElement?.classList.contains('cm-deepseek-currents')),true);
  });

  await check('free request mode generates a compact prompt and can go back',async()=>{
    await page.bringToFront();await page.locator('.cm-deepseek-currents').click();
    const frame=await compactFrame(page);assert.ok(frame);
    await frame.locator('#tab-free').click();
    assert.equal(await frame.locator('.cc-examples').count(),0);
    assert.match(await frame.locator('#free-question').getAttribute('placeholder'),/Scrivi la tua richiesta/);
    await frame.locator('#free-question').fill('Un film italiano degli anni ’70 sull’emigrazione, da vedere in coppia.');
    await frame.locator('#generate').click();
    await frame.locator('#prompt-preview').waitFor({state:'visible'});
    assert.ok((await frame.locator('#prompt-preview').inputValue()).length>100);
    await frame.locator('#back').click();
    assert.equal(await frame.locator('#free-question').isVisible(),true);
    await frame.locator('#close').click();
    await page.locator('.cm-currents-popover').waitFor({state:'hidden',timeout:5000});
  });

  await check('the extension icon popup shows only settings and can toggle DeepSeek',async()=>{
    const popup=await context.newPage();popup.on('pageerror',error=>errors.push(error.message));
    await popup.goto(`chrome-extension://${id}/settings-popup.html`);await popup.waitForTimeout(600);
    assert.equal(await popup.locator('#credential').isVisible(),true);
    assert.equal(await popup.locator('#tab-connection').getAttribute('aria-selected'),'true');
    assert.equal(await popup.locator('#tab-functions').getAttribute('aria-selected'),'false');
    assert.equal(await popup.locator('.cc-row').count(),0);
    assert.equal(await popup.locator('.cc-chip').count(),0);
    assert.equal(await popup.locator('#current-version').innerText(),await worker.evaluate(()=>chrome.runtime.getManifest().version));
    assert.match(await popup.locator('#connection').innerText(),/TMDB/);
    assert.match(await popup.locator('#tmdb-site-status').innerText(),/TMDB|Disattivo|Attivo|Permesso/);
    assert.match(await popup.locator('#cb01-status').innerText(),/Disattivo|Attivo|Permesso/);
    assert.match(await popup.locator('#deepseek-status').innerText(),/Pulsante Correnti/);
    await popup.locator('#tab-functions').click();
    assert.equal(await popup.locator('#functions-view').isVisible(),true);
    assert.equal(await popup.locator('#tmdb-site-toggle').isVisible(),true);
    assert.equal(await popup.locator('#cb01-toggle').isVisible(),true);
    assert.equal(await popup.locator('#deepseek-toggle').isChecked(),true);
    await popup.locator('#deepseek-toggle').uncheck();
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===0,{},{timeout:10000});
    await popup.locator('#deepseek-toggle').check();
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===1,{},{timeout:10000});
    assert.equal(await popup.locator('#open-options').isVisible(),true);
    await popup.close();
  });

  await check('a DeepSeek rerender restores exactly one button',async()=>{
    await page.evaluate(()=>{const row=document.querySelector('#toggle-row');row.innerHTML='<div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><span>Pensiero Profondo</span></div><div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><span>Ricerca</span></div>';});
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===1,{},{timeout:10000});
    assert.equal(await page.locator('.cm-deepseek-currents').count(),1);
  });

  await check('turning the setting off removes the button and closes the popover',async()=>{
    await page.locator('.cm-deepseek-currents').click();
    await page.locator('.cm-currents-popover').waitFor({state:'visible'});
    await manager.bringToFront();await manager.locator('#deepseek-currents-toggle').uncheck();
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===0,{},{timeout:10000});
    await page.waitForTimeout(300);
    assert.equal(await page.locator('.cm-currents-popover').count(),1);
    assert.equal(await page.locator('.cm-currents-popover').evaluate(node=>node.classList.contains('open')),false);
  });

  await check('other chats never receive the currents button or the old floating launcher',async()=>{
    const other=await context.newPage();other.on('pageerror',error=>errors.push(error.message));
    await other.goto('https://chatgpt.com/test');await other.waitForTimeout(500);
    assert.equal(await other.locator('.cm-deepseek-currents').count(),0);assert.equal(await other.locator('.cm-fab').count(),0);
    await other.close();
  });

  await check('the compact popover fits a narrow DeepSeek layout',async()=>{
    await manager.locator('#deepseek-currents-toggle').check();
    await page.setViewportSize({width:360,height:800});await page.bringToFront();await page.waitForTimeout(400);
    await page.locator('.cm-deepseek-currents').click();
    const popover=page.locator('.cm-currents-popover');await popover.waitFor({state:'visible'});
    const result=await popover.evaluate(node=>({width:node.getBoundingClientRect().width,viewport:innerWidth,overflow:document.documentElement.scrollWidth>innerWidth}));
    assert.ok(result.width<=result.viewport);assert.equal(result.overflow,false);
    await page.keyboard.press('Escape');await page.waitForTimeout(300);
  });

  await mkdir('test-results',{recursive:true});
  await manager.screenshot({path:'test-results/deepseek-settings.png'});
  await page.screenshot({path:'test-results/deepseek-currents-compact.png'});
  assert.deepEqual(errors,[]);
  console.log(`${passed} DeepSeek compact checks passed. No real account or AI calls.`);
}finally{
  await context.close();await rm(profile,{recursive:true,force:true});
}
