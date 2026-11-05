/* CineMatch on DeepSeek: a compact currents popover anchored to a native-looking
   button beside DeepThink/Search. The full-screen panel and side panel are gone. */
(() => {
  if (globalThis.CineMatchDeepSeekCurrents) return;
  globalThis.CineMatchDeepSeekCurrents = true;

  const EXTENSION_ORIGIN = new URL(chrome.runtime.getURL("")).origin;
  const BUTTON_CLASS = "cm-deepseek-currents";
  const HOST_ID = "cinematch-currents-host";
  const REDUCED_MOTION = matchMedia("(prefers-reduced-motion: reduce)");

  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    :host{all:initial}
    .cm-currents-popover{position:fixed;z-index:2147483000;display:none;width:min(420px,calc(100vw - 24px));height:min(560px,calc(100vh - 120px));overflow:hidden;border:1px solid rgba(127,127,127,.32);border-radius:16px;background:#17181c;box-shadow:0 28px 80px rgba(0,0,0,.48);opacity:0;transform:translateY(6px) scale(.99)}
    .cm-currents-popover.open{display:block;animation:cm-currents-in .18s cubic-bezier(.22,1,.36,1) forwards}
    .cm-currents-popover.mobile{left:8px!important;right:8px!important;bottom:8px!important;width:auto!important;height:min(72vh,560px)!important;border-radius:14px}
    .cm-currents-frame{display:block;width:100%;height:100%;border:0;background:transparent}
    .cm-currents-frame[hidden]{display:none}
    .cm-currents-loading{position:absolute;inset:0;z-index:2;display:grid;place-items:center;background:#17181c;color:#9ca1ad;font:12px/1.4 system-ui,sans-serif}
    .cm-currents-loading[hidden]{display:none}
    .cm-currents-spinner{width:22px;height:22px;border:2px solid rgba(127,127,127,.3);border-top-color:#5b7cfa;border-radius:50%;animation:cm-currents-spin .8s linear infinite;margin-bottom:8px}
    @keyframes cm-currents-in{to{opacity:1;transform:none}}
    @keyframes cm-currents-spin{to{transform:rotate(360deg)}}
    @media(prefers-reduced-motion:reduce){.cm-currents-popover.open{animation:none;opacity:1;transform:none}.cm-currents-spinner{animation:none}}
  `);

  let host, shadow, popover, frame, loading, inlineButton = null;
  let enabled = false, observer = null, scanTimer = null, lastFocus = null;
  let themeObserver = null, themeTimer = null;

  const icon = '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="5" width="18" height="14" rx="3"/><path d="M10 9l5 3-5 3z"/></svg>';
  const visible = element => Boolean(element?.getClientRects().length) && getComputedStyle(element).visibility !== "hidden" && !element.closest('[aria-hidden="true"], [inert]');

  function ensureShell() {
    if (host) return;
    host = document.createElement("div");
    host.id = HOST_ID;
    host.setAttribute("data-cinematch-ui", "deepseek-currents");
    host.style.cssText = "all:initial";
    shadow = host.attachShadow({mode:"open"});
    shadow.adoptedStyleSheets = [sheet];
    shadow.innerHTML = `
      <div class="cm-currents-popover" role="dialog" aria-label="Correnti CineMatch">
        <div class="cm-currents-loading"><div class="cm-currents-spinner" aria-hidden="true"></div><span>Preparo le correnti…</span></div>
        <iframe class="cm-currents-frame" title="CineMatch · Correnti" allow="clipboard-write" hidden></iframe>
      </div>`;
    document.documentElement.append(host);
    popover = shadow.querySelector(".cm-currents-popover");
    frame = shadow.querySelector(".cm-currents-frame");
    loading = shadow.querySelector(".cm-currents-loading");
    window.addEventListener("message", message => {
      if (!popover.classList.contains("open") || message.source !== frame?.contentWindow || message.origin !== EXTENSION_ORIGIN) return;
      const data = message.data;
      if (!data || typeof data.type !== "string") return;
      if (data.type === "cinematch:compact-ready") {
        loading.hidden = true;
        frame.hidden = false;
        sendTheme();
      } else if (data.type === "cinematch:compact-close") close();
    });
    document.addEventListener("pointerdown", event => {
      if (!popover.classList.contains("open")) return;
      if (host.contains(event.target) || inlineButton === event.target || inlineButton?.contains(event.target)) return;
      close();
    }, true);
    window.addEventListener("resize", position);
    window.addEventListener("scroll", position, true);
  }

  function deepseekTheme() {
    let dark = !matchMedia("(prefers-color-scheme: light)").matches;
    for (const candidate of [document.querySelector("main"), document.body, document.documentElement]) {
      if (!candidate) continue;
      const color = getComputedStyle(candidate).backgroundColor;
      const values = color?.match(/\d+(?:\.\d+)?/g);
      if (values && values.length >= 3 && !/^rgba\([^)]*,\s*0\)$/.test(color)) {
        const [r,g,b] = values.map(Number);
        dark = (0.2126*r + 0.7152*g + 0.0722*b) / 255 < 0.52;
        break;
      }
    }
    return dark ? "dark" : "light";
  }
  function sendTheme() {
    if (!popover?.classList.contains("open") || !frame?.contentWindow) return;
    try { frame.contentWindow.postMessage({type:"cinematch:compact-theme", theme:deepseekTheme()}, EXTENSION_ORIGIN); } catch {}
  }
  function startThemeWatch() {
    stopThemeWatch();
    const targets = [document.documentElement, document.body, document.querySelector("main")].filter(Boolean);
    if (targets.length) {
      themeObserver = new MutationObserver(() => {
        clearTimeout(themeTimer);
        themeTimer = setTimeout(sendTheme, 120);
      });
      for (const target of targets) themeObserver.observe(target, {attributes:true, attributeFilter:["class","style","data-theme","data-color-mode","data-mode"]});
    }
  }
  function stopThemeWatch() {
    themeObserver?.disconnect();
    themeObserver = null;
    clearTimeout(themeTimer);
    themeTimer = null;
  }

  function position() {
    if (!popover?.classList.contains("open") || !inlineButton?.isConnected) return;
    const rect = inlineButton.getBoundingClientRect();
    if (innerWidth <= 600) {
      popover.classList.add("mobile");
      popover.style.left = "";
      popover.style.bottom = "";
      return;
    }
    popover.classList.remove("mobile");
    const width = Math.min(420, innerWidth - 24);
    const height = Math.min(560, innerHeight - 120);
    popover.style.width = `${width}px`;
    popover.style.height = `${height}px`;
    popover.style.left = `${Math.max(12, Math.min(rect.left, innerWidth - width - 12))}px`;
    popover.style.bottom = `${Math.max(12, innerHeight - rect.top + 10)}px`;
  }

  function open() {
    if (!enabled || !inlineButton?.isConnected) return;
    ensureShell();
    if (popover.classList.contains("open")) return;
    lastFocus = inlineButton;
    loading.hidden = false;
    frame.hidden = true;
    frame.src = "about:blank";
    popover.classList.add("open");
    inlineButton.setAttribute("aria-expanded", "true");
    inlineButton.classList.add("ds-toggle-button--selected");
    position();
    startThemeWatch();
    requestAnimationFrame(() => {
      frame.src = chrome.runtime.getURL("compact.html?view=popover");
      frame.hidden = false;
    });
  }

  function close() {
    if (!popover?.classList.contains("open")) return;
    popover.classList.remove("open");
    inlineButton?.setAttribute("aria-expanded", "false");
    inlineButton?.classList.remove("ds-toggle-button--selected");
    frame.src = "about:blank";
    frame.hidden = true;
    loading.hidden = false;
    stopThemeWatch();
    const focusTarget = lastFocus;
    lastFocus = null;
    if (focusTarget?.isConnected) requestAnimationFrame(() => {
      if (focusTarget.isConnected) focusTarget.focus({preventScroll:true});
    });
  }

  function findComposer() {
    const preferred = document.querySelector("textarea#chat-input");
    const editor = preferred && visible(preferred) ? preferred : [...document.querySelectorAll("textarea,[contenteditable='true']")].find(visible);
    if (!editor) return null;
    const all = [...document.querySelectorAll(".ds-toggle-button")].filter(element => !element.classList.contains(BUTTON_CLASS) && visible(element));
    if (!all.length) return null;
    const rect = editor.getBoundingClientRect();
    const nearby = all.filter(element => {
      const box = element.getBoundingClientRect();
      return Math.abs(box.bottom - rect.bottom) < 260 || Math.abs(box.top - rect.top) < 260;
    });
    const candidates = nearby.length ? nearby : all;
    return {editor, toggles:candidates.slice(0, 2)};
  }

  function createInlineButton() {
    const button = document.createElement("div");
    button.className = `ds-toggle-button ds-toggle-button--m ${BUTTON_CLASS}`;
    button.setAttribute("role", "button");
    button.tabIndex = 0;
    button.setAttribute("aria-label", "Apri le correnti CineMatch");
    button.setAttribute("aria-haspopup", "dialog");
    button.setAttribute("aria-expanded", "false");
    button.title = "Correnti CineMatch";
    button.innerHTML = `<div class="ds-toggle-button__icon">${icon}</div><span class="cm-deepseek-currents-label">Correnti</span>`;
    button.addEventListener("click", event => {
      event.preventDefault();
      event.stopPropagation();
      if (popover?.classList.contains("open")) close();
      else open();
    });
    button.addEventListener("keydown", event => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      event.stopPropagation();
      if (popover?.classList.contains("open")) close();
      else open();
    });
    return button;
  }

  function scan() {
    if (!enabled) return;
    const composer = findComposer();
    if (!composer?.toggles.length) {
      if (popover?.classList.contains("open")) close();
      inlineButton?.remove();
      inlineButton = null;
      return;
    }
    const anchor = composer.toggles[composer.toggles.length - 1];
    const row = anchor.parentElement;
    if (!row) return;
    if (inlineButton?.isConnected && inlineButton.parentElement === row && inlineButton.previousElementSibling === anchor) return;
    const wasOpen = Boolean(popover?.classList.contains("open"));
    inlineButton?.remove();
    inlineButton = createInlineButton();
    anchor.after(inlineButton);
    if (wasOpen) {
      lastFocus = inlineButton;
      inlineButton.setAttribute("aria-expanded", "true");
      inlineButton.classList.add("ds-toggle-button--selected");
      position();
    }
  }

  function schedule() {
    if (!enabled) return;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(scan, 120);
  }
  function startObserver() {
    if (observer) return;
    observer = new MutationObserver(() => schedule());
    observer.observe(document.body, {childList:true, subtree:true});
    schedule();
  }
  function stopObserver() {
    observer?.disconnect();
    observer = null;
    clearTimeout(scanTimer);
  }
  function setEnabled(value) {
    const next = value === true;
    if (enabled === next) {
      if (next) schedule();
      return;
    }
    enabled = next;
    if (!enabled) {
      stopObserver();
      close();
      inlineButton?.remove();
      inlineButton = null;
      return;
    }
    if (document.body) startObserver();
    else document.addEventListener("DOMContentLoaded", startObserver, {once:true});
  }

  function requestState() {
    chrome.runtime.sendMessage({type:"deepseek-currents-status"})
      .then(result => setEnabled(result?.enabled === true))
      .catch(() => {});
  }
  chrome.runtime.onMessage.addListener(message => {
    if (message?.type === "cinematch:deepseek-currents") setEnabled(message.enabled === true);
  });
  requestState();

  globalThis.CineMatchDeepSeekCurrentsApi = {open, close, isOpen:() => Boolean(popover?.classList.contains("open"))};
})();
