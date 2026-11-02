/* Adds controls only. Response text is read exclusively after a trusted click. */
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
  const {extract}=globalThis.CineMatchReferences;
  const make=(tag,text,cls)=>{const el=document.createElement(tag);if(text!==undefined)el.textContent=text;if(cls)el.className=cls;return el;};
  const act=(label,handler,cls='')=>{const b=make('button',label,cls);b.type='button';b.addEventListener('click',e=>{if(e.isTrusted)Promise.resolve().then(()=>handler(e)).catch(error=>{const message=make('p',error.message,'status error');message.setAttribute('role','alert');b.after(message);});});return b;};
  const send=async message=>{try{const result=await chrome.runtime.sendMessage(message);if(!result?.ok)throw new Error(result?.error??'Ricarica la chat per riattivare CineMatch.');return result;}catch(error){throw new Error(error.message?.includes('context invalidated')?'Estensione aggiornata: ricarica la chat.':error.message??'CineMatch non disponibile: ricarica la chat.');}};
  const css=`
    :host{display:block;clear:both;margin:16px 0;color-scheme:dark;font:14px/1.5 Arial,sans-serif;color:#f4eee2;text-align:left;--muted:#b5b1a7;--line:#3b414d}
    *{box-sizing:border-box}[hidden]{display:none!important}button,input,select{font:inherit;color:inherit;background:#11141b;border:1px solid var(--line);border-radius:4px;padding:8px 10px;min-width:0}button{cursor:pointer}button:hover{border-color:#a7a399;background:#242936}button:disabled{opacity:.5;cursor:default}:focus-visible{outline:2px solid #ff794f;outline-offset:3px}input{width:100%;caret-color:#ff794f}a{color:#ff946f;text-underline-offset:3px;overflow-wrap:anywhere}::selection{color:#f4eee2;background:#753a28}
    .toolbar{display:flex;gap:8px;flex-wrap:wrap;align-items:center}.toolbar .main{color:#ffd2c2;border-color:#80523e;background:#201813;font-weight:600}.quiet{font-size:12px;background:transparent}.gallery{margin-top:12px;padding:16px;background:#0e1016;border:1px solid var(--line);border-radius:5px;overflow-wrap:anywhere}.heading{font:600 21px/1.25 Georgia,serif;margin:0 0 6px}.note,.status{font-size:12px;color:var(--muted);margin:8px 0}.status.error{color:#ff9aa4}.list{display:grid;gap:20px;margin-top:16px}.film{border-top:1px solid var(--line);padding-top:16px;min-width:0}.film-head{display:grid;grid-template-columns:88px minmax(0,1fr);gap:14px;align-items:start}.poster{width:88px;aspect-ratio:2/3;background:#20252f;object-fit:cover;border-radius:3px;display:block}.missing{display:flex;align-items:center;justify-content:center;text-align:center;color:var(--muted);font-size:11px;padding:5px}.film h3{font:600 19px/1.3 Georgia,serif;margin:0 0 6px}.meta{font-size:12px;color:var(--muted);margin:5px 0}.film p{margin:6px 0;line-height:1.55}.film a{font-size:12px}.people{display:grid;grid-template-columns:repeat(auto-fill,minmax(74px,1fr));gap:14px;margin:12px 0}.person{min-width:0}.person img,.person .missing{width:64px;aspect-ratio:2/3;object-fit:cover;background:#20252f;border-radius:3px;display:block;margin-bottom:5px}.person strong{display:block;font-size:12px;line-height:1.4;font-weight:600}.person small{display:block;color:var(--muted);font-size:11px;line-height:1.4;margin-top:3px}details{margin-top:12px}summary{cursor:pointer;font-size:13px;font-weight:600}summary:focus-visible{outline:2px solid #ff794f}.candidates{display:flex;flex-direction:column;gap:8px;margin:12px 0}.candidates button{text-align:left}.search-form{border-top:1px solid var(--line);padding-top:14px;margin-top:16px}.search-form label{display:block;color:var(--muted);font-size:12px;margin-bottom:5px}.search-fields{display:grid;grid-template-columns:1fr 80px;gap:8px}.search-form select{width:100%;margin-bottom:8px}.search-form button{margin-top:8px}.credits{border-top:1px solid var(--line);padding-top:12px;margin-top:20px;font-size:11px;color:var(--muted)}.more{margin-top:14px}.credits p{margin:6px 0}.type-label{font-weight:600}.manual-title{margin:18px 0 6px;font-size:14px}.original{color:var(--muted);font-size:12px}.film .retry{margin-top:8px}@media(max-width:380px){.gallery{padding:12px}.film-head{grid-template-columns:72px minmax(0,1fr);gap:10px}.poster{width:72px}.film h3{font-size:18px}}
  `;
  const sheet=new CSSStyleSheet();sheet.replaceSync(css+`.quiet{background:#11141b;color:#f4eee2}.search-fields{align-items:end}.search-fields>input{height:38px}`);
  const lazyImages=new IntersectionObserver(entries=>{for(const entry of entries){if(!entry.isIntersecting)continue;const img=entry.target;lazyImages.unobserve(img);send({type:'tmdb-image',path:img.dataset.path,size:img.dataset.size}).then(r=>{if(img.isConnected)img.src=r.data;}).catch(()=>fallbackImage(img));}},{rootMargin:'120px'});
  function fallbackImage(img){if(!img.isConnected)return;const box=make('span','Foto non disponibile',`${img.className} missing`);box.setAttribute('role','img');box.setAttribute('aria-label',img.alt);img.replaceWith(box);}
  function image(path,alt,cls='poster',size='w185'){
    if(!path){const el=make('span','Foto non disponibile',`${cls} missing`);el.setAttribute('role','img');el.setAttribute('aria-label',alt);return el;}
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
    head.append(body);card.append(head);
    if(item.type==='movie'){
      if(item.directors.length){const details=make('details');details.append(make('summary','Regia'),people(item.directors));card.append(details);}
      const cast=make('details');cast.append(make('summary',`Cast · ${item.cast.length} persone`));let count=0;const grid=make('div');
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
  async function loadReference(card,reference){
    card.replaceChildren(make('h3',reference.title),make('p','Cerco la scheda su TMDB…','status'));card.setAttribute('aria-busy','true');
    try{
      const result=await send({type:'tmdb-search',reference});if(!card.isConnected)return;
      if(result.matched){renderItem(card,result.matched);return;}
      card.replaceChildren(make('h3',reference.title),make('p',result.candidates.length?'Scegli la scheda corretta: l’abbinamento non è univoco.':'Nessuna corrispondenza. Prova il titolo originale o correggi l’anno.','note'));
      const candidates=make('div',undefined,'candidates');for(const candidate of result.candidates){const b=act(`${candidate.title}${candidate.year?' · '+candidate.year:''}`,async()=>{
        b.disabled=true;try{const response=await send({type:'tmdb-details',kind:reference.type,id:candidate.id});if(card.isConnected)renderItem(card,response.item);}catch(error){const message=make('p',error.message,'status error');candidates.append(message);b.disabled=false;}
      });candidates.append(b);}card.append(candidates);
      if(result.hasMore)card.append(make('p','Altri risultati potrebbero esistere: restringi il titolo o l’anno qui sotto.','note'));
      searchForm(card,reference,ref=>loadReference(card,ref));
    }catch(error){card.replaceChildren(make('h3',reference.title),make('p',error.message,'status error'),act('Riprova',()=>loadReference(card,reference),'retry'));}
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
  function mount(root){
    if([...root.children].some(el=>el.hasAttribute('data-cinematch-ui')))return;
    const host=make('div');host.setAttribute('data-cinematch-ui','');const shadow=host.attachShadow({mode:'open'});shadow.adoptedStyleSheets=[sheet];
    const toolbar=make('div',undefined,'toolbar'),gallery=make('section',undefined,'gallery');gallery.hidden=true;gallery.setAttribute('aria-label','Schede film e persone da TMDB');
    let loaded=false;
    async function show(refresh=false){
      if(loaded&&!refresh){gallery.hidden=!gallery.hidden;toggle.textContent=gallery.hidden?'Mostra locandine e cast':'Nascondi schede';return;}
      toggle.disabled=true;gallery.hidden=false;gallery.replaceChildren(make('p','Preparo le schede…','status'));
      try{
        const state=await send({type:'tmdb-status'});if(!state.configured){gallery.replaceChildren(make('h2','Attiva le immagini TMDB','heading'),make('p','Configura la credenziale una volta per vedere locandine e volti.','note'),act('Configura TMDB',()=>send({type:'tmdb-options'})));return;}
        const refs=extract(responseText(root));gallery.replaceChildren(make('h2','Locandine e cast','heading'),make('p',refs.length?`${refs.length} riferimenti riconosciuti. Controlla gli abbinamenti; puoi correggere o cercare altri titoli.`:'Non riconosco titoli con un anno. Cerca qui sotto oppure chiedi alla chat di usare Titolo (anno).','note'));
        const list=make('div',undefined,'list');gallery.append(list);let count=0;
        const more=act('Carica altri riferimenti',loadMore,'more');
        function loadMore(){for(const ref of refs.slice(count,count+12)){const card=make('article',undefined,'film');list.append(card);loadReference(card,ref);}count+=12;more.hidden=count>=refs.length;}
        gallery.append(more);loadMore();
        gallery.append(make('h3','Aggiungi un film o una persona','manual-title'));
        searchForm(gallery,null,ref=>{const card=make('article',undefined,'film');list.append(card);loadReference(card,ref);});
        const credits=make('div',undefined,'credits');const tmdb=make('a','Dati e immagini: TMDB');tmdb.href='https://www.themoviedb.org';tmdb.target='_blank';tmdb.rel='noopener noreferrer';credits.append(tmdb,make('p','This product uses the TMDB API but is not endorsed or certified by TMDB.'));gallery.append(credits);
        loaded=true;toggle.textContent='Nascondi schede';refreshButton.hidden=false;
      }catch(error){gallery.replaceChildren(make('p',error.message,'status error'));}
      finally{toggle.disabled=false;}
    }
    const toggle=act('Mostra locandine e cast',()=>show(), 'main');
    const refreshButton=act('Aggiorna schede',()=>show(true),'quiet');refreshButton.hidden=true;
    toolbar.append(toggle,refreshButton,act('Impostazioni TMDB',()=>send({type:'tmdb-options'}),'quiet'));shadow.append(toolbar,gallery);root.append(host);
  }
  let timer;
  const scan=()=>{
    let roots=[];for(const selector of selectors){roots=[...document.querySelectorAll(selector)];if(roots.length)break;}
    for(const root of roots)if(!root.closest('[data-message-author-role="user"],[data-role="user"],[data-testid="user-message"]')&&!roots.some(other=>other!==root&&other.contains(root)))mount(root);
  };
  new MutationObserver(()=>{clearTimeout(timer);timer=setTimeout(scan,350);}).observe(document.body,{childList:true,subtree:true});
  scan();
})();
