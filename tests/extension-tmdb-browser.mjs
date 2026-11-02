import assert from 'node:assert/strict';
import {mkdtemp,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
const argument=name=>process.argv.find(arg=>arg.startsWith(name+'='))?.slice(name.length+1);
if(argument('--browser-path'))process.env.PLAYWRIGHT_BROWSERS_PATH=argument('--browser-path');
const {chromium}=await import(argument('--playwright-module')||process.env.CINEMATCH_PLAYWRIGHT_MODULE||'playwright');
const live=process.argv.includes('--live');
const token=live?(process.env.CINEMATCH_TMDB_TOKEN||await new Promise(resolve=>{let value='';process.stdin.setEncoding('utf8');process.stdin.on('data',part=>value+=part);process.stdin.on('end',()=>resolve(value.trim()));})):'eyJtest.payload.signature';
if(!token)throw new Error('Live test requires a TMDB credential.');
const profile=await mkdtemp(join(tmpdir(),'cinematch-tmdb-test-'));
const extension=resolve('dist/CineMatch-Chrome');
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,viewport:{width:900,height:1200},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
await mkdir('test-results',{recursive:true});
let passed=0,requests=0;const searches=[],errors=[];
const check=async(name,fn)=>{await fn();console.log(`PASS ${name}`);passed++;};
const movie={id:5156,title:'Ladri di biciclette',original_title:'Ladri di biciclette',release_date:'1948-11-24',poster_path:'/poster.jpg',credits:{crew:[{id:1,name:'Vittorio De Sica',job:'Director',profile_path:'/director.jpg'}],cast:[{id:2,name:'Lamberto Maggiorani',character:'Antonio',profile_path:'/cast.jpg'},{id:3,name:'Enzo Staiola',character:'Bruno',profile_path:null}]}};
const films=[movie,{...movie,id:10,title:'La terrazza',original_title:'La terrazza',release_date:'1980-02-08',credits:{...movie.credits,crew:[{id:4,name:'Ettore Scola',job:'Director',profile_path:'/scola.jpg'}]}},{...movie,id:19,title:'Metropolis',original_title:'Metropolis',release_date:'1927-01-01'},{...movie,id:20,title:'Amarcord',original_title:'Amarcord',release_date:'1973-01-01'}];
const answer='<h3>1. Ladri di biciclette (1948) — Regia: Vittorio De Sica</h3><p>Un classico del neorealismo da scoprire.</p><h3>8. La terrazza (1980) — Regia: Ettore Scola</h3><ul><li><strong>Titolo originale:</strong> La terrazza</li><li><strong>Attinenza:</strong> Una chiusura discussa del ciclo, come nella schermata di riferimento.</li></ul>';
const fixture=body=>`<!doctype html><html lang="it"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>AI answer fixture</title><body><main>${body}</main></body></html>`;
const privateMessage='PRIVATE_USER_CONVERSATION (1999)';
const providers=[['chatgpt.com',`<div data-message-author-role="user">${privateMessage}</div><div data-message-author-role="assistant">${answer}</div>`],['chat.deepseek.com',`<div data-role="user"><div class="ds-markdown">${privateMessage}</div></div><div class="ds-markdown">${answer}</div>`],['claude.ai',`<div data-testid="assistant-message">${answer}</div>`],['gemini.google.com',`<model-response><message-content>${answer}</message-content></model-response>`]];
for(const [host,body]of providers)await context.route(`https://${host}/**`,r=>r.fulfill({contentType:'text/html',headers:{'Content-Security-Policy':"default-src 'none'; script-src 'none'; style-src 'none'; img-src data:; connect-src 'none'"},body:fixture(body)}));
if(!live){
  await context.route('https://api.themoviedb.org/**',async route=>{
    requests++;const url=new URL(route.request().url());assert.ok(!url.href.includes('PRIVATE_USER_CONVERSATION'));
    let data={images:{}};let status=200;
    if(url.pathname.includes('/search/')){
      const query=url.searchParams.get('query');searches.push(query);
      data={results:query==='missing'?[]:query==='ambiguous'?[{...movie,id:30,title:'Ambiguous'},{...movie,id:31,title:'Ambiguous'}]:query==='unauthorized'?[]:url.pathname.endsWith('person')?[{id:1,name:'Vittorio De Sica',profile_path:'/director.jpg',known_for_department:'Directing'}]:films.filter(m=>m.title===query)};
      if(query==='unauthorized')status=401;
    }else if(url.pathname.includes('/movie/'))data=films.find(m=>m.id===Number(url.pathname.split('/').at(-1)))??movie;
    else if(url.pathname.includes('/person/'))data={id:1,name:'Vittorio De Sica',profile_path:'/director.jpg',known_for_department:'Directing',biography:'Regista e attore italiano.'};
    await route.fulfill({status,contentType:'application/json',body:JSON.stringify(data)});
  });
  await context.route('https://image.tmdb.org/**',route=>{requests++;return route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')});});
}
const cards=page=>page.locator('[data-cinematch-ui="film"]');
const tools=page=>page.locator('[data-cinematch-ui="response"]');
async function allVisibleImagesLoad(page){
  await page.waitForFunction(()=>[...document.querySelectorAll('[data-cinematch-ui="film"]')].every(host=>[...host.shadowRoot.querySelectorAll('img')].every(img=>img.complete&&img.naturalWidth>0)),{},{timeout:20000});
}
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');const id=new URL(worker.url()).host;
  const options=await context.newPage();options.on('pageerror',e=>errors.push(e.message));await options.goto(`chrome-extension://${id}/options.html`);
  const first=await context.newPage();first.on('pageerror',e=>errors.push(e.message));await first.goto('https://chatgpt.com/test');await first.getByRole('button',{name:'Configura immagini TMDB',exact:true}).waitFor();
  await check('missing credential causes no TMDB requests or response parsing cards',async()=>{assert.equal(requests,0);assert.equal(await cards(first).count(),0);});
  await check('saving once starts automatic images in already-open chats',async()=>{
    await options.locator('#credential').evaluate((el,value)=>{el.value=value},token);await options.locator('#save').click();
    await options.waitForFunction(()=>document.getElementById('connection').textContent.startsWith('TMDB è configurato'),{},{timeout:20000});
    assert.equal(await options.locator('#credential').inputValue(),'');assert.ok(!(await options.locator('body').innerText()).includes(token));
    assert.deepEqual(await options.evaluate(()=>chrome.runtime.sendMessage({type:'tmdb-status'})),{ok:true,configured:true,automatic:true});
    await first.getByRole('link',{name:'Apri il film su TMDB',exact:true}).first().waitFor({timeout:20000});
    assert.equal(await first.getByRole('button',{name:'Mostra locandine e cast',exact:true}).count(),0);
    await options.setViewportSize({width:320,height:800});assert.ok(await options.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await options.screenshot({path:'test-results/tmdb-auto-settings-320.png'});
  });
  await check('credential storage remains inaccessible to the content-script context',async()=>{
    const result=await worker.evaluate(async()=>{const [tab]=await chrome.tabs.query({url:'https://chatgpt.com/*'});return chrome.scripting.executeScript({target:{tabId:tab.id},func:async()=>{try{await chrome.storage.local.get('tmdbCredential');return false;}catch{return true;}}});});assert.equal(result[0].result,true);
  });
  for(const [host]of providers){
    const page=host==='chatgpt.com'?first:await context.newPage();if(page!==first)await page.goto(`https://${host}/test`);
    await check(`${host}: automatic inline images appear after numbered headings under strict CSP`,async()=>{
      await page.getByRole('link',{name:'Apri il film su TMDB',exact:true}).nth(1).waitFor({timeout:20000});assert.equal(await cards(page).count(),2);
      assert.ok(await page.getByRole('img',{name:'Foto di Vittorio De Sica',exact:true}).isVisible());assert.ok(await page.getByRole('img',{name:'Foto di Ettore Scola',exact:true}).isVisible());
      assert.ok(await page.getByRole('img',{name:'Foto di Lamberto Maggiorani',exact:true}).first().isVisible());
      assert.equal(await cards(page).evaluateAll(hosts=>hosts.every(el=>el.previousElementSibling?.tagName==='H3')),true);
      assert.ok((await page.locator('h3').filter({hasText:'8. La terrazza (1980)'}).count())>=1);
      await allVisibleImagesLoad(page);
      for(const width of host==='chatgpt.com'?[320,900]:[420]){
        await page.setViewportSize({width,height:1200});assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow ${host}/${width}`);
        await page.screenshot({path:`test-results/tmdb-auto-${live?'live-':''}${host}-${width}.png`,fullPage:true});
      }
    });
    if(page!==first)await page.close();
  }
  if(!live){
    await check('numbering and user messages never enter TMDB queries',async()=>{assert.ok(searches.includes('La terrazza'));assert.ok(!searches.some(s=>s.startsWith('8.')||s.includes('PRIVATE_USER_CONVERSATION')));});
    await check('streamed titles load before the answer finishes, without repeated searches',async()=>{
      await first.setViewportSize({width:900,height:1800});
      await first.evaluate(()=>{
        const root=document.querySelector('[data-message-author-role="assistant"]');const heading=document.createElement('h3');heading.textContent='9. Metropolis (1927)';const prose=document.createElement('p');const text=document.createTextNode('Un film');prose.append(text);root.append(heading,prose);window.streamDone=false;let n=0;
        const timer=setInterval(()=>{text.data+=' con nuove parole';if(++n===60){clearInterval(timer);window.streamDone=true;}},50);
      });
      await first.getByRole('link',{name:'Apri il film su TMDB',exact:true}).nth(2).waitFor({timeout:4000});assert.equal(await first.evaluate(()=>window.streamDone),false);
      await first.waitForFunction(()=>window.streamDone);assert.equal(searches.filter(s=>s==='Metropolis').length,1);
    });
    await check('character-data title updates replace stale cards rather than duplicating them',async()=>{
      await first.evaluate(()=>{const h=[...document.querySelectorAll('h3')].find(el=>el.textContent==='9. Metropolis (1927)');h.firstChild.data='9. Amarcord (1973)';});
      await first.waitForFunction(()=>[...document.querySelectorAll('[data-cinematch-ui="film"]')].some(h=>h.shadowRoot.textContent.includes('Amarcord')));
      await first.waitForTimeout(1100);assert.equal(await cards(first).count(),3);assert.equal(searches.filter(s=>s==='Amarcord').length,1);
    });
    await check('a site re-render restores inline hosts without another TMDB search',async()=>{
      const before=searches.length;await first.evaluate(()=>{const root=document.querySelector('[data-message-author-role="assistant"]');root.querySelectorAll('[data-cinematch-ui]').forEach(el=>el.remove());});
      await first.waitForTimeout(1100);assert.equal(await cards(first).count(),3);assert.equal(searches.length,before);
    });
    await check('automatic mode can be paused and resumed without changing the credential',async()=>{
      await options.locator('#automatic').uncheck();await first.waitForFunction(()=>!document.querySelector('[data-cinematch-ui="film"]'));
      const before=requests;await first.evaluate(()=>{const h=document.createElement('h3');h.textContent='10. Metropolis (1927)';document.querySelector('[data-message-author-role="assistant"]').append(h);});await first.waitForTimeout(1000);assert.equal(requests,before);
      await options.locator('#automatic').check();await first.getByRole('link',{name:'Apri il film su TMDB',exact:true}).nth(3).waitFor({timeout:10000});assert.equal(await cards(first).count(),4);
    });
    await check('manual correction still handles ambiguity and standalone people',async()=>{
      await tools(first).getByText('Cerca un altro film o una persona',{exact:true}).click();const form=tools(first).locator('form').first();
      await form.locator('input').first().fill('ambiguous');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await tools(first).getByRole('button',{name:'Ambiguous · 1948',exact:true}).first().waitFor();assert.equal(await tools(first).getByRole('button',{name:'Ambiguous · 1948',exact:true}).count(),2);
      await tools(first).getByRole('button',{name:'Ambiguous · 1948',exact:true}).first().click();
      await form.locator('select').selectOption('person');await form.locator('input').first().fill('Vittorio De Sica');await form.getByRole('button',{name:'Cerca su TMDB',exact:true}).click();await tools(first).getByRole('link',{name:'Apri la persona su TMDB',exact:true}).waitFor();
    });
    await check('an answer without movie references adds no images and performs no queries',async()=>{
      const before=requests;await first.evaluate(()=>{const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.textContent='Una risposta nuova senza film';document.querySelector('main').append(root);});await first.waitForTimeout(1100);assert.equal(requests,before);
    });
  }
  await check('removing the credential clears automatic cards and prevents future lookups',async()=>{
    await options.locator('#remove').click();await options.waitForFunction(()=>document.getElementById('remove').disabled);await first.waitForFunction(()=>!document.querySelector('[data-cinematch-ui="film"]'));
    assert.deepEqual(await options.evaluate(()=>chrome.runtime.sendMessage({type:'tmdb-status'})),{ok:true,configured:false,automatic:true});
  });
  assert.deepEqual(errors,[]);console.log(`TMDB automatic ${live?'LIVE':'fixture'} browser checks passed: ${passed}`);
}catch(error){console.error(String(error).replaceAll(token,'[redacted]'));process.exitCode=1;}
finally{await context.close();await rm(profile,{recursive:true,force:true});}
