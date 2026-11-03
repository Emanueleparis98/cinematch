import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {insertIntoComposer} from '../extensions/cinematch/composer.js';
const {chromium}=await import(process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const extension=resolve('dist/CineMatch-Chrome');
const profile=await mkdtemp(join(tmpdir(),'cinematch-extension-test-'));
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:420,height:850},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
let passed=0;const errors=[];
const check=async(name,fn)=>{await fn();passed++;console.log(`PASS ${name}`);};
const fixture=body=>`<!doctype html><html><head><meta charset="utf-8"><style>body{margin:20px}textarea,[contenteditable]{display:block;width:300px;min-height:120px}</style></head><body>${body}<script>window.submits=0;window.enters=0;window.inputs=0;document.addEventListener('submit',e=>{e.preventDefault();window.submits++});document.addEventListener('keydown',e=>{if(e.key==='Enter')window.enters++});document.addEventListener('input',()=>window.inputs++);</script></body></html>`;
const bodies={chatgpt:'<main><form><div id="prompt-textarea" contenteditable="true"><p><br></p></div><button>Send</button></form></main>',deepseek:'<main><form><textarea id="chat-input" placeholder="Message DeepSeek"></textarea><button>Send</button></form></main>',claude:'<main><form><div class="tiptap ProseMirror" role="textbox" contenteditable="true"><p><br></p></div><button>Send</button></form></main>',gemini:'<main><form><rich-textarea><div class="ql-editor" contenteditable="true"><p><br></p></div></rich-textarea><button>Send</button></form></main>'};
const hosts={chatgpt:'chatgpt.com',deepseek:'chat.deepseek.com',claude:'claude.ai',gemini:'gemini.google.com'};
for(const host of Object.values(hosts))await context.route(`https://${host}/**`,route=>route.fulfill({contentType:'text/html',body:fixture(bodies[Object.keys(hosts).find(k=>hosts[k]===host)])}));
await context.route('https://example.org/**',route=>route.fulfill({contentType:'text/html',body:fixture('<textarea></textarea>')}));
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');
  const id=new URL(worker.url()).host;
  await check('browser side panel is configured to open on the extension action',async()=>{
    assert.equal((await worker.evaluate(()=>chrome.sidePanel.getPanelBehavior())).openPanelOnActionClick,true);
    assert.equal((await worker.evaluate(()=>chrome.sidePanel.getOptions({}))).path,'panel.html');
  });
  const panel=await context.newPage();panel.on('pageerror',e=>errors.push(e.message));panel.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await panel.goto(`chrome-extension://${id}/panel.html`);await panel.locator('#question').waitFor();
  await check('the composer opens in free mode with no preselected current',async()=>{
    assert.equal(await panel.locator('#prompt-tab').getAttribute('aria-selected'),'true');
    assert.equal(await panel.locator('#mode-free').getAttribute('aria-checked'),'true');
    assert.equal(await panel.locator('#current-fields').isVisible(),false);
    assert.equal(await panel.locator('#free-note').isVisible(),true);
    assert.equal(await panel.locator('#examples').isVisible(),true);
    assert.equal(await panel.locator('#selected button').count(),0);
    assert.equal(await panel.locator('#selection-count').isHidden(),true);
    assert.equal(await panel.locator('#generate').isDisabled(),true);
    assert.match(await panel.locator('#generate-hint').innerText(),/Scrivi la tua richiesta/);
    await panel.locator('#mode-free').focus();await panel.keyboard.press('ArrowLeft');assert.equal(await panel.locator('#mode-current').getAttribute('aria-checked'),'true');
    await panel.keyboard.press('ArrowRight');assert.equal(await panel.locator('#mode-free').getAttribute('aria-checked'),'true');await panel.locator('#question').focus();
  });
  await check('a free request generates without a current and survives mode switches',async()=>{
    await panel.locator('#question').fill('Spiega il cinema muto comico in pochi passaggi.');
    await panel.locator('#generate').click();
    const free=await panel.locator('#prompt-preview').inputValue();
    assert.match(free,/OBIETTIVO: Richiesta libera/);assert.doesNotMatch(free,/CONTESTO DELLE VOCI SELEZIONATE/);assert.match(free,/Spiega il cinema muto comico/);
    assert.equal(await panel.locator('#copy').isEnabled(),true);assert.equal(await panel.locator('#generate-hint').isHidden(),true);
    await panel.locator('#mode-current').click();
    assert.equal(await panel.locator('#current-fields').isVisible(),true);assert.equal(await panel.locator('#generate').isDisabled(),true);
    assert.equal(await panel.locator('#copy').isEnabled(),false);assert.equal(await panel.locator('#stale').isVisible(),true);
    await panel.locator('#mode-free').click();
    assert.equal(await panel.locator('#copy').isEnabled(),true);assert.equal(await panel.locator('#stale').isHidden(),true);
  });
  await panel.locator('#explore-tab').click();await panel.locator('.movement-row').first().waitFor();
  await check('real MV3 extension loads all local reference entries',async()=>{assert.equal(await panel.locator('.movement-row').count(),74);assert.equal(await panel.locator('#boot-error').isVisible(),false);});
  await check('local fonts load with no remote font service',async()=>{
    await panel.evaluate(async()=>{await document.fonts.load('14px Manrope');await document.fonts.load('800 30px Manrope');await document.fonts.ready;});assert.equal(await panel.evaluate(()=>document.fonts.check('14px Manrope')),true);
  });
  await check('extension stays dark and uses custom checkbox rendering',async()=>{
    assert.equal(await panel.evaluate(()=>getComputedStyle(document.documentElement).colorScheme),'dark');
    assert.equal(await panel.locator('.app-header').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(21, 29, 42)');
    assert.equal(await panel.locator('.movement-row input').first().evaluate(el=>getComputedStyle(el).appearance),'none');
    await panel.locator('.movement-row input').first().check();await mkdir('test-results',{recursive:true});await panel.locator('.movement-row').first().screenshot({path:'test-results/selection-checked.png'});await panel.locator('.movement-row input').first().uncheck();
  });
  await check('custom lists commit selections and reset without an operating-system menu',async()=>{
    await panel.locator('.filter-disclosure summary').click();const trigger=panel.locator('#kind').locator('..').locator('.picker-trigger');
    await trigger.click();await panel.getByRole('option',{name:'Movimento storico',exact:true}).click();assert.equal(await panel.locator('#kind').inputValue(),'movement');assert.equal(await panel.locator('.movement-row').count(),44);assert.ok((await trigger.innerText()).includes('Movimento storico'));
    await panel.locator('#reset').click();assert.equal(await panel.locator('.movement-row').count(),74);assert.ok((await trigger.innerText()).includes('Tutte le categorie'));
  });
  await check('custom lists support keyboard navigation, cancellation and typeahead',async()=>{
    const trigger=panel.locator('#sort').locator('..').locator('.picker-trigger');await trigger.focus();await trigger.press('ArrowDown');await trigger.press('End');await trigger.press('Enter');assert.equal(await panel.locator('#sort').inputValue(),'alphabetic');
    await trigger.press('ArrowDown');await trigger.press('Home');await trigger.press('Escape');assert.equal(await panel.locator('#sort').inputValue(),'alphabetic');assert.equal(await trigger.getAttribute('aria-expanded'),'false');
    await trigger.press('c');await trigger.press('Enter');assert.equal(await panel.locator('#sort').inputValue(),'chronological');await panel.locator('.filter-disclosure summary').click();
  });
  await check('current selection is capped at four and free mode ignores it',async()=>{
    await panel.locator('#prompt-tab').click();await panel.locator('#mode-current').click();await panel.locator('#explore-tab').click();
    const boxes=panel.locator('.movement-row input');
    for(let i=0;i<4;i++)await boxes.nth(i).check();
    assert.equal(await panel.locator('#selection-count').innerText(),'4');assert.equal(await panel.locator('#selection-count').isVisible(),true);
    assert.equal(await boxes.nth(4).isDisabled(),true);
    await panel.locator('#prompt-tab').click();await panel.locator('#mode-free').click();
    assert.equal(await panel.locator('#selection-count').isHidden(),true);assert.match(await panel.locator('#free-note-text').innerText(),/Le 4 correnti selezionate/);
    assert.equal(await panel.locator('#copy').isEnabled(),true);
    await panel.locator('#mode-current').click();assert.equal(await panel.locator('#selection-count').isVisible(),true);
    await panel.locator('#explore-tab').click();
    for(let i=0;i<4;i++)await boxes.nth(i).uncheck();
    assert.equal(await panel.locator('#selection-count').isHidden(),true);assert.equal(await boxes.nth(4).isDisabled(),false);
  });
  await check('search, detail and sources work',async()=>{
    await panel.locator('#search').fill('neorealismo');assert.ok(await panel.locator('.movement-row').count()>=1);
    await panel.getByRole('button',{name:'Neorealismo italiano',exact:true}).click();
    assert.ok(await panel.locator('#detail a').count()>0);await panel.getByRole('button',{name:'Prepara per questa voce',exact:true}).click();
    await mkdir('test-results',{recursive:true});await panel.evaluate(()=>document.querySelector('main').scrollTop=0);await panel.screenshot({path:'test-results/redesign-compose-420.png'});
  });
  await check('prompt generation respects requested quantity and detects stale edits',async()=>{
    await panel.locator('#exact-count').check();await panel.locator('#count').fill('12');await panel.locator('#question').fill('Una visione accessibile in coppia, senza violenza esplicita.');
    await panel.locator('#generate').click();assert.match(await panel.locator('#prompt-preview').inputValue(),/12 film complessivi/);assert.equal(await panel.locator('#copy').isEnabled(),true);
    await panel.locator('#question').fill('Nuovo criterio');assert.equal(await panel.locator('#copy').isEnabled(),false);assert.equal(await panel.locator('#stale').isVisible(),true);await panel.locator('#generate').click();
  });
  await mkdir('test-results',{recursive:true});
  await check('responsive layouts have no horizontal overflow, including conflict controls',async()=>{
    for(const [width,height]of [[320,568],[360,740],[420,850],[900,850]]){
      await panel.setViewportSize({width,height});await panel.evaluate(()=>document.body.classList.toggle('full-page',innerWidth>500));
      assert.ok(await panel.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow at ${width}`);
      await panel.screenshot({path:`test-results/extension-prompt-${width}.png`});
      const box=await panel.locator('#generate').boundingBox();assert.ok(box.y>=0&&box.y+box.height<=height,`generate clipped at ${width}`);
    }
  });
  await check('menu opens above the dock and remains fully visible at 320px',async()=>{
    await panel.setViewportSize({width:320,height:568});await panel.evaluate(()=>document.body.classList.remove('full-page'));
    const trigger=panel.locator('#provider').locator('..').locator('.picker-trigger');await trigger.click();const box=await panel.locator('#cm-menu-provider').boundingBox(),button=await trigger.boundingBox();assert.ok(box.y>=0&&box.y+box.height<=568);assert.ok(box.y<button.y);assert.equal(await panel.locator('#cm-menu-provider .picker-option').count(),4);await panel.screenshot({path:'test-results/dark-menu-320.png'});await trigger.press('Escape');
  });
  const send=message=>panel.evaluate(message=>chrome.runtime.sendMessage(message),message);
  const windowId=await panel.evaluate(async()=>(await chrome.windows.getCurrent()).id);
  for(const provider of Object.keys(hosts)){
    const page=await context.newPage();await page.goto(`https://${hosts[provider]}/test`);
    const tabId=await panel.evaluate(async host=>(await chrome.tabs.query({})).find(t=>t.url?.startsWith(`https://${host}/`)).id,hosts[provider]);
    await panel.bringToFront();await panel.evaluate(async()=>{const own=await chrome.tabs.getCurrent();await chrome.tabs.update(own.id,{active:true});});
    const base={type:'insert-prompt',provider,windowId,targetTabId:tabId,prompt:'CineMatch: prima riga.\nSeconda riga: città e cinema.',mode:'empty'};
    await check(`${provider}: real scripting insertion never submits`,async()=>{
      const result=await send(base);assert.equal(result.ok,true,JSON.stringify({result,editor:await page.locator('[contenteditable],textarea').first().evaluate(el=>({text:el.innerText,value:el.value,html:el.innerHTML,focus:document.hasFocus()}))}));
      const counters=await page.evaluate(()=>({submits,enters,inputs}));assert.equal(counters.submits,0);assert.equal(counters.enters,0);assert.ok(counters.inputs>0);
      assert.equal((await send(base)).code,'already-inserted');
    });
    await check(`${provider}: draft confirmation, changed-draft protection and append`,async()=>{
      const conflict=await send({...base,prompt:'Nuovo prompt.'});assert.equal(conflict.code,'draft-not-empty');assert.ok(!('draft' in conflict));
      const wrong=await send({...base,prompt:'Nuovo prompt.',mode:'replace',draftToken:'wrong'});assert.equal(wrong.code,'draft-not-empty');
      const result=await send({...base,prompt:'Nuovo prompt.',mode:'append',draftToken:conflict.draftToken});assert.equal(result.ok,true,JSON.stringify(result));
      assert.equal((await send({...base,prompt:'Nuovo prompt.',mode:'append',draftToken:conflict.draftToken})).code,'already-inserted');
      const latest=await send({...base,prompt:'Sostituito.'});assert.equal(latest.code,'draft-not-empty');
      assert.equal((await send({...base,prompt:'Sostituito.',mode:'replace',draftToken:latest.draftToken})).ok,true);
    });
    await page.close();
  }
  const page=await context.newPage();await page.goto('https://chat.deepseek.com/test');
  const run=options=>page.evaluate(insertIntoComposer,{provider:'deepseek',prompt:'Prompt di prova',mode:'empty',...options});
  await check('wrong site, hidden, disabled and ambiguous composers are rejected',async()=>{
    assert.equal((await run({provider:'chatgpt'})).code,'wrong-site');
    await page.setContent(fixture('<textarea style="display:none"></textarea>'));assert.equal((await run()).code,'no-editor');
    await page.setContent(fixture('<textarea disabled></textarea>'));assert.equal((await run()).code,'disabled-editor');
    await page.setContent(fixture('<textarea></textarea><textarea></textarea>'));assert.equal((await run()).code,'ambiguous-editor');
    await page.locator('textarea').last().focus();assert.equal((await run()).ok,true);assert.equal(await page.locator('textarea').first().inputValue(),'');
  });
  await check('modal, beforeinput cancellation and site reversion prevent claimed success',async()=>{
    await page.setContent(fixture('<textarea></textarea><div role="dialog" aria-modal="true">Sign in</div>'));assert.equal((await run()).code,'dialog-open');
    await page.setContent(fixture('<textarea></textarea>'));await page.locator('textarea').evaluate(el=>el.addEventListener('beforeinput',e=>e.preventDefault()));assert.equal((await run()).code,'site-rejected');assert.equal(await page.locator('textarea').inputValue(),'');
    await page.setContent(fixture('<textarea></textarea>'));await page.locator('textarea').evaluate(el=>el.addEventListener('input',()=>{el.value=''}));assert.equal((await run()).code,'site-rejected');
  });
  await check('changed draft token cannot overwrite newer text',async()=>{
    await page.setContent(fixture('<textarea>old draft</textarea>'));const result=await run();await page.locator('textarea').fill('newer draft');
    assert.equal((await run({mode:'replace',draftToken:result.draftToken})).code,'draft-not-empty');assert.equal(await page.locator('textarea').inputValue(),'newer draft');
  });
  await check('controlled textarea receives native setter and input event',async()=>{
    await page.setContent(fixture('<textarea></textarea>'));await page.locator('textarea').evaluate(el=>{let tracked='';Object.defineProperty(el,'value',{get(){return HTMLTextAreaElement.prototype.__lookupGetter__('value').call(this)},set(value){tracked=value;HTMLTextAreaElement.prototype.__lookupSetter__('value').call(this,value)}});el.addEventListener('input',()=>{window.nativeChange=el.value!==tracked});});
    assert.equal((await run()).ok,true);assert.equal(await page.evaluate(()=>window.nativeChange),true);
  });
  await check('background refuses unrelated active pages even with remembered target',async()=>{
    const tabId=await panel.evaluate(async()=>(await chrome.tabs.query({})).find(t=>t.url?.startsWith('https://chat.deepseek.com/')).id);
    const other=await context.newPage();await other.goto('https://example.org/');await other.bringToFront();
    assert.equal((await send({type:'insert-prompt',provider:'deepseek',windowId,targetTabId:tabId,prompt:'Test'})).code,'wrong-site');await other.close();
  });
  await page.close();await panel.bringToFront();
  await check('draft session survives panel reopening without persistent local storage',async()=>{
    await panel.locator('#question').fill('Bozza conservata soltanto nella sessione');await panel.waitForTimeout(350);await panel.reload();await panel.locator('#question').waitFor();
    assert.equal(await panel.locator('#question').inputValue(),'Bozza conservata soltanto nella sessione');assert.deepEqual(await panel.evaluate(()=>chrome.storage.local.get(null)),{});
  });
  await check('real panel buttons handle existing drafts without overwriting them',async()=>{
    await panel.setViewportSize({width:320,height:568});await panel.locator('#generate').click();
    const ai=await context.newPage();await ai.goto('https://chat.deepseek.com/test');await ai.locator('textarea').fill('La mia bozza personale.');
    const target=await panel.evaluate(async()=>(await chrome.tabs.query({})).find(t=>t.url?.startsWith('https://chat.deepseek.com/')).id);
    await panel.evaluate(async id=>{await chrome.tabs.update(id,{active:true});},target);await panel.waitForTimeout(100);
    await panel.locator('#insert').click();await panel.locator('#conflict').waitFor();assert.equal(await ai.locator('textarea').inputValue(),'La mia bozza personale.');
    assert.ok((await panel.locator('.app-header').boundingBox()).y>=0,JSON.stringify(await panel.evaluate(()=>({scrollY,root:document.documentElement.scrollHeight,body:document.body.getBoundingClientRect().toJSON(),main:document.querySelector('main').getBoundingClientRect().toJSON(),footer:document.querySelector('footer').getBoundingClientRect().toJSON()}))));
    for(const selector of ['#append','#replace','#cancel-conflict']){const box=await panel.locator(selector).boundingBox();assert.ok(box.y>=0&&box.y+box.height<=568,`${selector} clipped`);}
    await panel.screenshot({path:'test-results/extension-conflict-320.png'});
    await panel.locator('#cancel-conflict').click();assert.equal(await ai.locator('textarea').inputValue(),'La mia bozza personale.');
    await panel.locator('#insert').click();await panel.locator('#append').click();await panel.waitForFunction(()=>document.getElementById('status').textContent.includes('Prompt inserito'));
    assert.ok((await ai.locator('textarea').inputValue()).startsWith('La mia bozza personale.\n\n'));assert.equal(await ai.evaluate(()=>submits),0);await ai.close();
  });
  await check('reference list and details remain usable at narrow width',async()=>{
    await panel.locator('#explore-tab').click();if(await panel.locator('#detail').isVisible())await panel.getByRole('button',{name:'Tutte le voci',exact:true}).click();await panel.locator('#reset').click();await panel.evaluate(()=>document.querySelector('main').scrollTop=0);await panel.screenshot({path:'test-results/extension-reference-320.png'});
    await panel.locator('#search').fill('neorealismo');await panel.getByRole('button',{name:'Neorealismo italiano',exact:true}).click();await panel.screenshot({path:'test-results/extension-detail-320.png'});
    assert.ok(await panel.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  });
  assert.deepEqual(errors,[]);console.log(`Browser checks passed: ${passed}`);
}finally{await context.close();await rm(profile,{recursive:true,force:true});}
