import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const extension=resolve('dist/CineMatch-Chrome');
const profile=await mkdtemp(join(tmpdir(),'cinematch-launcher-test-'));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:1280,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
const fixture=`<!doctype html><html lang="it"><head><meta charset="utf-8"><style>
body{margin:0;min-height:100vh;background:#16181c;color:#ececec;font:14px system-ui}
main{max-width:760px;margin:0 auto;padding:80px 24px}
form{position:fixed;left:50%;bottom:40px;transform:translateX(-50%);width:min(680px,80vw)}
textarea{width:100%;height:56px;background:#242527;color:#fff;border:1px solid #333;border-radius:14px;padding:16px;font:14px system-ui;box-sizing:border-box}
</style></head><body><main><h1>Chat di prova</h1><p>Correnti e prompt, senza cambiare scheda.</p></main><form id="composer"><textarea id="chat-input" placeholder="Scrivi un messaggio…"></textarea><button type="submit">Invia</button></form><script>window.submits=0;document.getElementById('composer').addEventListener('submit',event=>{event.preventDefault();window.submits++});</script></body></html>`;
await context.route('https://chat.deepseek.com/**',route=>route.fulfill({contentType:'text/html',body:fixture}));
await context.route('https://example.org/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Altra pagina</title><p>Nessuna chat qui.</p>'}));
let passed=0;const errors=[];const check=async(name,fn)=>{await fn();passed++;console.log('PASS',name);};
try{
  const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  await page.goto('https://chat.deepseek.com/test',{waitUntil:'load'});
  await page.locator('.cm-fab').waitFor({timeout:15000});
  await check('the floating launcher appears once on a supported chat',async()=>{assert.equal(await page.locator('.cm-fab').count(),1);});
  await page.locator('.cm-fab').click();
  const dialog=page.locator('dialog.cm-prompt-dialog');await dialog.waitFor({state:'visible',timeout:10000});
  const iframe=dialog.locator('iframe.cm-prompt-frame');await iframe.waitFor({state:'visible',timeout:10000});
  let frame=null;for(let i=0;i<30&&!frame;i++){const handle=await iframe.elementHandle();frame=await handle.contentFrame();if(!frame)await page.waitForTimeout(150);}
  assert.ok(frame,'panel frame');await frame.locator('#question').waitFor({timeout:20000});
  await check('the real panel opens on the free composer',async()=>{
    assert.equal(await frame.locator('#prompt-tab').getAttribute('aria-selected'),'true');
    assert.equal(await frame.locator('#mode-free').getAttribute('aria-checked'),'true');
    assert.equal(await frame.locator('#current-fields').isVisible(),false);
    assert.equal(await frame.locator('#examples').isVisible(),true);
  });
  await frame.locator('#explore-tab').click();await frame.locator('.movement-row').first().waitFor({timeout:20000});
  await check('the real panel opens with the 74 currents and no loading overlay',async()=>{assert.equal(await frame.locator('.movement-row').count(),74);assert.equal(await dialog.locator('.cm-prompt-loading').isVisible(),false);});
  await page.waitForTimeout(500);
  await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/launcher-panel-v350.png'});
  await check('the panel detects the current chat and the chat target',async()=>{assert.match(await frame.locator('#chat-state').innerText(),/DeepSeek|pronta/i);});
  await check('the wide explore view keeps the list and opens the detail beside it',async()=>{
    await frame.locator('.movement-name').first().click();
    await frame.locator('#detail').waitFor({state:'visible',timeout:5000});
    await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/launcher-detail-v350.png'});
    assert.equal(await frame.locator('#browser').isVisible(),true);
    assert.equal(await frame.locator('#detail-empty').isVisible(),false);
    assert.ok((await frame.locator('#detail').innerText()).length>100);
    await frame.getByRole('button',{name:'Tutte le voci',exact:true}).click();
    await frame.locator('#detail').waitFor({state:'hidden'});
    assert.equal(await frame.locator('#detail-empty').isVisible(),true);
  });
  await check('a current can be selected and a prompt generated in the two-column view',async()=>{
    await frame.locator('.movement-row input').first().check();
    await frame.locator('#prompt-tab').click();
    await frame.locator('#mode-current').click();
    await frame.locator('#question').fill('Un film intenso da vedere stasera, senza violenza esplicita.');
    await frame.locator('#generate').click();
    await frame.waitForFunction(()=>document.querySelector('#prompt-preview').value.length>200);
    const prompt=await frame.locator('#prompt-preview').inputValue();
    assert.ok(prompt.length>200);assert.match(prompt,/CONTESTO DELLE VOCI SELEZIONATE/);
    assert.equal(await frame.locator('#request-form').isVisible(),true);
    assert.equal(await frame.locator('#preview-section').isVisible(),true);
    assert.equal(await frame.locator('#preview-empty').isVisible(),false);
  });
  await mkdir('test-results',{recursive:true});await page.screenshot({path:'test-results/launcher-prompt-v350.png'});
  const prompt=await frame.locator('#prompt-preview').inputValue();
  await check('the prompt is inserted into the composer, without submitting, and the overlay closes',async()=>{
    await frame.locator('#insert').click();
    await dialog.waitFor({state:'hidden',timeout:10000});
    await page.waitForFunction(()=>document.querySelector('#chat-input').value.length>200);
    assert.ok((await page.locator('#chat-input').inputValue()).includes(prompt.trim().slice(0,80)));
    assert.equal(await page.evaluate(()=>window.submits),0);
  });
  await check('focus returns to the launcher and the panel can reopen and close with Esc',async()=>{
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(()=>document.activeElement?.shadowRoot?.activeElement?.className??null),'cm-fab');
    await page.locator('.cm-fab').click();await dialog.waitFor({state:'visible'});
    await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden',timeout:5000});
  });
  await check('the launcher is absent on unrelated pages',async()=>{
    const other=await context.newPage();await other.goto('https://example.org/',{waitUntil:'load'});await other.waitForTimeout(500);assert.equal(await other.locator('.cm-fab').count(),0);await other.close();
  });
  await check('the panel fits a narrow window without horizontal overflow',async()=>{
    await page.setViewportSize({width:430,height:850});
    await page.locator('.cm-fab').click();await dialog.waitFor({state:'visible'});
    let narrowFrame=null;for(let i=0;i<30&&!narrowFrame;i++){const handle=await iframe.elementHandle();narrowFrame=await handle.contentFrame();if(!narrowFrame)await page.waitForTimeout(150);}
    assert.ok(narrowFrame);
    assert.ok(await narrowFrame.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await page.keyboard.press('Escape');await dialog.waitFor({state:'hidden',timeout:5000});
  });
  assert.deepEqual(errors,[]);
  console.log(`${passed} launcher/browser checks passed. No new tabs, accounts or AI calls.`);
}finally{
  await context.close();
  await rm(profile,{recursive:true,force:true});
}
