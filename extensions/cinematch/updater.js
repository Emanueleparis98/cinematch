/* In-app updater for the unpacked extension. It downloads the verified release
   ZIP from the project's GitHub releases, verifies its SHA-256, extracts it in
   memory and writes it into the extension folder selected by the user. */
import {extractZip} from "./zip-reader.js";
import {validUpdate,versionIsNewer,readUpdateBody,verifyLoadedFolder,installUpdate} from "./update-core.js";

const REPO = "Emanueleparis98/cinematch";
const LATEST_URL = `https://github.com/${REPO}/releases/latest/download/latest.json`;
const RELEASE_URL = version => `https://github.com/${REPO}/releases/download/v${version}/CineMatch-Chrome-${version}.zip`;
const RELEASE_TAG = version => `https://github.com/${REPO}/releases/tag/v${version}`;
const ORIGINS = [
  "https://github.com/*",
  "https://objects.githubusercontent.com/*",
  "https://release-assets.githubusercontent.com/*"
];
const DB_NAME = "cinematch-updater";
const STORE = "handles";
const KEY = "extension";
const currentVersion = chrome.runtime.getManifest().version;

const $ = id => document.getElementById(id);
const pageMode = new URL(location.href).searchParams.has("page");
const compactUpdater = !pageMode && !$("updates-dialog");
let latest = null;
let busy = false;
let dirHandle = null;
let githubAllowed = false;
let initializing = true;
const applyOnOpen = new URL(location.href).searchParams.get("apply") === "1";

function openUpdaterWindow({apply=false}={}) {
  if (typeof chrome.windows?.create !== "function") {
    setStatus("Aggiornamenti non disponibili in questa finestra. Apri il pannello CineMatch.", "error");
    return;
  }
  chrome.windows.create({url:chrome.runtime.getURL(`settings-popup.html?page=updates${apply?"&apply=1":""}`),type:"popup",width:420,height:620,focused:true})
    .then(()=>window.close())
    .catch(error=>setStatus(error.message ?? "Non riesco ad aprire gli aggiornamenti.", "error"));
}

function setStatus(text, type = "") {
  const node = $("update-status");
  node.textContent = text;
  node.className = type;
}
function setProgress(value, visible) {
  const bar = $("update-progress");
  bar.hidden = !visible;
  if (visible) bar.value = Math.max(0, Math.min(100, value));
}
function setBusy(value) {
  busy = value;
  $("update-check").disabled = value;
  $("update-apply").disabled = value;
  $("update-folder").disabled = value;
}
function updateFolderLabel() {
  $("update-folder").textContent = dirHandle ? "Cambia cartella" : "Scegli cartella";
  $("update-folder").title = dirHandle ? `Cartella attuale: ${dirHandle.name}` : "Scegli la cartella dell’estensione";
}
async function hasOrigins() {
  try { return await chrome.permissions.contains({origins: ORIGINS}); } catch { return false; }
}
async function requestOrigins() {
  try { return await chrome.permissions.request({origins: ORIGINS}); } catch { return false; }
}
async function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}
async function storedHandle(mode, operation) {
  const database=await openDatabase();
  try {
    return await new Promise((resolve,reject)=>{
      const transaction=database.transaction(STORE,mode),request=operation(transaction.objectStore(STORE));
      let value;
      request.onsuccess=()=>{value=request.result;};
      transaction.oncomplete=()=>resolve(value??null);
      transaction.onerror=()=>reject(transaction.error??new Error("Archivio cartella non disponibile."));
      transaction.onabort=()=>reject(transaction.error??new Error("Salvataggio cartella interrotto."));
    });
  } finally { database.close(); }
}
async function readStoredHandle() {
  return storedHandle("readonly",store=>store.get(KEY));
}
async function writeStoredHandle(handle) {
  await storedHandle("readwrite",store=>store.put(handle,KEY));
}
async function validateFolder(handle) {
  const file = await handle.getFileHandle("manifest.json");
  const manifest = JSON.parse(await (await file.getFile()).text());
  if (!String(manifest.name ?? "").includes("CineMatch")) {
    throw new Error("La cartella scelta non è quella di CineMatch: manca il manifest dell’estensione.");
  }
  return manifest;
}
async function chooseFolder() {
  if (compactUpdater) { openUpdaterWindow(); return; }
  if (typeof window.showDirectoryPicker !== "function") {
    setStatus("Questo browser non supporta la scelta della cartella. Aggiorna Chrome o usa la build manuale.", "error");
    return;
  }
  let handle;
  try {
    handle = await window.showDirectoryPicker({mode: "readwrite", id: "cinematch-extension"});
  } catch (error) {
    if (error?.name !== "AbortError") setStatus(error.message ?? "Cartella non selezionata.", "error");
    return;
  }
  try {
    const manifest = await validateFolder(handle);
    await verifyLoadedFolder(handle,path=>chrome.runtime.getURL(path));
    await writeStoredHandle(handle);
    dirHandle = handle;
    updateFolderLabel();
    await chrome.storage.local.remove("pendingUpdate");
    setStatus(`Cartella ricordata: ${handle.name}. Non devi selezionarla di nuovo.`, "success");
    if(applyOnOpen&&latest&&versionIsNewer(latest.version,currentVersion))await applyUpdate();
  } catch (error) {
    setStatus(error.message ?? "La cartella scelta non è valida.", "error");
  }
}
async function check({interactive = false} = {}) {
  if (busy) return;
  if (!interactive && !await hasOrigins()) {
    setStatus("Premi Controlla aggiornamenti per verificare la presenza di nuove versioni.");
    return;
  }
  if (interactive && compactUpdater && !await hasOrigins()) { openUpdaterWindow(); return; }
  if (interactive && !await hasOrigins() && !await requestOrigins()) {
    setStatus("Accesso a GitHub non concesso. Riprova e consenti il collegamento.", "error");
    return;
  }
  setBusy(true);
  setProgress(0, false);
  $("update-apply").hidden = true;
  $("update-release").hidden = true;
  setStatus("Controllo gli aggiornamenti…");
  try {
    githubAllowed = await hasOrigins();
    const response = await fetch(LATEST_URL, {cache: "no-store", credentials: "omit", redirect: "follow", signal:AbortSignal.timeout(15000)});
    if (!response.ok) throw new Error(`GitHub ha risposto con errore ${response.status}.`);
    const info = validUpdate(JSON.parse(new TextDecoder().decode(await readUpdateBody(response,{maxBytes:64*1024}))));
    latest = info;
    if (!versionIsNewer(info.version, currentVersion)) {
      setStatus(`CineMatch è aggiornato (${currentVersion}).`, "success");
      chrome.action?.setBadgeText?.({text: ""});
    } else {
      setStatus(`Aggiornamento ${info.version} disponibile.` , "success");
      $("update-release").href = RELEASE_TAG(info.version);
      $("update-release").hidden = false;
      $("update-apply").hidden = false;
    }
  } catch (error) {
    latest = null;
    setStatus(error.message ?? "Non è stato possibile controllare gli aggiornamenti.", "error");
  } finally {
    setBusy(false);
  }
}
async function directoryPermission(handle,{automatic=false}={}) {
  if(!handle?.queryPermission)return true;
  if(automatic)return await handle.queryPermission({mode:"readwrite"})==="granted";
  // Called directly by the apply click: do not spend its user gesture on
  // network requests or IndexedDB before asking for this existing handle.
  if(handle.requestPermission)return await handle.requestPermission({mode:"readwrite"})==="granted";
  return await handle.queryPermission({mode:"readwrite"})==="granted";
}
async function downloadZip(info) {
  const response=await fetch(RELEASE_URL(info.version),{cache:"no-store",credentials:"omit",redirect:"follow",signal:AbortSignal.timeout(90000)});
  if(!response.ok)throw new Error(`Download non riuscito (${response.status}).`);
  const bytes=await readUpdateBody(response,{maxBytes:info.size,expectedSize:info.size,onProgress:value=>{
    setProgress(value,true);setStatus(`Scarico il pacchetto… ${value}%`);
  }});
  const digest=new Uint8Array(await crypto.subtle.digest("SHA-256",bytes));
  const hash=Array.from(digest,byte=>byte.toString(16).padStart(2,"0")).join("");
  if(hash!==info.sha256)throw new Error("La verifica SHA-256 del pacchetto non è riuscita.");
  return bytes;
}
function validateArchive(files, info) {
  const manifestFile = files.find(file => file.path === "manifest.json");
  if (!manifestFile) throw new Error("Il pacchetto non contiene manifest.json.");
  let manifest;
  try { manifest = JSON.parse(new TextDecoder().decode(manifestFile.data)); }
  catch { throw new Error("Il manifest del pacchetto non è valido."); }
  if (!String(manifest.name ?? "").includes("CineMatch")) throw new Error("Il pacchetto non è CineMatch.");
  if (manifest.version !== info.version) throw new Error("La versione del pacchetto non corrisponde al canale di aggiornamento.");
  for (const required of ["background.js", "launcher.js", "panel.html", "cinema.html", "watch.html"]) {
    if (!files.some(file => file.path === required)) throw new Error(`Il pacchetto non contiene ${required}.`);
  }
  return manifest;
}
async function applyUpdate({automatic=false}={}) {
  if(busy||initializing)return;
  if(compactUpdater){openUpdaterWindow({apply:true});return;}
  if(!latest){await check({interactive:!automatic});return;}
  if(!versionIsNewer(latest.version,currentVersion)){setStatus(`CineMatch è aggiornato (${currentVersion}).`,"success");return;}
  if(!dirHandle){setStatus("Scegli una volta la cartella dell’estensione. Verrà ricordata per i prossimi aggiornamenti.");$("update-folder").focus();return;}
  let allowed=false;
  try{allowed=await directoryPermission(dirHandle,{automatic});}catch{}
  if(!allowed){
    $("update-apply").hidden=false;$("update-apply").textContent="Conferma accesso e aggiorna";
    setStatus("La cartella è ricordata. Premi Conferma accesso e aggiorna per autorizzarla, senza selezionarla di nuovo.");return;
  }
  if(!await hasOrigins()){githubAllowed=false;setStatus("Premi Controlla per riconfermare il collegamento a GitHub.");return;}
  setBusy(true);setProgress(0,false);
  try {
    await navigator.locks.request("cinematch-extension-update",{ifAvailable:true},async lock=>{
      if(!lock)throw new Error("Un altro aggiornamento è già in corso. Attendi che termini.");
      setStatus("Verifico la cartella salvata…");
      await validateFolder(dirHandle);
      await verifyLoadedFolder(dirHandle,path=>chrome.runtime.getURL(path));
      const bytes=await downloadZip(latest);
      setStatus("Verifico ed estraggo il pacchetto…");
      const files=await extractZip(bytes),manifest=validateArchive(files,latest);
      setStatus("Preparo l’aggiornamento e il ripristino dei file…");
      await installUpdate(dirHandle,files,{onProgress:(done,total)=>{
        setProgress(Math.round(done/total*100),true);setStatus(`Aggiorno i file modificati… ${done}/${total}`);
      }});
      await chrome.storage.local.set({pendingUpdate:{version:manifest.version,at:Date.now()}});
      setProgress(100,true);setStatus(`CineMatch ${manifest.version} pronto. Riavvio l’estensione…`,"success");
      if(globalThis.__cineMatchUpdaterNoReload){setStatus(`CineMatch ${manifest.version} pronto (test).`,"success");return;}
      chrome.runtime.reload();
    });
  } catch(error) {
    setStatus(error.name==="TimeoutError"?"Il download ha impiegato troppo tempo. La cartella resta ricordata: riprova.":error.message??"Aggiornamento non riuscito. Riprova.","error");
    setProgress(0,false);
  } finally {setBusy(false);}
}
async function init() {
  $("update-current").textContent=currentVersion;
  dirHandle=await readStoredHandle();updateFolderLabel();
  const saved=await chrome.storage.local.get("pendingUpdate");
  if(saved.pendingUpdate){
    if(!versionIsNewer(saved.pendingUpdate.version,currentVersion)){
      await chrome.storage.local.remove("pendingUpdate");
      setStatus(`CineMatch aggiornato alla ${currentVersion}.`,"success");
      chrome.action?.setBadgeText?.({text:""});
    }else{
      // A delayed reload is not evidence of a wrong directory. Keep both the
      // handle and the pending version so the user can retry the restart.
      setStatus(`Pacchetto ${saved.pendingUpdate.version} scritto. La versione attiva è ancora ${currentVersion}: premi Riavvia estensione.`);
      if($("update-reload"))$("update-reload").hidden=false;
      return;
    }
  }
  githubAllowed=await hasOrigins();
  if(githubAllowed){if(!compactUpdater||applyOnOpen)await check();else setStatus("Premi Controlla per verificare la presenza di nuove versioni.");}
  else setStatus("Premi Controlla per collegare GitHub. La cartella salvata verrà riutilizzata.");
}
document.addEventListener("cinematch:updates-open", () => {
  if (compactUpdater && !busy && !latest) check().catch(() => {});
});
$("update-check").addEventListener("click", () => check({interactive: true}));
$("update-folder").addEventListener("click", chooseFolder);
$("update-apply").addEventListener("click", ()=>applyUpdate());
$("update-reload")?.addEventListener("click",()=>chrome.runtime.reload());
const updatesDialog = $("updates-dialog");
$("update-close")?.addEventListener("click", () => updatesDialog?.close());
updatesDialog?.addEventListener("click", event => { if (event.target === updatesDialog) updatesDialog.close(); });
updatesDialog?.addEventListener("close", () => { if (location.hash === "#updates") history.replaceState(null, "", location.pathname + location.search); });
if (location.hash === "#updates") updatesDialog?.showModal();
$("update-check").disabled=true;$("update-folder").disabled=true;$("update-apply").disabled=true;
init().catch(()=>setStatus("Non riesco a leggere la cartella ricordata. Riapri il popup prima di selezionarla di nuovo.","error")).finally(async()=>{
  initializing=false;setBusy(false);
  if(applyOnOpen&&githubAllowed&&latest&&versionIsNewer(latest.version,currentVersion))await applyUpdate({automatic:true});
});
