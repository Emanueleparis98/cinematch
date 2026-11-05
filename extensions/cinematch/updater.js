/* In-app updater for the unpacked extension. It downloads the signed release
   ZIP from the project's GitHub releases, verifies its SHA-256, extracts it in
   memory and writes it into the extension folder selected by the user. */
import {extractZip} from "./zip-reader.js";

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
const MAX_BYTES = 50 * 1024 * 1024;
const currentVersion = chrome.runtime.getManifest().version;

const $ = id => document.getElementById(id);
const pageMode = new URL(location.href).searchParams.has("page");
const compactUpdater = !pageMode && !$("updates-dialog");
let latest = null;
let busy = false;
let dirHandle = null;

function openUpdaterWindow() {
  if (typeof chrome.windows?.create !== "function") {
    setStatus("Aggiornamenti non disponibili in questa finestra. Apri il pannello CineMatch.", "error");
    return;
  }
  chrome.windows.create({url:chrome.runtime.getURL("settings-popup.html?page=updates"),type:"popup",width:420,height:620,focused:true})
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
function versionIsNewer(candidate, current) {
  const a = candidate.split(".").map(Number);
  const b = current.split(".").map(Number);
  for (let index = 0; index < 3; index += 1) {
    if (a[index] !== b[index]) return a[index] > b[index];
  }
  return false;
}
function validLatest(data) {
  const version = typeof data?.version === "string" ? data.version : "";
  const sha256 = typeof data?.sha256 === "string" ? data.sha256.toLowerCase() : "";
  const size = Number(data?.size);
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error("Il canale di aggiornamento contiene una versione non valida.");
  if (!/^[a-f0-9]{64}$/.test(sha256)) throw new Error("Il canale di aggiornamento non contiene un hash SHA-256 valido.");
  if (!Number.isSafeInteger(size) || size < 1 || size > MAX_BYTES) throw new Error("Il canale di aggiornamento contiene una dimensione non valida.");
  return {version, sha256, size};
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
async function handleStore(mode) {
  const database = await openDatabase();
  return database.transaction(STORE, mode).objectStore(STORE);
}
async function readStoredHandle() {
  try {
    const store = await handleStore("readonly");
    return await new Promise((resolve, reject) => {
      const request = store.get(KEY);
      request.onsuccess = () => resolve(request.result ?? null);
      request.onerror = () => reject(request.error);
    });
  } catch { return null; }
}
async function writeStoredHandle(handle) {
  const store = await handleStore("readwrite");
  await new Promise((resolve, reject) => {
    const request = store.put(handle, KEY);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}
async function clearStoredHandle() {
  try {
    const store = await handleStore("readwrite");
    await new Promise((resolve, reject) => {
      const request = store.delete(KEY);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  } catch {}
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
    await writeStoredHandle(handle);
    dirHandle = handle;
    updateFolderLabel();
    setStatus(`Cartella pronta: ${handle.name} (CineMatch ${manifest.version}).`, "success");
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
  if (interactive && !await requestOrigins()) {
    setStatus("Accesso a GitHub non concesso. Riprova e consenti il collegamento.", "error");
    return;
  }
  setBusy(true);
  setProgress(0, false);
  $("update-apply").hidden = true;
  $("update-release").hidden = true;
  setStatus("Controllo gli aggiornamenti…");
  try {
    const response = await fetch(LATEST_URL, {cache: "no-store", credentials: "omit", redirect: "follow"});
    if (!response.ok) throw new Error(`GitHub ha risposto con errore ${response.status}.`);
    const info = validLatest(await response.json());
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
async function directoryPermission(handle) {
  if (!handle?.queryPermission) return true;
  let permission = "prompt";
  try { permission = await handle.queryPermission({mode: "readwrite"}); } catch {}
  if (permission === "granted") return true;
  if (compactUpdater) { openUpdaterWindow(); return false; }
  if (!handle.requestPermission) return false;
  try { permission = await handle.requestPermission({mode: "readwrite"}); } catch { return false; }
  return permission === "granted";
}
async function downloadZip(info) {
  const response = await fetch(RELEASE_URL(info.version), {cache: "no-store", credentials: "omit", redirect: "follow"});
  if (!response.ok) throw new Error(`Download non riuscito (${response.status}).`);
  const total = Number(response.headers.get("content-length")) || info.size;
  const reader = response.body.getReader();
  const chunks = [];
  let received = 0;
  for (;;) {
    const {done, value} = await reader.read();
    if (done) break;
    chunks.push(value);
    received += value.length;
    setProgress(Math.round(received / total * 100), true);
    setStatus(`Scarico il pacchetto… ${Math.min(100, Math.round(received / total * 100))}%`);
  }
  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  if (bytes.length !== info.size) throw new Error("La dimensione del pacchetto non corrisponde al manifesto.");
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const hash = Array.from(digest, byte => byte.toString(16).padStart(2, "0")).join("");
  if (hash !== info.sha256) throw new Error("La verifica SHA-256 del pacchetto non è riuscita.");
  return bytes;
}
async function writeFile(root, path, data) {
  const parts = path.split("/");
  const name = parts.pop();
  let directory = root;
  for (const part of parts) directory = await directory.getDirectoryHandle(part, {create: true});
  const file = await directory.getFileHandle(name, {create: true});
  const writable = await file.createWritable();
  try { await writable.write(data); } finally { await writable.close(); }
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
async function applyUpdate() {
  if (busy) return;
  if (compactUpdater && (!dirHandle || !await hasOrigins())) { openUpdaterWindow(); return; }
  if (!latest) { await check({interactive: true}); if (!latest) return; }
  if (!dirHandle) {
    setStatus("Scegli prima la cartella dell’estensione con Scegli cartella.", "error");
    $("update-folder").focus();
    return;
  }
  if (!await hasOrigins() && !await requestOrigins()) {
    setStatus("Accesso a GitHub non concesso. Riprova e consenti il collegamento.", "error");
    return;
  }
  if (!await directoryPermission(dirHandle)) {
    setStatus("La cartella non è autorizzata. Premi Cambia cartella e concedi di nuovo l’accesso.", "error");
    return;
  }
  setBusy(true);
  setProgress(0, true);
  try {
    const bytes = await downloadZip(latest);
    setStatus("Estraggo il pacchetto…");
    setProgress(0, true);
    const files = await extractZip(bytes);
    const manifest = validateArchive(files, latest);
    const manifestFile = files.find(file => file.path === "manifest.json");
    const payload = files.filter(file => file.path !== "manifest.json");
    for (let index = 0; index < payload.length; index += 1) {
      await writeFile(dirHandle, payload[index].path, payload[index].data);
      setProgress(Math.round((index + 1) / (payload.length + 1) * 100), true);
      setStatus(`Scrivo i file… ${index + 1}/${payload.length}`);
    }
    await writeFile(dirHandle, manifestFile.path, manifestFile.data);
    setProgress(100, true);
    await chrome.storage.local.set({pendingUpdate: {version: manifest.version, at: Date.now()}});
    setStatus(`CineMatch ${manifest.version} pronto. Riavvio l’estensione…`, "success");
    if (globalThis.__cineMatchUpdaterNoReload) { setBusy(false); setStatus(`CineMatch ${manifest.version} pronto (test).`, "success"); return; }
    chrome.runtime.reload();
  } catch (error) {
    setStatus(error.message ?? "Aggiornamento non riuscito. La versione installata non è stata riavviata.", "error");
    setProgress(0, false);
    setBusy(false);
  }
}
async function init() {
  $("update-current").textContent = currentVersion;
  dirHandle = await readStoredHandle();
  updateFolderLabel();
  const saved = await chrome.storage.local.get("pendingUpdate");
  if (saved.pendingUpdate) {
    await chrome.storage.local.remove("pendingUpdate");
    if (saved.pendingUpdate.version === currentVersion) {
      setStatus(`CineMatch aggiornato alla ${currentVersion}.`, "success");
      chrome.action?.setBadgeText?.({text: ""});
    } else {
      await clearStoredHandle();
      dirHandle = null;
      updateFolderLabel();
      setStatus("La cartella scelta non è quella caricata da Chrome. Premi Scegli cartella e seleziona la cartella giusta.", "error");
      return;
    }
  }
  if (await hasOrigins()) {
    if (compactUpdater) setStatus("Premi Controlla per verificare la presenza di nuove versioni.");
    else check().catch(() => {});
  } else setStatus("Premi Controlla aggiornamenti per verificare la presenza di nuove versioni.");
}
document.addEventListener("cinematch:updates-open", () => {
  if (compactUpdater && !busy && !latest) check().catch(() => {});
});
$("update-check").addEventListener("click", () => check({interactive: true}));
$("update-folder").addEventListener("click", chooseFolder);
$("update-apply").addEventListener("click", applyUpdate);
const updatesDialog = $("updates-dialog");
$("update-close")?.addEventListener("click", () => updatesDialog?.close());
updatesDialog?.addEventListener("click", event => { if (event.target === updatesDialog) updatesDialog.close(); });
updatesDialog?.addEventListener("close", () => { if (location.hash === "#updates") history.replaceState(null, "", location.pathname + location.search); });
if (location.hash === "#updates") updatesDialog?.showModal();
init().catch(() => setStatus("Aggiornamenti non disponibili in questa pagina. Riprova dopo aver riaperto il pannello.", "error"));
