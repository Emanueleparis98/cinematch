import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {defaultRequest,generatePrompt,matches,providerForUrl,signature,KINDS,MAX_MOVEMENTS} from '../extensions/cinematch/core.js';

const catalog=JSON.parse(readFileSync(new URL('../plugins/cinema-currents/assets/movements.json',import.meta.url),'utf8'));
const current={movements:['neorealismo'],mode:'recommendations'};
const prompt=changes=>generatePrompt({...defaultRequest(),...current,...changes},catalog).prompt;

test('reference has 74 unique, sourced entries with explicit categories',()=>{
  assert.equal(catalog.length,74);assert.equal(new Set(catalog.map(m=>m.id)).size,74);
  for(const m of catalog){assert.ok(KINDS[m.kind]);assert.ok(m.scope_note);assert.ok(m.sources.length);for(const s of m.sources)assert.equal(new URL(s.url).protocol,'https:');}
});
test('new requests start free and without a preselected current',()=>{
  const request=defaultRequest();assert.equal(request.mode,'free');assert.deepEqual(request.movements,[]);assert.equal(request.question,'');
});
test('free requests use no current context, even when selections are still stored',()=>{
  const bare=generatePrompt({...defaultRequest(),question:'Spiega il montaggio in tre paragrafi.'},catalog).prompt;
  const withSelections=generatePrompt({...defaultRequest(),movements:['neorealismo',catalog[0].id],question:'Spiega il montaggio in tre paragrafi.'},catalog);
  assert.equal(withSelections.prompt,bare);assert.doesNotMatch(bare,/CONTESTO DELLE VOCI SELEZIONATE/);assert.equal(withSelections.warnings.length,0);
});
test('current requests require one to four distinct currents',()=>{
  assert.throws(()=>prompt({movements:[]}));
  assert.throws(()=>prompt({movements:catalog.slice(0,MAX_MOVEMENTS+1).map(m=>m.id)}));
  assert.ok(prompt({movements:catalog.slice(0,MAX_MOVEMENTS).map(m=>m.id)}).includes('CONTESTO DELLE VOCI SELEZIONATE'));
});
test('automatic quantity does not impose five results',()=>{
  const text=prompt({});assert.match(text,/senza un limite predefinito di cinque/);assert.doesNotMatch(text,/Quantità richiesta:/);
});
test('exact quantities are retained without a five-film ceiling',()=>{
  for(const count of [1,5,12,100,4294967295])assert.ok(prompt({count}).includes(`Quantità richiesta: ${count} film complessivi`));
});
test('invalid quantities and invalid modes are rejected',()=>{
  for(const count of [0,-1,1.5,NaN,Infinity,4294967296,'5',undefined])assert.throws(()=>prompt({count}));
  for(const mode of ['wrong','constructor','toString','__proto__'])assert.throws(()=>prompt({mode}));
});
test('comparison requires two distinct real entries and shares total count',()=>{
  assert.throws(()=>prompt({mode:'compare',movements:['neorealismo','neorealismo']}));
  assert.throws(()=>prompt({movements:['missing']}));assert.throws(()=>prompt({movements:[]}));
  const text=prompt({mode:'compare',movements:catalog.slice(0,2).map(m=>m.id),count:8});
  assert.match(text,/8 film complessivi/);assert.match(text,/ripartisci la quantità complessiva/);
});
test('free mode follows question rather than imposing film output',()=>{
  assert.throws(()=>prompt({mode:'free',question:'  '}));
  const text=prompt({mode:'free',question:'Spiega il montaggio in tre paragrafi.'});
  assert.match(text,/proponi film soltanto se richiesti/);assert.match(text,/Spiega il montaggio in tre paragrafi/);
});
test('personal criteria and exclusions are preserved, watched aliases excluded',()=>{
  const text=prompt({question:'Spooky, non horror: per Halloween in coppia.',preferences:'Relazione tra cane e gatto',exclusions:'Violenza esplicita',watched:['  Roma città aperta  ','','Ladri di biciclette']});
  for(const word of ['Spooky, non horror','Relazione tra cane e gatto','Violenza esplicita','Roma città aperta; Ladri di biciclette','nemmeno con un titolo alternativo'])assert.ok(text.includes(word));
});
test('prompt includes only selected reference context and distinguishes legacy',()=>{
  const text=prompt({depth:'detailed',relation:'legacy'});assert.match(text,/Caratteristiche:/);assert.match(text,/eredità e influenze successive/);
  assert.ok(text.includes('Neorealismo'));assert.ok(!text.includes(`- ${catalog[0].label} /`));
});
test('non-movement categories receive an explicit warning',()=>{
  const m=catalog.find(m=>m.kind!=='movement');const out=generatePrompt({...defaultRequest(),...current,movements:[m.id]},catalog);
  assert.equal(out.warnings.length,1);assert.ok(out.prompt.includes(KINDS[m.kind]));
});
test('search accepts aliases, author, work and accents',()=>{
  const m=catalog.find(m=>m.id==='neorealismo');assert.ok(matches(m,'neorealismo'));assert.ok(matches(m,m.authors[0]));assert.ok(matches(m,m.works[0].title));assert.ok(!matches(m,'zzzznonesistente'));
});
test('only exact HTTPS provider hosts are recognized',()=>{
  for(const [host,id]of [['chatgpt.com','chatgpt'],['chat.deepseek.com','deepseek'],['claude.ai','claude'],['gemini.google.com','gemini']])assert.equal(providerForUrl(`https://${host}/chat/123`).id,id);
  for(const url of ['http://chatgpt.com','https://chatgpt.com.example.org','https://fakechatgpt.com','https://chatgpt.com@evil.test','file:///chatgpt.com','bad'])assert.equal(providerForUrl(url),null);
});
test('signature changes when criteria change; invalid data never silently changes request',()=>{
  const req=defaultRequest();assert.notEqual(signature(req),signature({...req,count:12}));
  for(const change of [{schema_version:99},{depth:'x'},{relation:'x'},{watched:[2]},{question:null}])assert.throws(()=>prompt(change));
});
test('manifest limits chat controls to four sites and TMDB access to official hosts',()=>{
  const manifest=JSON.parse(readFileSync(new URL('../extensions/cinematch/manifest.json',import.meta.url),'utf8'));
  assert.equal(manifest.manifest_version,3);assert.equal(manifest.host_permissions.length,6);assert.equal(manifest.content_scripts[0].matches.length,4);assert.ok(manifest.content_scripts[0].js.includes('enrich.js'));
  assert.ok(!manifest.permissions.some(p=>['tabs','history','cookies'].includes(p)));assert.ok(!JSON.stringify(manifest).includes('<all_urls>'));
});
