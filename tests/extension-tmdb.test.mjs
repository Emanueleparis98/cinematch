import {test} from 'node:test';
import assert from 'node:assert/strict';
import '../extensions/cinematch/references.js';
import {credential,validateReference,chooseMatch,createTmdbClient,safeImagePath} from '../extensions/cinematch/tmdb.js';
const {extract}=globalThis.CineMatchReferences;
const ref={type:'movie',title:'Ladri di biciclette',variants:['Ladri di biciclette'],year:1948,director:''};
const movie={id:5156,title:'Ladri di biciclette',original_title:'Ladri di biciclette',release_date:'1948-11-24',poster_path:'/poster.jpg',credits:{crew:[{id:1,name:'Vittorio De Sica',job:'Director',profile_path:'/director.jpg'}],cast:[{id:2,name:'Lamberto Maggiorani',character:'Antonio',profile_path:'/cast.jpg'}]}};
const response=(data,status=200)=>new Response(JSON.stringify(data),{status,headers:{'Content-Type':'application/json'}});
const client=fetcher=>createTmdbClient({getCredential:async()=>'a'.repeat(32),fetcher});
test('reference parser accepts headings, bold lists, alternate titles and tables',()=>{
  const refs=extract('1. **Ladri di biciclette** (1948) — Regia: Vittorio De Sica\n# The Thing (1982)\n3. La cosa (The Thing, 1982)\n| 4 | Metropolis | 1927 | Fritz Lang |\n**Nosferatu**\nAnno: 1922');
  assert.equal(refs.length,5);assert.equal(refs[0].title,'Ladri di biciclette');assert.equal(refs[0].director,'Vittorio De Sica');assert.equal(refs[2].variants[1],'The Thing');assert.equal(refs[3].title,'Metropolis');assert.equal(refs[4].year,1922);
});
test('parser deduplicates titles and avoids unsupported prose and huge input',()=>{
  assert.equal(extract('Ladri di biciclette (1948)\nLadri di biciclette (1948)').length,1);assert.deepEqual(extract('Il movimento nasce nel 1948.\nNon inventare titoli senza un anno.'),[]);assert.throws(()=>extract('a'.repeat(200001)));
});
test('explicit person labels are recognized',()=>{assert.equal(extract('Regista: Vittorio De Sica')[0].type,'person');});
test('director metadata does not duplicate a movie as an extra person card',()=>{const refs=extract('Ladri di biciclette (1948)\nRegista: Vittorio De Sica');assert.equal(refs.length,1);assert.equal(refs[0].director,'Vittorio De Sica');});
test('numbered headings from the real La terrazza screenshot strip both heading and ordinal',()=>{const [ref]=extract('# 8. La terrazza (1980) — Regia: Ettore Scola');assert.equal(ref.title,'La terrazza');assert.equal(ref.director,'Ettore Scola');assert.equal(ref.year,1980);});
test('Flow screenshot decoration is removed before searching, subtitle supplies a safe variant',()=>{const [ref]=extract('# 🌊 2. Flow - Un mondo da salvare (2024)');assert.equal(ref.title,'Flow - Un mondo da salvare');assert.ok(ref.variants.includes('Flow'));assert.equal(ref.year,2024);});
test('emoji cleanup preserves real numeric and international film names',()=>{
  const [numeric,japanese]=extract('# 🎥 3. 2001: Odissea nello spazio (1968)\n# 🇯🇵 4. 七人の侍 (1954)');assert.equal(numeric.title,'2001: Odissea nello spazio');assert.equal(japanese.title,'七人の侍');
});
test('a unique exact person name matches automatically; homonyms need a choice',()=>{const ref={type:'person',variants:['Vittorio De Sica']};assert.equal(chooseMatch([{id:1,name:'Vittorio De Sica'}],ref).id,1);assert.equal(chooseMatch([{id:1,name:'Vittorio De Sica'},{id:2,name:'Vittorio De Sica'}],ref),null);});
test('invalid credentials are rejected without echoing them',()=>{
  assert.equal(credential(' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa '),'a'.repeat(32));assert.equal(credential('Bearer eyJtest.payload.signature'),'eyJtest.payload.signature');assert.throws(()=>credential('secret-invalid'),e=>!e.message.includes('secret-invalid'));
});
test('references and image paths are validated',()=>{
  for(const value of [{...ref,year:0},{...ref,title:''},{...ref,type:'url'},{...ref,title:'x'.repeat(151)}])assert.throws(()=>validateReference(value));
  for(const path of ['https://evil.test/a.jpg','/../a.jpg','/a.svg','/a.jpg?x=1'])assert.equal(safeImagePath(path),null);
  assert.equal(safeImagePath('/safe-1_name.jpg'),'/safe-1_name.jpg');
});
test('only unique exact titles with matching years are automatically selected',()=>{
  assert.equal(chooseMatch([movie],ref).id,5156);assert.equal(chooseMatch([movie,{...movie,id:6}],ref),null);assert.equal(chooseMatch([movie],{...ref,year:1949}),null);assert.equal(chooseMatch([movie],{...ref,year:null}),null);assert.equal(chooseMatch([{...movie,title:'Different',original_title:'Different'}],ref),null);
});
test('API authentication, cache and selected movie credit summary work',async()=>{
  const calls=[];const api=client(async(url,opts)=>{calls.push({url,opts});return response(url.includes('/search/')?{results:[movie]}:movie);});
  const result=await api.search(ref);assert.equal(result.matched.id,5156);assert.equal(result.matched.directors[0].photo,'/director.jpg');assert.equal(result.matched.cast[0].name,'Lamberto Maggiorani');
  await api.search(ref);assert.equal(calls.length,2);assert.equal(new URL(calls[0].url).searchParams.get('api_key'),'a'.repeat(32));assert.equal(calls[0].opts.credentials,'omit');api.clear();await api.search(ref);assert.equal(calls.length,4);
});
test('Bearer token is an authorization header, never a URL parameter',async()=>{
  const api=createTmdbClient({getCredential:async()=>'eyJtest.payload.signature',fetcher:async(url,opts)=>{assert.ok(!url.includes('eyJ'));assert.equal(opts.headers.Authorization,'Bearer eyJtest.payload.signature');return response({images:{}});}});await api.verify();
});
test('ambiguous matches and conflicting director require manual choice',async()=>{
  const api=client(async url=>response(url.includes('/search/')?{results:[movie]}:movie));const result=await api.search({...ref,director:'Wrong Director'});assert.equal(result.matched,null);assert.equal(result.candidates.length,1);
});
test('alternate original title is tried after a fuzzy translated-title result',async()=>{
  const calls=[];const api=client(async url=>{const query=new URL(url).searchParams.get('query');calls.push(query);return response(url.includes('/search/')?{results:query==='Wrong translation'?[{...movie,title:'Other film',original_title:'Other film',id:6}]:[movie]}:movie);});
  assert.equal((await api.search({...ref,title:'Wrong translation',variants:['Wrong translation','Ladri di biciclette']})).matched.id,5156);assert.ok(calls.includes('Ladri di biciclette'));
});
test('verified alternative titles resolve translated names without guessing',async()=>{
  const film={...movie,id:823219,title:'Straume',original_title:'Straume',release_date:'2024-01-01',alternative_titles:{titles:[{title:'Flow'}]}};
  const api=client(async url=>response(url.includes('/search/')?{results:[film]}:film));assert.equal((await api.search({type:'movie',title:'Flow',variants:['Flow'],year:2024,director:''})).matched.id,823219);
});
test('errors are recoverable and never expose credentials or URLs',async()=>{
  for(const status of [401,403,404,429,500]){const api=client(async()=>response({},status));await assert.rejects(api.search(ref),e=>!e.message.includes('a'.repeat(32))&&!e.message.includes('https:'));}
  await assert.rejects(client(async()=>{throw new Error('https://secret-url.test')}).search(ref),e=>!e.message.includes('secret-url'));
});
test('missing credential prevents network calls',async()=>{
  let called=false;const api=createTmdbClient({getCredential:async()=>null,fetcher:async()=>{called=true}});await assert.rejects(api.search(ref));assert.equal(called,false);
});
test('image proxy accepts only small raster images from TMDB',async()=>{
  const api=client(async url=>{assert.equal(url,'https://image.tmdb.org/t/p/w185/poster.jpg');return new Response(new Uint8Array([1,2,3]),{headers:{'content-type':'image/jpeg'}});});assert.equal(await api.image('/poster.jpg','w185'),'data:image/jpeg;base64,AQID');await assert.rejects(api.image('https://evil.test/image.jpg','w185'));await assert.rejects(api.image('/poster.jpg','original'));
  await assert.rejects(client(async()=>new Response('<svg/>',{headers:{'content-type':'image/svg+xml'}})).image('/poster.jpg','w185'));
});
test('concurrency is bounded and identical pending requests are shared',async()=>{
  let active=0,max=0,calls=0;const api=client(async()=>{calls++;max=Math.max(max,++active);await new Promise(r=>setTimeout(r,5));active--;return response({results:[]});});
  await Promise.all(Array.from({length:12},(_,i)=>api.search({...ref,title:`Film ${i%6}`,variants:[`Film ${i%6}`]})));assert.equal(calls,6);assert.ok(max<=3);
});
