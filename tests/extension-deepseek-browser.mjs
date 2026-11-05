import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {readFile} from 'node:fs/promises';
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
const waitForFrame=async(page,dialog)=>{const iframe=dialog.locator('iframe.cm-prompt-frame');await iframe.waitFor({state:'visible',timeout:10000});let frame=null;for(let i=0;i<40&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await page.waitForTimeout(150);}return frame;};

try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker'),id=new URL(worker.url()).host;
  const manager=await context.newPage();manager.on('pageerror',error=>errors.push(error.message));
  await manager.goto(`chrome-extension://${id}/options.html`);await manager.waitForTimeout(400);
  const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));
  await page.goto('https://chat.deepseek.com/test',{waitUntil:'load'});
  await page.waitForTimeout(500);

  await check('production manifest has no side panel and a DeepSeek-only launcher',async()=>{
    const manifest=JSON.parse(await readFile('extensions/cinematch/manifest.json','utf8'));
    assert.equal(manifest.side_panel,undefined);
    assert.equal(manifest.permissions.includes('sidePanel'),false);
    assert.ok(manifest.content_scripts.some(script=>script.js.includes('launcher.js')&&script.matches.length===1&&script.matches[0]==='https://chat.deepseek.com/*'));
    assert.ok(!manifest.content_scripts.find(script=>script.js.includes('references.js')).js.includes('launcher.js'));
    assert.equal(await worker.evaluate(()=>chrome.action.onClicked.hasListeners()),true);
  });

  await check('the currents button is off by default and no floating launcher exists',async()=>{
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
      return {
        count:document.querySelectorAll('.cm-deepseek-currents').length,
        text:button?.innerText.trim(),
        classes:button?.className,
        role:button?.getAttribute('role'),
        expanded:button?.getAttribute('aria-expanded'),
        afterSecond:button?.previousElementSibling===second,
        style:button?{height:getComputedStyle(button).height,display:getComputedStyle(button).display,borderRadius:getComputedStyle(button).borderRadius}:null,
        nativeStyle:second?{height:getComputedStyle(second).height,display:getComputedStyle(second).display,borderRadius:getComputedStyle(second).borderRadius}:null
      };
    });
    assert.equal(info.count,1);
    assert.equal(info.text,'Correnti');
    assert.match(info.classes,/ds-toggle-button/);
    assert.equal(info.role,'button');
    assert.equal(info.afterSecond,true);
    assert.deepEqual(info.style,info.nativeStyle);
  });

  await check('the integrated button opens the real CineMatch panel in an overlay and returns focus',async()=>{
    await page.bringToFront();
    await page.locator('.cm-deepseek-currents').click();
    const dialog=page.locator('dialog.cm-prompt-dialog');await dialog.waitFor({state:'visible',timeout:10000});
    const frame=await waitForFrame(page,dialog);assert.ok(frame);
    await frame.locator('#question').waitFor({timeout:20000});
    assert.equal(await frame.locator('#prompt-tab').getAttribute('aria-selected'),'true');
    assert.equal(await frame.locator('#mode-free').getAttribute('aria-checked'),'true');
    await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden',timeout:5000});
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>document.activeElement?.classList.contains('cm-deepseek-currents')),true);
  });

  await check('the panel can generate and insert a free prompt without submitting',async()=>{
    await page.bringToFront();
    await page.locator('.cm-deepseek-currents').click();
    const dialog=page.locator('dialog.cm-prompt-dialog');await dialog.waitFor({state:'visible'});
    const frame=await waitForFrame(page,dialog);assert.ok(frame);
    await frame.locator('#question').fill('Un film italiano degli anni ’70 sull’emigrazione, da vedere in coppia.');
    await frame.locator('#generate').click();
    await frame.waitForFunction(()=>document.querySelector('#prompt-preview').value.length>200);
    const prompt=await frame.locator('#prompt-preview').inputValue();
    await frame.locator('#insert').click();
    await dialog.waitFor({state:'hidden',timeout:10000});
    await page.waitForFunction(()=>document.querySelector('#chat-input').value.length>200);
    assert.ok((await page.locator('#chat-input').inputValue()).includes(prompt.trim().slice(0,80)));
    assert.equal(await page.evaluate(()=>window.submits),0);
  });

  await check('a DeepSeek rerender restores exactly one button',async()=>{
    await page.evaluate(()=>{const row=document.querySelector('#toggle-row');row.innerHTML='<div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><span>Pensiero Profondo</span></div><div class="ds-toggle-button ds-toggle-button--m" aria-pressed="false"><span>Ricerca</span></div>';});
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===1,{},{timeout:10000});
    assert.equal(await page.locator('.cm-deepseek-currents').count(),1);
  });

  await check('turning the setting off removes the button and closes the overlay',async()=>{
    await page.locator('.cm-deepseek-currents').click();
    const dialog=page.locator('dialog.cm-prompt-dialog');await dialog.waitFor({state:'visible'});
    await manager.bringToFront();
    await manager.locator('#deepseek-currents-toggle').uncheck();
    await page.waitForFunction(()=>document.querySelectorAll('.cm-deepseek-currents').length===0,{},{timeout:10000});
    await page.waitForTimeout(300);
    assert.equal(await page.locator('.cm-deepseek-currents').count(),0);
    assert.equal(await dialog.isVisible().catch(()=>false),false);
  });

  await check('other chats never receive the currents button or the old floating launcher',async()=>{
    const other=await context.newPage();other.on('pageerror',error=>errors.push(error.message));
    await other.goto('https://chatgpt.com/test');await other.waitForTimeout(500);
    assert.equal(await other.locator('.cm-deepseek-currents').count(),0);
    assert.equal(await other.locator('.cm-fab').count(),0);
    await other.close();
  });

  await check('the DeepSeek button fits narrow layouts without overflow',async()=>{
    await manager.locator('#deepseek-currents-toggle').check();
    await page.setViewportSize({width:320,height:800});await page.waitForTimeout(400);
    const result=await page.evaluate(()=>({button:document.querySelector('.cm-deepseek-currents')?.getBoundingClientRect().width??0,body:document.documentElement.scrollWidth,viewport:innerWidth}));
    assert.ok(result.button>20);assert.ok(result.body<=result.viewport);
    await page.setViewportSize({width:1280,height:900});
  });

  await mkdir('test-results',{recursive:true});
  await manager.screenshot({path:'test-results/deepseek-settings.png'});
  await page.screenshot({path:'test-results/deepseek-currents.png'});
  assert.deepEqual(errors,[]);
  console.log(`${passed} DeepSeek currents checks passed. No real account or AI calls.`);
}finally{
  await context.close();
  await rm(profile,{recursive:true,force:true});
}
