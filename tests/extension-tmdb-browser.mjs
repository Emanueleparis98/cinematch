import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
const argument=name=>process.argv.find(arg=>arg.startsWith(name+'='))?.slice(name.length+1);
if(argument('--browser-path'))process.env.PLAYWRIGHT_BROWSERS_PATH=argument('--browser-path');
const {chromium}=await import(argument('--playwright-module')||process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const live=process.argv.includes('--live');
const token=live?(process.env.CINEMATCH_TMDB_TOKEN||await new Promise(resolve=>{let value='';process.stdin.setEncoding('utf8');process.stdin.on('data',part=>value+=part);process.stdin.on('end',()=>resolve(value.trim()));})):'eyJtest.payload.signature';
if(!token)throw new Error('Live test requires a TMDB credential in the environment.');
const profile=await mkdtemp(join(tmpdir(),'cinematch-tmdb-test-'));
const extension=resolve('dist/CineMatch-Chrome');
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:900,height:900},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
await mkdir('test-results',{recursive:true});
let passed=0,requests=0;const searches=[],errors=[];
const check=async(name,fn)=>{await fn();console.log(`PASS ${name}`);passed++;};
const movie={id:5156,title:'Ladri di biciclette',original_title:'Ladri di biciclette',release_date:'1948-11-24',poster_path:'/poster.jpg',credits:{crew:[{id:1,name:'Vittorio De Sica',job:'Director',profile_path:'/director.jpg'}],cast:[{id:2,name:'Lamberto Maggiorani',character:'Antonio',profile_path:'/cast.jpg'},{id:3,name:'Enzo Staiola',character:'Bruno',profile_path:null}]}};
const fixture=body=>`<!doctype html><html lang="it"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>AI editor fixture</title><body><main>${body}</main></body></html>`;
const answer='<h3>Ladri di biciclette (1948) — Regia: Vittorio De Sica</h3><p>Un classico del neorealismo da scoprire.</p>';
const providers=[['chatgpt.com',`<div data-message-author-role="user">PRIVATE_USER_CONVERSATION</div><div data-message-author-role="assistant">${answer}</div>`],['chat.deepseek.com',`<div data-role="user"><div class="ds-markdown">PRIVATE_USER_CONVERSATION</div></div><div class="ds-markdown">${answer}</div>`],['claude.ai',`<div data-testid="assistant-message">${answer}</div>`],['gemini.google.com',`<model-response><message-content>${answer}</message-content></model-response>`]];
for(const [host,body]of providers)await context.route(`https://${host}/**`,r=>r.fulfill({contentType:'text/html',headers:{'Content-Security-Policy':"default-src 'none'; script-src 'none'; style-src 'none'; img-src data:; connect-src 'none'"},body:fixture(body)}));
if(!live){
  await context.route('https://api.themoviedb.org/**',async route=>{
    requests++;const url=new URL(route.request().url());assert.ok(!url.href.includes('PRIVATE_USER_CONVERSATION'));
    let data={images:{}};let status=200;
    if(url.pathname.includes('/search/')){const query=url.searchParams.get('query');searches.push(query);data={results:query==='missing'?[]:query==='ambiguous'?[{...movie,id:10,title:'Ambiguous'},{...movie,id:11,title:'Ambiguous'}]:query==='unauthorized'?[]:url.pathname.endsWith('person')?[{id:1,name:'Vittorio De Sica',profile_path:'/director.jpg',known_for_department:'Directing'}]:[movie]};if(query==='unauthorized')status=401;}
    else if(url.pathname.includes('/movie/'))data=movie;
    else if(url.pathname.includes('/person/'))data={id:1,name:'Vittorio De Sica',profile_path:'/director.jpg',known_for_department:'Directing',biography:'Regista e attore italiano.'};
    await route.fulfill({status,contentType:'application/json',body:JSON.stringify(data)});
  });
  await context.route('https://image.tmdb.org/**',route=>{requests++;return route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')});});
}
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');const id=new URL(worker.url()).host;
  const options=await context.newPage();options.on('pageerror',e=>errors.push(e.message));await options.goto(`chrome-extension://${id}/options.html`);
  const first=await context.newPage();await first.goto('https://chatgpt.com/test');await first.getByRole('button',{name:'Mostra locandine e cast',exact:true}).waitFor();
  await check('response controls make no TMDB calls before a trusted click',async()=>{assert.equal(requests,0);assert.equal(await first.locator('[data-cinematch-ui]').count(),1);await first.getByRole('button',{name:'Mostra locandine e cast',exact:true}).click();await first.getByRole('button',{name:'Configura TMDB',exact:true}).waitFor();assert.equal(requests,0);});
  await check('settings verify and save credential without displaying or returning it',async()=>{
    await options.locator('#credential').evaluate((el,value)=>{el.value=value},token);await options.locator('#save').click();
    await options.waitForFunction(()=>document.getElementById('connection').textContent.startsWith('TMDB è configurato'),{},{timeout:20000});
    assert.equal(await options.locator('#credential').inputValue(),'');assert.ok(!(await options.locator('body').innerText()).includes(token));
    assert.deepEqual(await options.evaluate(()=>chrome.runtime.sendMessage({type:'tmdb-status'})),{ok:true,configured:true});
    await options.setViewportSize({width:320,height:700});assert.ok(await options.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await options.screenshot({path:'test-results/tmdb-settings-320.png'});
  });
  await check('credential storage is inaccessible to the content-script context',async()=>{
    const results=await worker.evaluate(async()=>{const [tab]=await chrome.tabs.query({url:'https://chatgpt.com/*'});return chrome.scripting.executeScript({target:{tabId:tab.id},func:async()=>{try{await chrome.storage.local.get('tmdbCredential');return false;}catch{return true;}}});});assert.equal(results[0].result,true);
  });
  for(const [host]of providers){
    const page=host==='chatgpt.com'?first:await context.newPage();if(page!==first)await page.goto(`https://${host}/test`);
    await check(`${host}: cards, posters, cast and director load under strict page CSP`,async()=>{
      await page.getByRole('button',{name:'Mostra locandine e cast',exact:true}).click();await page.getByRole('link',{name:'Apri il film su TMDB',exact:true}).waitFor({timeout:20000});
      assert.ok(await page.getByRole('img',{name:'Locandina di Ladri di biciclette',exact:true}).evaluate(img=>new Promise(resolve=>{if(img.complete&&img.naturalWidth)resolve(true);else{img.addEventListener('load',()=>resolve(true),{once:true});img.addEventListener('error',()=>resolve(false),{once:true});setTimeout(()=>resolve(false),10000)}})));
      await page.locator('summary').filter({hasText:'Regia'}).click();await page.getByRole('img',{name:'Foto di Vittorio De Sica',exact:true}).waitFor();
      await page.locator('summary').filter({hasText:'Cast ·'}).click();await page.getByRole('img',{name:'Foto di Lamberto Maggiorani',exact:true}).waitFor();
      await page.waitForFunction(()=>{const shadow=document.querySelector('[data-cinematch-ui]')?.shadowRoot;return shadow&&[...shadow.querySelectorAll('img')].every(img=>img.complete&&img.naturalWidth>0)},{},{timeout:15000});
      const style=await page.locator('[data-cinematch-ui]').evaluate(el=>getComputedStyle(el.shadowRoot.querySelector('.gallery')).backgroundColor);assert.equal(style,'rgb(14, 16, 22)');
      for(const width of host==='chatgpt.com'?[320,900]:[420]){await page.setViewportSize({width,height:900});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow ${host}/${width}`);await page.screenshot({path:`test-results/tmdb-${live?'live-':''}${host}-${width}.png`,fullPage:true});}
      assert.equal(await page.locator('[data-cinematch-ui]').count(),1);
    });
    if(page!==first)await page.close();
  }
  if(!live){
    await check('synthetic form submission cannot initiate TMDB queries',async()=>{
      const before=requests;await first.locator('[data-cinematch-ui]').evaluate(host=>{const form=[...host.shadowRoot.querySelectorAll('form')].at(-1);form.querySelector('input').value='Not a user request';form.requestSubmit();});await first.waitForTimeout(150);assert.equal(requests,before);
    });
    await check('ambiguous matches require a real selection and manual people search works',async()=>{
      const gallery=first.locator('[data-cinematch-ui]');const form=gallery.locator('form').last();
      await form.locator('input').first().fill('ambiguous');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await first.getByRole('button',{name:'Ambiguous · 1948',exact:true}).first().waitFor();
      assert.equal(await first.getByRole('button',{name:'Ambiguous · 1948',exact:true}).count(),2);await first.getByRole('button',{name:'Ambiguous · 1948',exact:true}).first().click();
      await form.locator('select').selectOption('person');await form.locator('input').first().fill('Vittorio De Sica');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await first.getByRole('button',{name:'Vittorio De Sica · Directing',exact:true}).click();await first.getByRole('link',{name:'Apri la persona su TMDB',exact:true}).waitFor();
    });
    await check('no results and expired credentials have actionable errors',async()=>{
      const form=first.locator('[data-cinematch-ui]').locator('form').last();await form.locator('select').selectOption('movie');await form.locator('input').first().fill('missing');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await first.getByText('Nessuna corrispondenza. Prova il titolo originale o correggi l’anno.',{exact:true}).waitFor();
      await form.locator('input').first().fill('unauthorized');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await first.getByText('Credenziale TMDB non valida. Controllala nelle impostazioni.',{exact:true}).waitFor();
    });
    await check('new streamed answers gain one control without reading or sending them',async()=>{
      const before=requests;await first.evaluate(()=>{const el=document.createElement('div');el.dataset.messageAuthorRole='assistant';el.textContent='Una risposta nuova senza film';document.querySelector('main').append(el);});await first.locator('[data-cinematch-ui]').nth(1).waitFor();assert.equal(requests,before);assert.ok(!searches.some(s=>s.includes('PRIVATE_USER_CONVERSATION')));
      await first.getByRole('button',{name:'Mostra locandine e cast',exact:true}).click();await first.getByText('Non riconosco titoli con un anno. Cerca qui sotto oppure chiedi alla chat di usare Titolo (anno).',{exact:true}).waitFor();
    });
  }
  await check('removing the credential disables future lookups',async()=>{await options.locator('#remove').click();await options.waitForFunction(()=>document.getElementById('remove').disabled);assert.deepEqual(await options.evaluate(()=>chrome.runtime.sendMessage({type:'tmdb-status'})),{ok:true,configured:false});});
  assert.deepEqual(errors,[]);console.log(`TMDB ${live?'LIVE':'fixture'} browser checks passed: ${passed}`);
}catch(error){console.error(String(error).replaceAll(token,'[redacted]'));process.exitCode=1;}
finally{await context.close();await rm(profile,{recursive:true,force:true});}
