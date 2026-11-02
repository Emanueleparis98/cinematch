import {KINDS,MODES,PROVIDERS,defaultRequest,generatePrompt,matches,normalize,period,providerForUrl,signature} from "./core.js";
import {enhanceSelects,refreshSelects} from "./controls.js";

const $ = id => document.getElementById(id);
const state = {request:defaultRequest(),query:"",kind:"",era:"all",region:"",sort:"chronological",tab:"explore",detail:null,prompt:"",generatedSig:null,provider:"chatgpt",targetTabId:null};
let catalog = [], windowId, saveTimer, conflict = null, busy = false;
const element = (tag,className,text) => {const el=document.createElement(tag);if(className)el.className=className;if(text!==undefined)el.textContent=text;return el;};
function button(text,action,className="") {const el=element("button",className,text);el.type="button";el.addEventListener("click",action);return el;}
const currentProvider = () => PROVIDERS.find(p=>p.id===state.provider) ?? PROVIDERS[0];
const ready = () => state.prompt.trim() && state.generatedSig === signature(state.request);
function status(message="",type="") {$("status").textContent=message;$("status").className=`status ${type}`;}
function resetConflict() {conflict=null;$("conflict").hidden=true;}
function actionState() {
  $("generate").disabled=busy;$("copy").disabled=busy||!ready();$("insert").disabled=busy||!ready();
  $("stale").hidden=!state.prompt||Boolean(ready());$("preview-section").hidden=!state.prompt;
  $("prompt-length").textContent=`${state.prompt.length.toLocaleString("it-IT")} caratteri`;
  $("selection-count").textContent=String(state.request.movements.length);
}
async function saveNow() {
  clearTimeout(saveTimer);
  try {await chrome.storage.session.set({[`draft:${windowId}`]:{version:1,...state}});}
  catch {status("La bozza resta nel pannello, ma non è stato possibile conservarla nella sessione.","error");}
}
function save() {clearTimeout(saveTimer);saveTimer=setTimeout(saveNow,250);}
function criteriaChanged() {resetConflict();status();actionState();save();}
function showTab(tab) {
  state.tab=tab;$("explore-pane").hidden=tab!=="explore";$("prompt-pane").hidden=tab!=="prompt";$("actions").hidden=tab!=="prompt";
  for(const id of ["explore","prompt"]) {$(id+"-tab").setAttribute("aria-selected",String(id===tab));$(id+"-tab").tabIndex=id===tab?0:-1;}
  actionState();save();
}
function eraMatches(m) {const ranges={origins:[0,1919],interwar:[1920,1945],postwar:[1946,1979],recent:[1980,1999],contemporary:[2000,9999]};const r=ranges[state.era];return !r||(m.start<=r[1]&&(m.end??9999)>=r[0]);}
function selectionChanged() {renderSelection();renderList();criteriaChanged();}
function renderList() {
  let list=catalog.filter(m=>(!state.kind||m.kind===state.kind)&&(!state.region||m.regions.includes(state.region))&&eraMatches(m)&&matches(m,state.query));
  if(state.sort==="alphabetic")list=list.toSorted((a,b)=>normalize(a.label).localeCompare(normalize(b.label),"it"));
  const fragment=document.createDocumentFragment();
  for(const m of list) {
    const row=element("div","movement-row"),check=document.createElement("input");check.type="checkbox";check.checked=state.request.movements.includes(m.id);check.setAttribute("aria-label",`Includi ${m.label} nel prompt`);
    check.addEventListener("change",()=>{if(check.checked)state.request.movements.push(m.id);else state.request.movements=state.request.movements.filter(id=>id!==m.id);selectionChanged();});
    const copy=element("div");copy.append(button(m.label,()=>showDetail(m.id),"movement-name"),element("span","movement-meta",`${period(m)} · ${KINDS[m.kind]}`));row.append(check,copy);fragment.append(row);
  }
  $("movement-list").replaceChildren(fragment);$("results-count").textContent=`${list.length} voci trovate`;
  $("empty").hidden=Boolean(list.length);$("selection-count").textContent=String(state.request.movements.length);
}
function showDetail(id) {
  const m=catalog.find(m=>m.id===id);if(!m)return;
  state.detail=id;$("browser").hidden=true;$("detail").hidden=false;
  const article=$("detail");article.replaceChildren(button("Tutte le voci",()=>{state.detail=null;$("browser").hidden=false;article.hidden=true;save();},"text-button detail-back"),element("h1","",m.label));
  if(m.original_name!==m.label)article.append(element("p","detail-subtitle",m.original_name));
  article.append(element("p","detail-meta",`${KINDS[m.kind]} · ${period(m)} · ${m.regions.join(", ")}`),element("p","detail-summary",m.summary));
  const actions=element("div","detail-actions");
  actions.append(button("Prepara per questa voce",()=>{state.request.movements=[id];renderSelection();renderList();criteriaChanged();showTab("prompt");},"primary"));
  const add=button(state.request.movements.includes(id)?"Già nella selezione":"Aggiungi alla selezione",()=>{if(!state.request.movements.includes(id)){state.request.movements.push(id);selectionChanged();showDetail(id);}});add.disabled=state.request.movements.includes(id);actions.append(add);article.append(actions);
  const section=(title)=>{const node=element("section","detail-section");node.append(element("h2","",title));article.append(node);return node;};
  const traits=element("ul");for(const trait of m.traits)traits.append(element("li","",trait));section("Come riconoscerla").append(traits);
  section("Figure di riferimento").append(element("p","",m.authors.join(", ")));
  const works=section("Opere per orientarsi");for(const work of m.works){const row=element("div","work");row.append(element("strong","",work.title),element("span","",String(work.year)));works.append(row);}
  works.append(element("p","muted small","Esempi orientativi: appartenenze e casi di confine vanno letti nel loro contesto."));
  section("Confini e interpretazioni").append(element("p","",m.scope_note));
  const sources=section("Fonti e approfondimenti");sources.classList.add("sources");
  for(const source of m.sources) {try{const url=new URL(source.url);if(url.protocol!=="https:")continue;const a=element("a","",source.title);a.href=url.href;a.target="_blank";a.rel="noopener noreferrer";sources.append(a);}catch{/* Invalid reference links are never made clickable. */}}
  article.append(element("p","coverage","Periodi indicativi e categorie distinte. La raccolta è ampliabile e non pretende di esaurire ogni interpretazione della storia del cinema."));
  document.querySelector("main").scrollTop=0;save();
}
function renderSelection() {
  const container=$("selected");container.replaceChildren();
  for(const id of state.request.movements){const m=catalog.find(m=>m.id===id);if(m){const chip=button(m.label,()=>{state.request.movements=state.request.movements.filter(x=>x!==id);selectionChanged();});chip.setAttribute("aria-label",`Rimuovi ${m.label}`);const ns="http://www.w3.org/2000/svg",close=document.createElementNS(ns,"svg"),path=document.createElementNS(ns,"path");close.setAttribute("viewBox","0 0 16 16");close.setAttribute("aria-hidden","true");path.setAttribute("d","M4 4l8 8M12 4l-8 8");close.append(path);chip.append(close);container.append(chip);}}
  if(!state.request.movements.length)container.append(element("p","notice","Scegli almeno una voce nell’elenco delle correnti."));
  actionState();
}
function generate() {
  try {
    const result=generatePrompt(state.request,catalog);state.prompt=result.prompt;state.generatedSig=signature(state.request);$("prompt-preview").value=state.prompt;
    resetConflict();actionState();status("Richiesta pronta per la tua chat.","success");save();
    const main=document.querySelector("main"),preview=$("preview-section");
    main.scrollTo({top:main.scrollTop+preview.getBoundingClientRect().top-main.getBoundingClientRect().top,behavior:matchMedia("(prefers-reduced-motion: reduce)").matches?"instant":"smooth"});
  }catch(error){status(error.message,"error");}
}
async function syncActive() {
  try {
    const [tab]=await chrome.tabs.query({active:true,windowId});const provider=providerForUrl(tab?.url);
    if(provider){state.provider=provider.id;state.targetTabId=tab.id;$("provider").value=provider.id;refreshSelects();$("chat-state").textContent=`${provider.name} è pronta per la richiesta.`;$("chat-state").classList.add("ready");save();}
    else {$("chat-state").textContent="Apri la chat per inserire la richiesta.";$("chat-state").classList.remove("ready");}
  }catch{$("chat-state").textContent="Puoi copiare il prompt o aprire la chat scelta.";}
}
const insertionMessages={
  "wrong-site":"Apri la chat scelta nella scheda attiva, oppure seleziona il servizio corrispondente.",
  "no-editor":"Non trovo l’editor. Accedi alla chat, attendi che sia pronta e riprova. Puoi sempre usare Copia.",
  "ambiguous-editor":"Ci sono più editor aperti. Fai clic nel campo del messaggio e riprova.",
  "disabled-editor":"L’editor non è disponibile. Attendi che la chat sia pronta e riprova.",
  "dialog-open":"Chiudi la finestra sovrapposta nella chat, poi riprova.",
  "draft-changed":"La bozza nella chat è cambiata. Premi di nuovo Inserisci per scegliere come procedere.",
  "site-rejected":"Il sito non ha confermato l’inserimento. Controlla l’editor; se necessario, usa Copia e incolla.",
  "invalid-prompt":"Il prompt è vuoto o troppo lungo per l’inserimento diretto. Puoi usare Copia.",
  "unavailable":"Non è stato possibile accedere all’editor. Verifica l’accesso dell’estensione al sito o usa Copia."
};
async function insert(mode="empty") {
  if(!ready()||busy)return;busy=true;actionState();status("Preparo l’inserimento…");
  try {
    const result=await chrome.runtime.sendMessage({type:"insert-prompt",windowId,targetTabId:state.targetTabId,provider:state.provider,prompt:state.prompt,mode,draftToken:conflict?.draftToken});
    if(result?.ok){resetConflict();status(result.code==="already-inserted"?"Il prompt è già nell’editor. L’invio resta a te.":"Prompt inserito nell’editor. Rileggilo e invia quando vuoi.","success");}
    else if(result?.code==="draft-not-empty"){conflict=result;$("conflict").hidden=false;status("La bozza presente non è stata modificata.");}
    else {resetConflict();status(insertionMessages[result?.code]??insertionMessages.unavailable,"error");}
  }catch{resetConflict();status(insertionMessages.unavailable,"error");}
  finally{busy=false;actionState();}
}
function fillForm() {
  for(const key of ["mode","question","preferences","exclusions","depth","relation"])$(key).value=state.request[key];
  $("watched").value=state.request.watched.join("\n");$("exact-count").checked=state.request.count!==null;$("count-wrap").hidden=state.request.count===null;$("count").value=String(state.request.count??5);
  for(const key of ["search","kind","era","region","sort","provider"])$(key).value=key==="search"?state.query:state[key];
  $("prompt-preview").value=state.prompt;
  refreshSelects();
}
async function boot() {
  const response=await fetch(chrome.runtime.getURL("data/movements.json"));if(!response.ok)throw new Error("Reference unavailable");catalog=await response.json();
  windowId=(await chrome.windows.getCurrent()).id;
  const saved=(await chrome.storage.session.get(`draft:${windowId}`))[`draft:${windowId}`];
  if(saved?.version===1&&saved.request){
    const request={...defaultRequest(),...saved.request};request.movements=Array.isArray(request.movements)?request.movements.filter(id=>catalog.some(m=>m.id===id)):[];
    if(Array.isArray(request.watched)&&request.watched.every(s=>typeof s==="string")&&typeof request.question==="string"&&typeof request.preferences==="string"&&typeof request.exclusions==="string"){
      state.request=request;
      for(const key of ["query","kind","era","region","sort","tab","detail","prompt","generatedSig","provider"])if(saved[key]===null||typeof saved[key]==="string")state[key]=saved[key];
      if(Number.isInteger(saved.targetTabId))state.targetTabId=saved.targetTabId;
    }
  }
  if(!PROVIDERS.some(p=>p.id===state.provider))state.provider="chatgpt";
  for(const [value,text]of Object.entries(KINDS))$("kind").append(new Option(text,value));
  for(const [value,text]of Object.entries(MODES))$("mode").append(new Option(text,value));
  for(const region of [...new Set(catalog.flatMap(m=>m.regions))].sort((a,b)=>a.localeCompare(b,"it")))$("region").append(new Option(region,region));
  for(const provider of PROVIDERS)$("provider").append(new Option(provider.name,provider.id));
  fillForm();renderList();renderSelection();if(state.detail)showDetail(state.detail);showTab(state.tab==="prompt"?"prompt":"explore");
  $("explore-tab").addEventListener("click",()=>showTab("explore"));$("prompt-tab").addEventListener("click",()=>showTab("prompt"));
  document.querySelector(".tabs").addEventListener("keydown",event=>{if(["ArrowLeft","ArrowRight"].includes(event.key)){event.preventDefault();const tab=state.tab==="prompt"?"explore":"prompt";showTab(tab);$(tab+"-tab").focus();}});
  $("search").addEventListener("input",()=>{state.query=$("search").value;renderList();save();});
  for(const key of ["kind","era","region","sort"])$(key).addEventListener("change",()=>{state[key]=$(key).value;renderList();save();});
  $("reset").addEventListener("click",()=>{state.query="";state.kind="";state.era="all";state.region="";state.sort="chronological";fillForm();renderList();save();});
  $("request-form").addEventListener("submit",event=>{event.preventDefault();generate();});
  $("request-form").addEventListener("input",()=>{
    for(const key of ["mode","question","preferences","exclusions","depth","relation"])state.request[key]=$(key).value;
    state.request.watched=$("watched").value.split(/[\n;]/).map(s=>s.trim()).filter(Boolean);
    state.request.count=$("exact-count").checked?($("count").value===""?0:Number($("count").value)):null;$("count-wrap").hidden=!$("exact-count").checked;criteriaChanged();
  });
  $("prompt-preview").addEventListener("input",()=>{state.prompt=$("prompt-preview").value;resetConflict();status();actionState();save();});
  $("generate").addEventListener("click",generate);
  $("copy").addEventListener("click",async()=>{if(!ready())return;try{await navigator.clipboard.writeText(state.prompt);status("Prompt copiato. Puoi incollarlo in qualsiasi chat.","success");}catch{status("Copia non riuscita: seleziona il testo del prompt e usa Ctrl+C.","error");}});
  $("provider").addEventListener("change",()=>{state.provider=$("provider").value;resetConflict();status();$("chat-state").textContent=`Apri ${currentProvider().name} e premi Inserisci nella chat, oppure usa Copia.`;$("chat-state").classList.remove("ready");save();});
  $("open-chat").addEventListener("click",async()=>{try{const tab=await chrome.tabs.create({url:currentProvider().url,windowId});state.targetTabId=tab.id;status("Attendi che la chat sia pronta, poi premi Inserisci nella chat.");}catch{status("Non è stato possibile aprire la chat. Puoi usare Copia.","error");}});
  $("insert").addEventListener("click",()=>insert());$("append").addEventListener("click",()=>insert("append"));$("replace").addEventListener("click",()=>insert("replace"));$("cancel-conflict").addEventListener("click",()=>{resetConflict();status("Bozza mantenuta.");});
  $("expand").addEventListener("click",async()=>{await saveNow();await chrome.tabs.create({url:chrome.runtime.getURL("panel.html?view=full"),windowId});});
  $("tmdb-settings").addEventListener("click",()=>chrome.runtime.openOptionsPage());
  chrome.runtime.sendMessage({type:"tmdb-status"}).then(result=>{$("tmdb-settings").classList.toggle("connected",Boolean(result?.configured));$("tmdb-settings").title=result?.configured?"TMDB collegato · immagini automatiche":"Collega TMDB per le immagini";}).catch(()=>{});
  chrome.tabs.onActivated.addListener(info=>{if(info.windowId===windowId)syncActive();});
  chrome.tabs.onUpdated.addListener((_id,info)=>{if(info.url||info.status==="complete")syncActive();});
  document.addEventListener("visibilitychange",()=>{if(document.visibilityState==="hidden")saveNow();});
  document.addEventListener("keydown",event=>{if((event.ctrlKey||event.metaKey)&&event.key==="Enter"&&state.tab==="prompt"){event.preventDefault();generate();}if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==="f"){event.preventDefault();state.detail=null;$("browser").hidden=false;$("detail").hidden=true;showTab("explore");$("search").focus();}});
  if(new URL(location.href).searchParams.get("view")==="full")document.body.classList.add("full-page");
  enhanceSelects();
  await syncActive();
}
boot().catch(()=>{$("boot-error").hidden=false;document.querySelector("main").hidden=true;$("actions").hidden=true;});
