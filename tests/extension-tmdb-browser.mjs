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
const context=await chromium.launchPersistentContext(profile,{channel:'chromium',headless:true,reducedMotion:'reduce',viewport:{width:1087,height:1300},args:[`--disable-extensions-except=${extension}`,`--load-extension=${extension}`]});
await mkdir('test-results',{recursive:true});
let passed=0,requests=0,failOnceQuery='';const searches=[],errors=[];
const check=async(name,fn)=>{await fn();console.log(`PASS ${name}`);passed++;};
const movie={id:5156,title:'Ladri di biciclette',original_title:'Ladri di biciclette',release_date:'1948-11-24',poster_path:'/poster.jpg',credits:{crew:[{id:1,name:'Vittorio De Sica',job:'Director',profile_path:'/director.jpg'}],cast:[{id:2,name:'Lamberto Maggiorani',character:'Antonio',profile_path:'/cast.jpg'},{id:3,name:'Enzo Staiola',character:'Bruno',profile_path:null}]}};
const films=[movie,{...movie,id:10,title:'La terrazza',original_title:'La terrazza',release_date:'1980-02-08',credits:{...movie.credits,crew:[{id:4,name:'Ettore Scola',job:'Director',profile_path:'/scola.jpg'}]}},{...movie,id:823219,title:'Flow - Un mondo da salvare',original_title:'Straume',release_date:'2024-01-01',credits:{cast:[],crew:[{id:5,name:'Gints Zilbalodis',job:'Director',profile_path:'/gints.jpg'}]}},{...movie,id:19,title:'Metropolis',original_title:'Metropolis',release_date:'1927-01-01'},{...movie,id:20,title:'Amarcord',original_title:'Amarcord',release_date:'1973-01-01'},{...movie,id:99,title:'Temporaneo',original_title:'Temporaneo'}];
const series={id:136315,name:'The Bear',original_name:'The Bear',first_air_date:'2022-06-23',poster_path:'/bear.jpg',external_ids:{imdb_id:'tt14452776'},created_by:[{id:7,name:'Christopher Storer',profile_path:'/storer.jpg'}],credits:{cast:[{id:8,name:'Jeremy Allen White',character:'Carmy',profile_path:'/white.jpg'}]},alternative_titles:{results:[{title:'The Bear'}]},genres:[{name:'Commedia'}],number_of_seasons:2,number_of_episodes:18,vote_average:8.4,vote_count:1200,seasons:[{season_number:1,name:'Stagione 1',episode_count:8,air_date:'2022-06-23'}]};
films.push({...movie,id:34,title:'Non aprite quella porta',original_title:'The Texas Chain Saw Massacre',release_date:'1974-01-01',credits:{...movie.credits,crew:[{id:6,name:'Tobe Hooper',job:'Director',profile_path:'/hooper.jpg'}]}});
const answer='<h3>1. Ladri di biciclette (1948) — Regia: Vittorio De Sica</h3><p>Un classico del neorealismo da scoprire.</p><h3>🌊 2. Flow - Un mondo da salvare (2024)</h3><p>Un viaggio animato attraverso un mondo sommerso.</p><h3>8. La terrazza (1980) — Regia: Ettore Scola</h3><ul><li><strong>Titolo originale:</strong> La terrazza</li><li><strong>Attinenza:</strong> Una chiusura discussa del ciclo.</li></ul>';
const fixture=body=>`<!doctype html><html lang="it"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>AI answer fixture</title><body><main>${body}</main></body></html>`;
const providers=[['chatgpt.com',`<div data-message-author-role="user">PRIVATE_USER_CONVERSATION (1999)</div><div data-message-author-role="assistant">${answer}</div>`],['chat.deepseek.com',`<div data-role="user"><div class="ds-markdown">PRIVATE_USER_CONVERSATION (1999)</div></div><div class="ds-markdown">${answer}</div>`],['claude.ai',`<div data-testid="assistant-message">${answer}</div>`],['gemini.google.com',`<model-response><message-content>${answer}</message-content></model-response>`]];
for(const [host,body]of providers)await context.route(`https://${host}/**`,r=>r.fulfill({contentType:'text/html',headers:{'Content-Security-Policy':"default-src 'none'; script-src 'none'; style-src 'none'; img-src data:; connect-src 'none'"},body:fixture(body)}));
if(!live){
  await context.route('https://api.themoviedb.org/**',async route=>{
    requests++;const url=new URL(route.request().url());assert.ok(!url.href.includes('PRIVATE_USER_CONVERSATION'));
    let data={images:{}},status=200;
    if(url.pathname.includes('/search/tv')){
      const query=url.searchParams.get('query');searches.push(query);
      data={results:[series].filter(show=>show.name===query||show.original_name===query)};
    }else if(url.pathname.includes('/search/')){
      const query=url.searchParams.get('query');searches.push(query);
      data={results:query==='Ambiguous'?[{...movie,id:30,title:'Ambiguous'},{...movie,id:31,title:'Ambiguous'}]:films.filter(m=>m.title===query||m.original_title===query)};
      if(query===failOnceQuery){failOnceQuery='';status=500;}
    }else if(url.pathname.includes('/tv/'))data=series;
    else if(url.pathname.includes('/movie/'))data=films.find(m=>m.id===Number(url.pathname.split('/').at(-1)))??movie;
    await route.fulfill({status,contentType:'application/json',body:JSON.stringify(data)});
  });
  await context.route('https://image.tmdb.org/**',route=>{requests++;return route.fulfill({contentType:'image/png',body:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=','base64')});});
}
const hosts=page=>page.locator('[data-cinematch-ui="film"]');
const media=page=>hosts(page).locator('.media');
async function settle(page){await page.evaluate(async()=>{await Promise.all([...document.querySelectorAll('[data-cinematch-ui]')].flatMap(host=>[...host.shadowRoot.querySelectorAll('img')]).map(img=>img.decode().catch(()=>{})));await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));});await page.waitForTimeout(250);}
async function theme(page,mode='dark'){
  await page.evaluate(mode=>{const sheet=new CSSStyleSheet();sheet.replaceSync(`body{margin:0;background:${mode==='dark'?'#121212':'#fff'};color:${mode==='dark'?'#ededf0':'#22252b'};font:16px/1.7 Arial,sans-serif}main{max-width:800px;margin:auto;padding:24px}h3{font-size:22px;line-height:1.4;margin:28px 0 16px}p{margin:16px 0}li{margin:10px 0}[data-message-author-role="user"],[data-role="user"]{display:none}@media(max-width:400px){main{padding:16px}h3{font-size:20px}}`);document.adoptedStyleSheets=[sheet];},mode);
}
async function noControls(page){return hosts(page).evaluateAll(nodes=>nodes.every(n=>{const shadow=n.shadowRoot;return !shadow.querySelector('input,select,form,details,h3')&&[...shadow.querySelectorAll('button')].every(button=>button.classList.contains('streams-button'));}));}
try{
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker');const id=new URL(worker.url()).host;
  const options=await context.newPage();options.on('pageerror',e=>errors.push(e.message));await options.goto(`chrome-extension://${id}/settings-popup.html?page=1`);await options.waitForTimeout(500);
  const first=await context.newPage();first.on('pageerror',e=>errors.push(e.message));await first.goto('https://chatgpt.com/test');await theme(first);
  await check('missing credential leaves the AI answer entirely untouched',async()=>{await first.waitForTimeout(800);assert.equal(requests,0);assert.equal(await hosts(first).count(),0);assert.equal(await first.locator('[data-cinematch-ui="response"]').count(),0);});
  await check('saving once illustrates open chats automatically and does not expose the key',async()=>{
    await options.locator('#credential').evaluate((el,value)=>{el.value=value},token);await options.locator('#save').click();
    await options.waitForFunction(()=>document.getElementById('connection').textContent.startsWith('TMDB è configurato'),{},{timeout:20000});
    assert.equal(await options.locator('#credential').inputValue(),'');assert.ok(!(await options.locator('body').innerText()).includes(token));
    await media(first).nth(2).waitFor({timeout:25000});
    assert.equal(await options.locator('#automatic').count(),0);await options.setViewportSize({width:320,height:800});await options.screenshot({path:'test-results/seamless-connection-320.png'});
  });
  await check('credential storage remains inaccessible to the content-script context',async()=>{
    const result=await worker.evaluate(async()=>{const [tab]=await chrome.tabs.query({url:'https://chatgpt.com/*'});return chrome.scripting.executeScript({target:{tabId:tab.id},func:async()=>{try{await chrome.storage.local.get('tmdbCredential');return false;}catch{return true;}}});});assert.equal(result[0].result,true);
  });
  for(const [host]of providers){
    const page=host==='chatgpt.com'?first:await context.newPage();if(page!==first){await page.goto(`https://${host}/test`);await theme(page);}
    await check(`${host}: Flow and numbered films become seamless images under strict CSP`,async()=>{
      await media(page).nth(2).waitFor({timeout:25000});assert.equal(await media(page).count(),3);assert.equal(await noControls(page),true);
      assert.ok(await page.getByRole('img',{name:'Locandina di Flow - Un mondo da salvare',exact:true}).isVisible());assert.ok(await page.getByRole('img',{name:'Foto di Gints Zilbalodis',exact:true}).isVisible());
      assert.equal(await hosts(page).evaluateAll(nodes=>nodes.every(n=>n.previousElementSibling?.tagName==='H3')),true);
      for(const width of host==='chatgpt.com'?[320,1087]:[420]){await page.setViewportSize({width,height:1300});await settle(page);assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert.equal(await hosts(page).locator('.name').evaluateAll(nodes=>nodes.every(node=>[...node.textContent.matchAll(/\S+/gu)].every(word=>{const range=document.createRange();range.setStart(node.firstChild,word.index);range.setEnd(node.firstChild,word.index+word[0].length);return new Set([...range.getClientRects()].map(rect=>Math.round(rect.y))).size<=1;}))),true,'credit words must never fragment across lines');await page.screenshot({path:`test-results/seamless-${live?'live-':''}${host}-${width}-dark.png`,fullPage:true});}
    });
    if(page!==first)await page.close();
  }
  await check('each title poster has one explicit Flussi button outside its link',async()=>{
    const result=await hosts(first).evaluateAll(nodes=>nodes.map(host=>{
      const button=host.shadowRoot.querySelector('.streams-button');
      return {poster:Boolean(host.shadowRoot.querySelector('.poster')),buttons:host.shadowRoot.querySelectorAll('.streams-button').length,nested:Boolean(button?.closest('a')),label:button?.getAttribute('aria-label')??''};
    }));
    assert.ok(result.length>=3);assert.ok(result.filter(item=>item.poster).every(item=>item.buttons===1&&!item.nested&&/^Cerca i flussi per /.test(item.label)));
    const button=hosts(first).first().locator('.streams-button');
    assert.equal(await button.innerText(),'Flussi');
  });
  await check('explicit TV references receive a series poster and the same Flussi action',async()=>{
    const tv=await context.newPage();tv.on('pageerror',e=>errors.push(e.message));await tv.goto('https://chatgpt.com/tv');await theme(tv);
    await tv.evaluate(()=>{const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.innerHTML='<h3>7. The Bear (2022) — Serie TV</h3><p>Una serie intensa e brevissima.</p>';document.querySelector('main').append(root);});
    const tvButton=tv.getByRole('button',{name:'Cerca i flussi per The Bear',exact:true});
    await tvButton.waitFor({timeout:20000});
    assert.equal(await tv.getByRole('img',{name:'Locandina di The Bear',exact:true}).isVisible(),true);
    assert.equal(await tvButton.innerText(),'Flussi');
    assert.ok(searches.includes('The Bear'));
    assert.equal(await noControls(tv),true);
    await tv.close();
  });
  await check('light theme uses the answer typography and never introduces a boxed tool',async()=>{
    await theme(first,'light');assert.equal(await noControls(first),true);
    const style=await media(first).first().evaluate(node=>({background:getComputedStyle(node).backgroundColor,border:getComputedStyle(node).borderWidth,color:getComputedStyle(node).color}));assert.equal(style.background,'rgba(0, 0, 0, 0)');assert.equal(style.border,'0px');assert.equal(style.color,'rgb(34, 37, 43)');
    const light=await context.newPage();await light.goto('https://chatgpt.com/light');await theme(light,'light');await light.setViewportSize({width:1087,height:1300});await media(light).nth(2).waitFor({timeout:25000});await settle(light);await light.screenshot({path:`test-results/seamless-${live?'live-':''}1087-light.png`,fullPage:true});await light.close();
  });
  await check('inline bold title and description in one paragraph keep images before prose',async()=>{
    await first.setViewportSize({width:1140,height:2400});await theme(first,'dark');
    await first.evaluate(()=>{const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.id='inline-answer';root.innerHTML='<p id="inline-paragraph"><strong>3. The Texas Chain Saw Massacre (1974) — Regia: Tobe Hooper</strong><br><em>Non aprite quella porta.</em> Film-cardine e caso di confine: questa descrizione deve restare dopo le immagini.</p>';document.querySelector('main').append(root);});
    const paragraph=first.locator('#inline-paragraph');await paragraph.locator('.media').waitFor({timeout:25000});
    assert.equal(await paragraph.evaluate(p=>{const host=p.querySelector('[data-cinematch-ui]');return host.tagName==='SPAN'&&host.previousElementSibling.tagName==='BR'&&host.nextElementSibling.tagName==='EM';}),true);
    const image=await paragraph.locator('[data-cinematch-ui]').boundingBox(),description=await paragraph.locator('em').boundingBox();assert.ok(image.y+image.height<=description.y+1);
    await settle(first);await paragraph.screenshot({path:`test-results/title-first-${live?'live-':''}1140.png`});
  });
  await check('year and director outside bold text stay above the illustrations',async()=>{
    await first.evaluate(()=>{const p=document.createElement('p');p.id='mixed-title';p.innerHTML='<strong>4. The Texas Chain Saw Massacre</strong> (1974) — Regia: Tobe Hooper<br><em>Descrizione successiva.</em>';const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.id='mixed-answer';root.append(p);document.querySelector('main').append(root);});
    const p=first.locator('#mixed-title');await p.locator('.media').waitFor({timeout:15000});assert.equal(await p.evaluate(el=>el.querySelector('[data-cinematch-ui]').previousElementSibling.tagName),'BR');assert.ok((await p.innerText()).includes('(1974) — Regia: Tobe Hooper'));
  });
  await check('bold title without a line break still precedes inline description',async()=>{
    await first.evaluate(()=>{const p=document.createElement('p');p.id='bold-title';p.innerHTML='<strong>5. The Texas Chain Saw Massacre (1974) — Regia: Tobe Hooper</strong> <em>La descrizione prosegue qui.</em>';const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.append(p);document.querySelector('main').append(root);});
    const p=first.locator('#bold-title');await p.locator('.media').waitFor({timeout:15000});assert.equal(await p.evaluate(el=>el.querySelector('[data-cinematch-ui]').previousElementSibling.tagName),'STRONG');
  });
  await check('a repeated title in descriptive prose never overrides the actual heading',async()=>{
    await first.evaluate(()=>{const root=document.createElement('div');root.dataset.messageAuthorRole='assistant';root.id='repeated-title';root.innerHTML='<h3>6. The Texas Chain Saw Massacre (1974) — Regia: Tobe Hooper</h3><p>The Texas Chain Saw Massacre (1974) — testo descrittivo ripetuto.</p>';document.querySelector('main').append(root);});
    const root=first.locator('#repeated-title');await root.locator('.media').waitFor({timeout:15000});assert.equal(await root.locator('[data-cinematch-ui]').evaluate(el=>el.previousElementSibling.tagName),'H3');await first.locator('#inline-answer').evaluate(el=>el.remove());await first.locator('#mixed-answer').evaluate(el=>el.remove());await first.locator('#bold-title').evaluate(el=>el.parentElement.remove());await root.evaluate(el=>el.remove());
  });
  if(!live){
    await check('decoration and private user content never enter TMDB queries',async()=>{assert.ok(searches.includes('Flow - Un mondo da salvare'));assert.ok(!searches.some(s=>s.startsWith('8.')||s.includes('🌊')||s.includes('PRIVATE_USER_CONVERSATION')));});
    await check('unknown and ambiguous references remain quiet without configuration forms',async()=>{
      await first.setViewportSize({width:1087,height:2200});await first.evaluate(()=>{const root=document.querySelector('[data-message-author-role="assistant"]');for(const title of ['Missing (2024)','Ambiguous (1948)']){const h=document.createElement('h3');h.textContent=title;root.append(h);}});await first.waitForTimeout(1500);
      assert.equal(await media(first).count(),3);assert.equal(await noControls(first),true);assert.ok(!(await first.locator('body').innerText()).includes('Nessuna corrispondenza'));assert.equal(await first.locator('[data-cinematch-ui="response"]').count(),0);
    });
    await check('streamed titles load before the answer finishes without repeated searches',async()=>{
      await first.evaluate(()=>{const root=document.querySelector('[data-message-author-role="assistant"]');const h=document.createElement('h3');h.textContent='9. Metropolis (1927)';const p=document.createElement('p'),text=document.createTextNode('Un film');p.append(text);root.append(h,p);window.streamDone=false;let n=0;const timer=setInterval(()=>{text.data+=' con nuove parole';if(++n===60){clearInterval(timer);window.streamDone=true;}},50);});
      await media(first).nth(3).waitFor({timeout:4000});assert.equal(await first.evaluate(()=>window.streamDone),false);await first.waitForFunction(()=>window.streamDone);assert.equal(searches.filter(s=>s==='Metropolis').length,1);
    });
    await check('character-data updates replace stale images without duplicate media',async()=>{
      await first.evaluate(()=>{const h=[...document.querySelectorAll('h3')].find(n=>n.textContent==='9. Metropolis (1927)');h.firstChild.data='9. Amarcord (1973)';});await first.getByRole('img',{name:'Locandina di Amarcord',exact:true}).waitFor();assert.equal(await media(first).count(),4);assert.equal(searches.filter(s=>s==='Amarcord').length,1);
    });
    await check('site rerenders restore the illustrations without new searches',async()=>{const before=searches.length;await first.evaluate(()=>document.querySelectorAll('[data-cinematch-ui]').forEach(n=>n.remove()));await first.waitForTimeout(1000);assert.equal(await media(first).count(),4);assert.equal(searches.length,before);});
    await check('legacy automatic-off preferences cannot bring back a manual workflow',async()=>{
      await options.evaluate(()=>chrome.storage.local.set({tmdbAutomatic:false}));await first.reload();await theme(first);await media(first).nth(2).waitFor();assert.equal(await noControls(first),true);assert.deepEqual(await options.evaluate(()=>chrome.runtime.sendMessage({type:'tmdb-status'})),{ok:true,configured:true,automatic:true});
    });
    await check('transient failure retries once automatically without displaying software errors',async()=>{
      await first.setViewportSize({width:1087,height:2000});failOnceQuery='Temporaneo';await first.evaluate(()=>{const h=document.createElement('h3');h.textContent='Temporaneo (1948)';document.querySelector('[data-message-author-role="assistant"]').append(h);});await first.getByRole('img',{name:'Locandina di Temporaneo',exact:true}).waitFor({timeout:7000});assert.equal(searches.filter(s=>s==='Temporaneo').length,2);assert.equal(await noControls(first),true);
    });
    await check('answers without film references add no interface or requests',async()=>{const before=requests;await first.evaluate(()=>{const n=document.createElement('div');n.dataset.messageAuthorRole='assistant';n.textContent='Una risposta senza film';document.querySelector('main').append(n);});await first.waitForTimeout(1000);assert.equal(requests,before);});
  }
  await check('removing the credential removes illustrations and prevents future queries',async()=>{await options.locator('#remove').click();await options.waitForFunction(()=>document.getElementById('remove').disabled);await first.waitForFunction(()=>!document.querySelector('[data-cinematch-ui="film"]'));});
  assert.deepEqual(errors,[]);console.log(`Seamless ${live?'LIVE':'fixture'} browser checks passed: ${passed}`);
}catch(error){console.error(String(error).replaceAll(token,'[redacted]'));process.exitCode=1;}
finally{await context.close();await rm(profile,{recursive:true,force:true});}
