const $ = id => document.getElementById(id);
const send = message => chrome.runtime.sendMessage(message);
const TMDB_SITE_ORIGIN = "https://www.themoviedb.org/*";
const CB01_ORIGINS = [
  "https://cb01one.org/*","https://www.cb01one.org/*","https://stayonline.pro/*","https://uprot.net/*",
  "https://*.mxdrop.click/*","https://*.m1xdrop.net/*","https://*.mixdrop.co/*","https://*.mixdrop.ag/*",
  "https://*.mxdrop.to/*","https://*.mixdrop.to/*","https://*.mxcontent.net/*"
];
const DEEPSEEK_KEY = "deepseekCurrentsButton";
let siteConfigured = false;
let busy = false;

const pageName = new URL(location.href).searchParams.get("page");
const pageMode = pageName !== null;
if (pageMode) {
  document.documentElement.removeAttribute("data-action");
  document.body.classList.add("ss-page");
}

const systemTheme = () => matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
document.documentElement.dataset.theme = systemTheme();
matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", () => {
  document.documentElement.dataset.theme = systemTheme();
});

function status(id, message, error = false) {
  $(id).textContent = message;
  $(id).className = `cc-status${error ? " error" : ""}`;
}
function showTab(name) {
  const selected = ["connection","streams","functions","updates"].includes(name) ? name : "connection";
  $("tab-connection").setAttribute("aria-selected", String(selected === "connection"));
  $("tab-streams").setAttribute("aria-selected", String(selected === "streams"));
  $("streams-view").hidden = selected !== "streams";
  $("tab-functions").setAttribute("aria-selected", String(selected === "functions"));
  $("tab-updates").setAttribute("aria-selected", String(selected === "updates"));
  $("connection-view").hidden = selected !== "connection";
  $("functions-view").hidden = selected !== "functions";
  $("updates-view").hidden = selected !== "updates";
  if (selected === "updates") document.dispatchEvent(new Event("cinematch:updates-open"));
}

async function refreshSite() {
  const result = await send({type:"tmdb-site-status"});
  if (!result?.ok) throw new Error(result?.error ?? "Stato TMDB non disponibile.");
  siteConfigured = Boolean(result.configured);
  $("tmdb-site-toggle").checked = Boolean(result.enabled);
  $("tmdb-site-toggle").disabled = !siteConfigured || busy;
  status("tmdb-site-status", !siteConfigured
    ? "Collega prima TMDB qui sopra."
    : result.enabled
      ? "Attivo su themoviedb.org."
      : result.requested && !result.allowed
        ? "Permesso non più attivo: riattiva l’opzione."
        : "Disattivo.");
}

async function refreshCb01() {
  const result = await send({type:"cb01-status"});
  if (!result?.ok) throw new Error(result?.error ?? "Stato CB01 non disponibile.");
  $("cb01-toggle").checked = Boolean(result.enabled);
  $("cb01-toggle").disabled = busy;
  status("cb01-status", result.enabled
    ? "Attivo su cb01one.org, film e serie TV."
    : result.requested && !result.allowed
      ? "Permesso non più attivo: riattiva l’opzione."
      : "Disattivo.");
}

async function refreshDeepSeek() {
  const data = await chrome.storage.local.get(DEEPSEEK_KEY);
  const enabled = data[DEEPSEEK_KEY] === true;
  $("deepseek-toggle").checked = enabled;
  $("deepseek-toggle").disabled = busy;
  status("deepseek-status", enabled
    ? "Pulsante Correnti attivo nel composer DeepSeek."
    : "Pulsante Correnti disattivo.");
}

async function refresh() {
  const result = await send({type:"tmdb-status"});
  $("connection").textContent = result?.configured
    ? "TMDB è configurato. Le immagini sono automatiche."
    : "Collega TMDB per attivare immagini e metadati.";
  $("remove").disabled = Boolean(result?.configured) === false;
  await Promise.allSettled([refreshSite(), refreshCb01(), refreshDeepSeek()]);
}

$("tab-connection").addEventListener("click", () => showTab("connection"));
$("tab-streams").addEventListener("click",()=>showTab("streams"));
$("tab-functions").addEventListener("click", () => showTab("functions"));
$("tab-updates").addEventListener("click", () => showTab("updates"));
$("close").addEventListener("click", () => window.close());
if (pageName === "updates") showTab("updates");
$("reveal").addEventListener("click", () => {
  const show = $("credential").type === "password";
  $("credential").type = show ? "text" : "password";
  $("reveal").textContent = show ? "Nascondi" : "Mostra";
  $("reveal").setAttribute("aria-pressed", String(show));
});
$("credential-form").addEventListener("submit", async event => {
  event.preventDefault();
  busy = true;
  $("save").disabled = true;
  $("remove").disabled = true;
  status("settings-status", "Verifico la connessione a TMDB…");
  try {
    const result = await send({type:"tmdb-save", credential:$("credential").value, remember:$("remember").checked});
    if (!result?.ok) throw new Error(result?.error ?? "Salvataggio non riuscito.");
    $("credential").value = "";
    $("credential").type = "password";
    $("reveal").textContent = "Mostra";
    $("reveal").setAttribute("aria-pressed", "false");
    status("settings-status", "Credenziale verificata e salvata.");
  } catch (error) {
    status("settings-status", error.message, true);
  } finally {
    busy = false;
    $("save").disabled = false;
    await refresh();
  }
});
$("remove").addEventListener("click", async () => {
  const result = await send({type:"tmdb-remove"});
  status("settings-status", result?.ok ? "Credenziale rimossa." : "Rimozione non riuscita.", !result?.ok);
  await refresh();
});
$("tmdb-site-toggle").addEventListener("change", async () => {
  const toggle = $("tmdb-site-toggle");
  busy = true; toggle.disabled = true;
  try {
    if (toggle.checked) {
      const granted = await chrome.permissions.request({origins:[TMDB_SITE_ORIGIN]});
      if (!granted) { toggle.checked = false; throw new Error("Permesso TMDB non concesso."); }
      await chrome.storage.local.set({tmdbSiteButton:true});
      const result = await send({type:"tmdb-site-sync"});
      if (!result?.ok) throw new Error(result?.error ?? "Attivazione non riuscita.");
      status("tmdb-site-status", "Attivo su themoviedb.org.");
    } else {
      const result = await send({type:"tmdb-site-disable"});
      if (!result?.ok) throw new Error(result?.error ?? "Disattivazione non riuscita.");
      status("tmdb-site-status", "Disattivo.");
      await Promise.race([chrome.permissions.remove({origins:[TMDB_SITE_ORIGIN]}).catch(()=>false), new Promise(resolve=>setTimeout(resolve,1200))]);
    }
  } catch (error) {
    status("tmdb-site-status", error.message, true);
    await refreshSite().catch(()=>{});
  } finally {
    busy = false; toggle.disabled = !siteConfigured;
  }
});
$("cb01-toggle").addEventListener("change", async () => {
  const toggle = $("cb01-toggle");
  busy = true; toggle.disabled = true;
  try {
    if (toggle.checked) {
      const granted = await chrome.permissions.request({origins:CB01_ORIGINS});
      if (!granted) { toggle.checked = false; throw new Error("Permesso CB01 non concesso."); }
      const result = await send({type:"cb01-sync"});
      if (!result?.ok) throw new Error(result?.error ?? "Attivazione CB01 non riuscita.");
      status("cb01-status", "Attivo su cb01one.org.");
    } else {
      const result = await send({type:"cb01-disable"});
      if (!result?.ok) throw new Error(result?.error ?? "Disattivazione CB01 non riuscita.");
      status("cb01-status", "Disattivo.");
      await Promise.race([chrome.permissions.remove({origins:CB01_ORIGINS}).catch(()=>false), new Promise(resolve=>setTimeout(resolve,1200))]);
    }
  } catch (error) {
    status("cb01-status", error.message, true);
    await refreshCb01().catch(()=>{});
  } finally {
    busy = false; toggle.disabled = false;
  }
});
$("deepseek-toggle").addEventListener("change", async () => {
  const toggle = $("deepseek-toggle");
  busy = true; toggle.disabled = true;
  try {
    await chrome.storage.local.set({[DEEPSEEK_KEY]: toggle.checked});
    const result = await send({type:"deepseek-currents-sync"});
    if (!result?.ok) throw new Error(result?.error ?? "Aggiornamento DeepSeek non riuscito.");
    status("deepseek-status", toggle.checked ? "Pulsante Correnti attivo nel composer DeepSeek." : "Pulsante Correnti disattivo.");
  } catch (error) {
    status("deepseek-status", error.message, true);
    await refreshDeepSeek().catch(()=>{});
  } finally {
    busy = false; toggle.disabled = false;
  }
});

$("current-version").textContent = chrome.runtime.getManifest().version;
refresh().catch(() => {
  $("connection").textContent = "Impostazioni non disponibili. Riapri il popup.";
});
