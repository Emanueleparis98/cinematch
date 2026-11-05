import {KINDS,MAX_MOVEMENTS,MODES,defaultRequest,generatePrompt,matches,normalize,period} from "./core.js";

const $ = id => document.getElementById(id);
const embedded = window.parent !== window;
if (!embedded) {
  document.documentElement.dataset.action = "true";
  const systemTheme = () => matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  document.documentElement.dataset.theme = systemTheme();
  matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", () => { document.documentElement.dataset.theme = systemTheme(); });
}
const state = {
  mode:"currents", selected:[], catalog:[], query:"", view:"form",
  goal:"recommendations", depth:"concise", question:"",
  preferences:"", exclusions:"", watched:"", prompt:""
};
const status = (message="", error=false) => {
  $("status").textContent = message;
  $("status").className = `cc-status${error ? " error" : ""}`;
};
const hint = message => { $("hint").textContent = message; };
const post = (type, extra={}) => { try { parent.postMessage({type,...extra}, "*"); } catch {} };
const closePanel = () => { if (embedded) post("cinematch:compact-close"); else window.close(); };

function selectedMovements() {
  return state.selected.map(id => state.catalog.find(item => item.id === id)).filter(Boolean);
}
function toggleMovement(id) {
  const index = state.selected.indexOf(id);
  if (index >= 0) state.selected.splice(index, 1);
  else if (state.selected.length < MAX_MOVEMENTS) state.selected.push(id);
  else { status(`Puoi scegliere al massimo ${MAX_MOVEMENTS} correnti.`, true); return; }
  status();
  renderChips();
  renderList();
}
function renderChips() {
  const node = $("chips");
  node.replaceChildren();
  node.hidden = !state.selected.length;
  for (const movement of selectedMovements()) {
    const chip = document.createElement("span");
    chip.className = "cc-chip";
    const label = document.createElement("span");
    label.textContent = movement.label;
    const remove = document.createElement("button");
    remove.type = "button";
    remove.setAttribute("aria-label", `Rimuovi ${movement.label}`);
    remove.textContent = "×";
    remove.addEventListener("click", () => toggleMovement(movement.id));
    chip.append(label, remove);
    node.append(chip);
  }
}
function renderList() {
  const query = normalize(state.query);
  const list = state.catalog.filter(movement => !query || matches(movement, query));
  $("list").replaceChildren();
  $("empty").hidden = Boolean(list.length);
  for (const movement of list) {
    const row = document.createElement("div");
    row.className = "cc-row";
    row.dataset.id = movement.id;
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(state.selected.includes(movement.id)));
    row.tabIndex = 0;
    const check = document.createElement("span");
    check.className = "cc-check";
    const main = document.createElement("span");
    main.className = "cc-row-main";
    const title = document.createElement("strong");
    title.textContent = movement.label;
    const meta = document.createElement("small");
    meta.textContent = `${period(movement)} · ${KINDS[movement.kind]}`;
    main.append(title, meta);
    row.append(check, main);
    row.addEventListener("click", () => toggleMovement(movement.id));
    row.addEventListener("keydown", event => {
      if (event.key === "Enter" || event.key === " ") { event.preventDefault(); toggleMovement(movement.id); }
    });
    $("list").append(row);
  }
}
function updateTabs() {
  const currents = state.mode === "currents";
  $("tab-currents").setAttribute("aria-selected", String(currents));
  $("tab-free").setAttribute("aria-selected", String(!currents));
  $("currents-view").hidden = !currents || state.view === "preview";
  $("free-view").hidden = currents || state.view === "preview";
  $("preview-view").hidden = state.view !== "preview";
  $("form-actions").hidden = state.view === "preview";
  $("preview-actions").hidden = state.view !== "preview";
  $("conflict").hidden = true;
  hint(currents
    ? state.selected.length ? `${state.selected.length} correnti scelte` : "Scegli una o più correnti."
    : "Scrivi la tua richiesta.");
}
function setMode(mode) {
  state.mode = mode;
  state.view = "form";
  updateTabs();
  (mode === "currents" ? $("search") : $("free-question")).focus();
}
function syncFields() {
  state.goal = $("goal").value;
  state.depth = state.mode === "currents" ? $("depth").value : $("free-depth").value;
  state.question = state.mode === "currents" ? $("current-question").value.trim() : $("free-question").value.trim();
  state.preferences = (state.mode === "currents" ? $("preferences") : $("free-preferences")).value.trim();
  state.exclusions = (state.mode === "currents" ? $("exclusions") : $("free-exclusions")).value.trim();
  state.watched = (state.mode === "currents" ? $("watched") : $("free-watched")).value.trim();
}
function makeRequest() {
  syncFields();
  const free = state.mode === "free";
  return {
    ...defaultRequest(),
    mode: free ? "free" : state.goal,
    movements: free ? [] : [...state.selected],
    question: state.question,
    depth: state.depth,
    preferences: state.preferences,
    exclusions: state.exclusions,
    watched: state.watched.split(/\n|;/).map(value => value.trim()).filter(Boolean)
  };
}
function generate() {
  try {
    const request = makeRequest();
    if (state.mode === "currents" && !state.selected.length) throw new Error("Scegli almeno una corrente.");
    if (state.mode === "free" && !request.question) throw new Error("Scrivi la tua richiesta.");
    const result = generatePrompt(request, state.catalog);
    state.prompt = result.prompt;
    $("prompt-preview").value = state.prompt;
    state.view = "preview";
    updateTabs();
    status();
    $("prompt-preview").focus();
  } catch (error) {
    status(error.message, true);
  }
}
function back() {
  state.view = "form";
  updateTabs();
}
async function copyPrompt() {
  try {
    await navigator.clipboard.writeText(state.prompt);
    status("Prompt copiato.");
  } catch {
    status("Copia non disponibile. Seleziona il testo e copialo manualmente.", true);
  }
}
async function insert(mode="empty") {
  if (!state.prompt) return;
  status("Inserisco nella chat…");
  try {
    const result = await chrome.runtime.sendMessage({type:"deepseek-insert-prompt", prompt:state.prompt, mode});
    if (result?.ok) {
      status("Prompt inserito.");
      closePanel();
      return;
    }
    if (result?.code === "draft-not-empty") {
      $("conflict").hidden = false;
      status("La chat contiene già una bozza: scegli come procedere.", true);
      return;
    }
    status(result?.error ?? "Inserimento non riuscito.", true);
  } catch {
    status("CineMatch non è disponibile in questa pagina.", true);
  }
}

function load() {
  $("goal").replaceChildren(...Object.entries(MODES).filter(([key]) => key !== "free").map(([key,label]) => new Option(label,key)));
  $("goal").value = "recommendations";
  $("tab-currents").addEventListener("click", () => setMode("currents"));
  $("tab-free").addEventListener("click", () => setMode("free"));
  $("search").addEventListener("input", event => {state.query = event.target.value; renderList();});
  $("generate").addEventListener("click", generate);
  $("copy").addEventListener("click", copyPrompt);
  $("insert").addEventListener("click", () => insert("empty"));
  $("append").addEventListener("click", () => insert("append"));
  $("replace").addEventListener("click", () => insert("replace"));
  $("cancel-conflict").addEventListener("click", () => {$("conflict").hidden = true; status();});
  $("back").addEventListener("click", back);
  $("close").addEventListener("click", closePanel);
  $("settings").hidden = embedded;
  $("settings").addEventListener("click", () => { chrome.runtime.openOptionsPage().catch(() => {}); if (!embedded) window.close(); });
  $("prompt-preview").addEventListener("input", event => {state.prompt = event.target.value;});
  document.addEventListener("keydown", event => {
    if (event.key === "Escape" && !event.defaultPrevented) closePanel();
  });
  window.addEventListener("message", event => {
    if (event.source !== parent) return;
    if (event.data?.type === "cinematch:compact-theme") {
      document.documentElement.dataset.theme = event.data.theme === "light" ? "light" : "dark";
    }
  });
  fetch(chrome.runtime.getURL("data/movements.json"))
    .then(response => response.ok ? response.json() : [])
    .then(catalog => {
      state.catalog = Array.isArray(catalog) ? catalog : [];
      renderChips();
      renderList();
      updateTabs();
      post("cinematch:compact-ready");
      $("search").focus();
      if (!embedded) chrome.tabs.query({url:"https://chat.deepseek.com/*"}).then(tabs => {
        if (!tabs.length) hint("Apri DeepSeek per inserire il prompt; puoi comunque copiarlo.");
      }).catch(() => {});
    })
    .catch(() => {
      $("empty").hidden = false;
      $("empty").textContent = "Correnti non disponibili.";
      status("Non riesco a caricare il catalogo.", true);
      post("cinematch:compact-ready");
    });
}
if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", load, {once:true});
else load();
