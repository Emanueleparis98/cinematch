/* Automatically enrich AI answers. Only movie/person queries leave the browser. */
(() => {
  if(globalThis.cinematchEnrichmentLoaded)return;
  globalThis.cinematchEnrichmentLoaded=true;
  const selectors={
    'chatgpt.com':['[data-message-author-role="assistant"]'],
    'chat.deepseek.com':['.ds-markdown','[data-role="assistant"]'],
    'claude.ai':['[data-testid="assistant-message"]','.font-claude-response'],
    'gemini.google.com':['model-response message-content','model-response .model-response-text','model-response']
  }[location.hostname];
  if(!selectors)return;
  const {extract,normalize}=globalThis.CineMatchReferences;
  const configuration={ready:false,configured:false,automatic:true};
  const rootStates=new Map();
  const make=(tag,text,cls)=>{const el=document.createElement(tag);if(text!==undefined)el.textContent=text;if(cls)el.className=cls;return el;};
  const act=(label,handler,cls='')=>{const b=make('button',label,cls);b.type='button';b.addEventListener('click',e=>{if(e.isTrusted)Promise.resolve().then(()=>handler(e)).catch(error=>{const message=make('p',error.message,'status error');message.setAttribute('role','alert');b.after(message);});});return b;};
  const send=async message=>{try{const result=await chrome.runtime.sendMessage(message);if(!result?.ok)throw new Error(result?.error??'Ricarica la chat per riattivare CineMatch.');return result;}catch(error){throw new Error(error.message?.includes('context invalidated')?'Estensione aggiornata: ricarica la chat.':error.message??'CineMatch non disponibile: ricarica la chat.');}};
  const css=`
    :host{display:block;clear:both;margin:16px 0;color-scheme:dark;font:14px/1.5 Arial,sans-serif;color:#f4eee2;text-align:left;--muted:#b5b1a7;--line:#3b414d}
    *{box-sizing:border-box}[hidden]{display:none!important}button,input,select{font:inherit;color:inherit;background:#11141b;border:1px solid var(--line);border-radius:4px;padding:8px 10px;min-width:0}button{cursor:pointer}button:hover{border-color:#a7a399;background:#242936}button:disabled{opacity:.5;cursor:default}:focus-visible{outline:2px solid #ff794f;outline-offset:3px}input{width:100%;caret-color:#ff794f}a{color:#ff946f;text-underline-offset:3px;overflow-wrap:anywhere}::selection{color:#f4eee2;background:#753a28}
    .toolbar{display:flex;gap:8px;flex-wrap:wrap;align-items:center}.toolbar .main{color:#ffd2c2;border-color:#80523e;background:#201813;font-weight:600}.quiet{font-size:12px;background:transparent}.gallery{margin-top:12px;padding:16px;background:#0e1016;border:1px solid var(--line);border-radius:5px;overflow-wrap:anywhere}.heading{font:600 21px/1.25 Georgia,serif;margin:0 0 6px}.note,.status{font-size:12px;color:var(--muted);margin:8px 0}.status.error{color:#ff9aa4}.list{display:grid;gap:20px;margin-top:16px}.film{border-top:1px solid var(--line);padding-top:16px;min-width:0}.film-head{display:grid;grid-template-columns:88px minmax(0,1fr);gap:14px;align-items:start}.poster{width:88px;aspect-ratio:2/3;background:#20252f;object-fit:cover;border-radius:3px;display:block}.missing{display:flex;align-items:center;justify-content:center;text-align:center;color:var(--muted);font-size:11px;padding:5px}.film h3{font:600 19px/1.3 Georgia,serif;margin:0 0 6px}.meta{font-size:12px;color:var(--muted);margin:5px 0}.film p{margin:6px 0;line-height:1.55}.film a{font-size:12px}.people{display:grid;grid-template-columns:repeat(auto-fill,minmax(74px,1fr));gap:14px;margin:12px 0}.person{min-width:0}.person img,.person .missing{width:64px;aspect-ratio:2/3;object-fit:cover;background:#20252f;border-radius:3px;display:block;margin-bottom:5px}.person strong{display:block;font-size:12px;line-height:1.4;font-weight:600}.person small{display:block;color:var(--muted);font-size:11px;line-height:1.4;margin-top:3px}details{margin-top:12px}summary{cursor:pointer;font-size:13px;font-weight:600}summary:focus-visible{outline:2px solid #ff794f}.candidates{display:flex;flex-direction:column;gap:8px;margin:12px 0}.candidates button{text-align:left}.search-form{border-top:1px solid var(--line);padding-top:14px;margin-top:16px}.search-form label{display:block;color:var(--muted);font-size:12px;margin-bottom:5px}.search-fields{display:grid;grid-template-columns:1fr 80px;gap:8px}.search-form select{width:100%;margin-bottom:8px}.search-form button{margin-top:8px}.credits{border-top:1px solid var(--line);padding-top:12px;margin-top:20px;font-size:11px;color:var(--muted)}.more{margin-top:14px}.credits p{margin:6px 0}.type-label{font-weight:600}.manual-title{margin:18px 0 6px;font-size:14px}.original{color:var(--muted);font-size:12px}.film .retry{margin-top:8px}@media(max-width:380px){.gallery{padding:12px}.film-head{grid-template-columns:72px minmax(0,1fr);gap:10px}.poster{width:72px}.film h3{font-size:18px}}
  `;
  const sheet=new CSSStyleSheet();sheet.replaceSync(css+`
    .quiet{background:#11141b;color:#f4eee2}.search-fields{align-items:end}.search-fields>input{height:38px}
    :host{margin:12px 0 20px}.inline-card{padding:14px;background:#0e1016;border:1px solid var(--line);border-radius:5px;overflow-wrap:anywhere}.inline-card>.film{padding:0;border:0}.film-head{grid-template-columns:88px minmax(0,1fr)}.film h3{font-size:17px}.director{display:flex;align-items:center;gap:8px;margin:8px 0}.director img,.director .missing{width:30px;height:40px;object-fit:cover;border-radius:2px;flex:none}.director a{font-size:12px}.cast-preview{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px;margin-top:10px;max-width:270px}.cast-preview a{text-decoration:none;min-width:0}.cast-preview img,.cast-preview .missing{width:44px;height:60px;object-fit:cover;border-radius:2px;display:block}.cast-preview small{display:block;margin-top:4px;font-size:11px;color:#f4eee2;line-height:1.3}.response-tools{font-size:11px;color:var(--muted);padding:8px;background:#0e1016;border:1px solid var(--line);border-radius:4px}.response-tools .credits{border:0;padding:0;margin:6px 0}.response-tools details{margin-top:8px}.response-tools summary{font-size:12px}.manual-results{display:grid;gap:12px}.inline-card .candidates{max-height:260px;overflow:auto}.inline-card .note{margin:4px 0}.director-label{font-size:11px;color:var(--muted)}.director .missing,.cast-preview .missing{font-size:0;padding:0;overflow:hidden;display:flex;align-items:center;justify-content:center}.missing svg{width:18px;height:18px;stroke:#b5b1a7;fill:none;stroke-width:1.5}@media(max-width:380px){.inline-card{padding:10px}.film-head{grid-template-columns:64px minmax(0,1fr);gap:10px}.poster{width:64px}.cast-preview img,.cast-preview .missing{width:40px;height:54px}.cast-preview{gap:6px}}
  `);
  const lazyImages=new IntersectionObserver(entries=>{for(const entry of entries){if(!entry.isIntersecting)continue;const img=entry.target;lazyImages.unobserve(img);send({type:'tmdb-image',path:img.dataset.path,size:img.dataset.size}).then(r=>{if(img.isConnected)img.src=r.data;}).catch(()=>fallbackImage(img));}},{rootMargin:'120px'});
  function missingImage(alt,cls){
    const box=make('span','Foto non disponibile',`${cls} missing`);box.setAttribute('role','img');box.setAttribute('aria-label',alt);box.title='Foto non disponibile';
    const ns='http://www.w3.org/2000/svg',svg=document.createElementNS(ns,'svg'),circle=document.createElementNS(ns,'circle'),path=document.createElementNS(ns,'path');svg.setAttribute('viewBox','0 0 24 24');svg.setAttribute('aria-hidden','true');circle.setAttribute('cx','12');circle.setAttribute('cy','8');circle.setAttribute('r','3');path.setAttribute('d','M5 21v-2a7 7 0 0 1 14 0v2');svg.append(circle,path);box.append(svg);return box;
  }
  function fallbackImage(img){if(img.isConnected)img.replaceWith(missingImage(img.alt,img.className));}
  function image(path,alt,cls='poster',size='w185'){
    if(!path)return missingImage(alt,cls);
    const img=make('img',undefined,cls);img.alt=alt;img.dataset.path=path;img.dataset.size=size;img.addEventListener('error',()=>fallbackImage(img),{once:true});lazyImages.observe(img);return img;
  }
  function link(label,type,id){const a=make('a',label);a.href=`https://www.themoviedb.org/${type}/${id}`;a.target='_blank';a.rel='noopener noreferrer';return a;}
  function people(list){const grid=make('div',undefined,'people');for(const p of list){const box=make('div',undefined,'person');box.append(image(p.photo,`Foto di ${p.name}`,'portrait','w92'),make('strong',p.name),make('small',p.role),link('Scheda TMDB','person',p.id));grid.append(box);}return grid;}
  function renderItem(card,item){
    card.replaceChildren();const head=make('div',undefined,'film-head');
    head.append(image(item.type==='movie'?item.poster:item.photo,item.type==='movie'?`Locandina di ${item.title}`:`Foto di ${item.name}`));
    const body=make('div');body.append(make('h3',item.title??item.name));
    if(item.type==='movie'){
      if(item.originalTitle!==item.title)body.append(make('p',item.originalTitle,'original'));
      body.append(make('p',`${item.year||'Anno non disponibile'}${item.directors.length?' · '+item.directors.map(p=>p.name).join(', '):''}`,'meta'),link('Apri il film su TMDB','movie',item.id));
    }else{body.append(make('p',[item.role,item.birthday,item.place].filter(Boolean).join(' · '),'meta'),link('Apri la persona su TMDB','person',item.id));}
    if(item.type==='movie'){
      for(const p of item.directors){const director=make('div',undefined,'director');director.append(image(p.photo,`Foto di ${p.name}`,'portrait','w92'),link(`Regia: ${p.name}`,'person',p.id));body.append(director);}
      const preview=make('div',undefined,'cast-preview');
      for(const p of item.cast.slice(0,3)){const a=link('','person',p.id);a.append(image(p.photo,`Foto di ${p.name}`,'portrait','w92'),make('small',p.name));a.title=p.role;preview.append(a);}body.append(preview);
    }
    head.append(body);card.append(head);
    if(item.type==='movie'){
      const cast=make('details');cast.append(make('summary',`Cast completo · ${item.cast.length} persone`));let count=0;const grid=make('div');
      const load=()=>{grid.append(people(item.cast.slice(count,count+12)));count+=12;more.hidden=count>=item.cast.length;};
      const more=act('Mostra altre persone',load,'more');cast.append(grid,more);cast.addEventListener('toggle',()=>{if(cast.open&&!count)load();});
      if(item.cast.length)card.append(cast);else card.append(make('p','Cast non disponibile su TMDB.','note'));
    }else if(item.biography)card.append(make('p',item.biography));
    const correction=make('details');correction.append(make('summary','Cambia abbinamento'));
    searchForm(correction,{type:item.type,title:item.title??item.name,year:item.type==='movie'&&item.year?Number(item.year):null},ref=>loadReference(card,ref));card.append(correction);
  }
  function searchForm(container,initial,onSearch){
    const form=make('form',undefined,'search-form');const label=make('label','Titolo del film o nome della persona');
    const type=make('select');type.setAttribute('aria-label','Tipo di ricerca');for(const [value,text]of [['movie','Film'],['person','Persona']]){const o=make('option',text);o.value=value;type.append(o);}type.value=initial?.type??'movie';
    const fields=make('div',undefined,'search-fields'),input=make('input'),year=make('input');input.value=initial?.title??'';input.required=true;input.maxLength=150;input.placeholder='Titolo o nome';label.append(input);year.type='number';year.placeholder='Anno';year.min='1800';year.max='2100';year.setAttribute('aria-label','Anno del film, facoltativo');year.value=initial?.year??'';
    fields.append(label,year);type.addEventListener('change',()=>{year.disabled=type.value==='person';});
    const submit=make('button','Cerca su TMDB');submit.type='submit';form.append(type,fields,submit);
    let armed=false;submit.addEventListener('click',event=>{armed=event.isTrusted;});form.addEventListener('keydown',event=>{if(event.key==='Enter')armed=event.isTrusted;});
    form.addEventListener('submit',event=>{event.preventDefault();const trusted=armed;armed=false;if(!event.isTrusted||!trusted||!form.reportValidity())return;onSearch({type:type.value,title:input.value.trim(),variants:[input.value.trim()],year:type.value==='movie'&&year.value?Number(year.value):null,director:''});});
    container.append(form);return form;
  }
  async function loadReference(card,reference,isCurrent=()=>card.isConnected){
    card.replaceChildren(make('h3',reference.title),make('p','Cerco la scheda su TMDB…','status'));card.setAttribute('aria-busy','true');
    try{
      const result=await send({type:'tmdb-search',reference});if(!isCurrent())return;
      if(result.matched){renderItem(card,result.matched);return;}
      card.replaceChildren(make('h3',reference.title),make('p',result.candidates.length?'Scegli la scheda corretta: l’abbinamento non è univoco.':'Nessuna corrispondenza. Prova il titolo originale o correggi l’anno.','note'));
      const candidates=make('div',undefined,'candidates');for(const candidate of result.candidates){const b=act(`${candidate.title}${candidate.year?' · '+candidate.year:''}`,async()=>{
        b.disabled=true;try{const response=await send({type:'tmdb-details',kind:reference.type,id:candidate.id});if(card.isConnected)renderItem(card,response.item);}catch(error){const message=make('p',error.message,'status error');candidates.append(message);b.disabled=false;}
      });candidates.append(b);}card.append(candidates);
      if(result.hasMore)card.append(make('p','Altri risultati potrebbero esistere: restringi il titolo o l’anno qui sotto.','note'));
      searchForm(card,reference,ref=>loadReference(card,ref));
    }catch(error){if(isCurrent())card.replaceChildren(make('h3',reference.title),make('p',error.message,'status error'),act('Riprova',()=>loadReference(card,reference),'retry'));}
    finally{card.removeAttribute('aria-busy');}
  }
  function responseText(root){
    const walk=node=>{
      if(node.nodeType===Node.TEXT_NODE)return node.textContent;
      if(node.nodeType!==Node.ELEMENT_NODE||node.matches('[data-cinematch-ui],button,form,textarea,input,script,style,[aria-hidden="true"]'))return '';
      if(node.tagName==='BR')return '\n';
      const text=[...node.childNodes].map(walk).join('');
      if(node.tagName==='STRONG'||node.tagName==='B')return `**${text}**`;
      if(/^H[1-6]$/.test(node.tagName))return `\n# ${text}\n`;
      if(node.tagName==='TD'||node.tagName==='TH')return ` | ${text}`;
      return /^(P|DIV|LI|TR|UL|OL|BLOCKQUOTE)$/.test(node.tagName)?`\n${text}\n`:text;
    };
    return walk(root);
  }
  const referenceKey=ref=>`${ref.type}:${normalize(ref.title)}:${ref.year??''}`;
  function createHost(kind){
    const host=make('div');host.setAttribute('data-cinematch-ui',kind);
    const shadow=host.attachShadow({mode:'open'});shadow.adoptedStyleSheets=[sheet];
    return {host,shadow};
  }
  function clearEntry(entry){clearTimeout(entry.timer);entry.version++;cardObserver.unobserve(entry.host);for(const img of entry.host.shadowRoot.querySelectorAll('img'))lazyImages.unobserve(img);entry.host.remove();}
  function clearState(state){for(const entry of state.entries.values())clearEntry(entry);state.entries.clear();state.tools.host.remove();}
  function toolsFor(state){
    const tools=state.tools.shadow;tools.replaceChildren();
    const box=make('div',undefined,'response-tools');
    if(!configuration.configured){box.append(act('Configura immagini TMDB',()=>send({type:'tmdb-options'}),'quiet'));}
    else{
      const text=make('span',configuration.automatic?'Immagini automatiche · ':'Immagini automatiche disattivate · ');
      const source=make('a','TMDB');source.href='https://www.themoviedb.org';source.target='_blank';source.rel='noopener noreferrer';text.append(source);box.append(text,document.createTextNode(' '),act('Impostazioni',()=>send({type:'tmdb-options'}),'quiet'));
      const manual=make('details');manual.append(make('summary','Cerca un altro film o una persona'));
      const results=make('div',undefined,'manual-results');searchForm(manual,null,ref=>{const card=make('article',undefined,'film inline-card');results.append(card);loadReference(card,ref);});manual.append(results);box.append(manual);
    }
    tools.append(box);
  }
  // Find the nearest original heading/paragraph for each reference. The original
  // prose is kept intact; only our own host is inserted immediately after it.
  function responseEntries(root){
    const refs=extract(responseText(root)),anchors=new Map();
    for(const node of root.querySelectorAll('h1,h2,h3,h4,h5,h6,p,li,tr')){
      if(node.closest('[data-cinematch-ui],[aria-hidden="true"]'))continue;
      for(const ref of extract(responseText(node))){
        const key=referenceKey(ref),previous=anchors.get(key);
        if(!previous||previous.contains(node))anchors.set(key,node);
      }
    }
    return refs.map(ref=>{const node=anchors.get(referenceKey(ref));return {ref,anchor:node?.closest('table')??node??root};});
  }
  function scheduleEntry(entry){
    clearTimeout(entry.timer);entry.stable=false;
    entry.timer=setTimeout(()=>{entry.stable=true;maybeLoad(entry);},700);
  }
  function maybeLoad(entry){
    if(!configuration.configured||!configuration.automatic||!entry.host.isConnected||!entry.near||!entry.stable||entry.phase!=='idle')return;
    entry.phase='loading';const version=entry.version;
    loadReference(entry.card,entry.ref,()=>entry.version===version&&entry.host.isConnected&&configuration.configured&&configuration.automatic).finally(()=>{if(entry.version===version)entry.phase='done';});
  }
  const cardObserver=new IntersectionObserver(entries=>{for(const item of entries){const entry=item.target.cinematchEntry;if(entry){entry.near=item.isIntersecting;if(entry.near)maybeLoad(entry);}}},{rootMargin:'250px'});
  function reconcile(state){
    const {root}=state;
    if(!state.tools.host.isConnected)root.append(state.tools.host);
    if(!configuration.configured||!configuration.automatic){for(const entry of state.entries.values())clearEntry(entry);state.entries.clear();return;}
    let entries;try{entries=responseEntries(root);}catch{return;}
    const keep=new Set(),tails=new Map();
    for(const {ref,anchor}of entries){
      const key=referenceKey(ref);keep.add(key);let entry=state.entries.get(key);
      if(!entry){
        const {host,shadow}=createHost('film'),frame=make('section',undefined,'inline-card'),card=make('article',undefined,'film');frame.setAttribute('aria-label',`Anteprima TMDB: ${ref.title}`);frame.append(card);shadow.append(frame);
        entry={host,card,ref,version:0,phase:'idle',near:false,stable:false,fingerprint:'',timer:null};host.cinematchEntry=entry;state.entries.set(key,entry);
      }
      const fingerprint=JSON.stringify(ref);
      if(entry.fingerprint!==fingerprint){entry.fingerprint=fingerprint;entry.ref=ref;entry.version++;entry.phase='idle';entry.card.replaceChildren(make('p',`Locandina e volti di ${ref.title}…`,'status'));scheduleEntry(entry);}
      const tail=tails.get(anchor)??anchor;
      if(anchor===root){if(entry.host.nextSibling!==state.tools.host)root.insertBefore(entry.host,state.tools.host);}
      else if(tail.nextSibling!==entry.host)tail.after(entry.host);
      tails.set(anchor,entry.host);cardObserver.observe(entry.host);
    }
    for(const [key,entry]of state.entries)if(!keep.has(key)){clearEntry(entry);state.entries.delete(key);}
  }
  let scanTimer;
  const scan=()=>{
    scanTimer=null;if(!configuration.ready)return;
    let roots=[];for(const selector of selectors){roots=[...document.querySelectorAll(selector)];if(roots.length)break;}
    roots=roots.filter(root=>!root.closest('[data-message-author-role="user"],[data-role="user"],[data-testid="user-message"]')&&!roots.some(other=>other!==root&&other.contains(root)));
    const current=new Set(roots);
    for(const [root,state]of rootStates)if(!root.isConnected||!current.has(root)){clearState(state);rootStates.delete(root);}
    for(const root of roots){let state=rootStates.get(root);if(!state){state={root,entries:new Map(),tools:createHost('response'),dirty:true};rootStates.set(root,state);toolsFor(state);}if(state.dirty||!state.tools.host.isConnected){reconcile(state);state.dirty=false;}}
  };
  const requestScan=()=>{if(!scanTimer)scanTimer=setTimeout(scan,250);};
  const ownNode=node=>{const el=node.nodeType===Node.ELEMENT_NODE?node:node.parentElement;return el?.closest?.('[data-cinematch-ui]');};
  new MutationObserver(mutations=>{
    const changed=mutations.filter(m=>!ownNode(m.target)&&(m.type==='characterData'||m.removedNodes.length||[...m.addedNodes].some(n=>!ownNode(n))));
    if(changed.length){for(const [root,state]of rootStates)if(changed.some(m=>root.contains(m.target)))state.dirty=true;requestScan();}
  }).observe(document.body,{childList:true,subtree:true,characterData:true});
  function applyConfiguration(value){
    configuration.ready=true;configuration.configured=Boolean(value.configured);configuration.automatic=value.automatic!==false;
    for(const state of rootStates.values()){clearState(state);toolsFor(state);state.dirty=true;}requestScan();
  }
  chrome.runtime.onMessage.addListener(message=>{if(message?.type==='tmdb-preferences-changed')applyConfiguration(message);});
  send({type:'tmdb-status'}).then(applyConfiguration).catch(()=>{});
})();
