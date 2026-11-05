/* CineMatch on DeepSeek: an inline "Correnti" control beside the native
   DeepThink/Search toggles. It opens the existing CineMatch panel in a dialog.
   The floating launcher and the side panel are intentionally gone. */
(() => {
  if (globalThis.CineMatchDeepSeekCurrents) return;
  globalThis.CineMatchDeepSeekCurrents = true;

  const EXTENSION_ORIGIN = new URL(chrome.runtime.getURL("")).origin;
  const REDUCED_MOTION = matchMedia("(prefers-reduced-motion: reduce)");
  const BUTTON_CLASS = "cm-deepseek-currents";
  const HOST_ID = "cinematch-currents-host";

  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    :host{all:initial}
    .cm-prompt-dialog{position:fixed;inset:0;margin:auto;width:min(1080px,96vw);height:min(820px,92dvh);max-width:none;max-height:none;padding:0;border:1px solid rgba(255,255,255,.12);border-radius:24px;background:#0c111b;color:#eef3ff;box-shadow:0 44px 130px rgba(0,0,0,.66);overflow:hidden;opacity:0;transform:translateY(8px) scale(.985)}
    .cm-prompt-dialog[open]{animation:cm-prompt-in .34s cubic-bezier(.22,1,.36,1) forwards}
    .cm-prompt-dialog.closing{animation:cm-prompt-out .18s ease forwards}
    .cm-prompt-dialog::backdrop{background:rgba(3,6,11,.62);backdrop-filter:blur(18px) saturate(.8);animation:cm-prompt-fade .3s ease both}
    .cm-prompt-dialog.closing::backdrop{animation:cm-prompt-fade-out .18s ease forwards}
    .cm-prompt-dialog:focus{outline:none}
    .cm-prompt-frame{position:absolute;inset:0;display:block;width:100%;height:100%;border:0;background:#0c111b}
    .cm-prompt-frame[hidden]{display:none}
    .cm-prompt-close{position:absolute;top:10px;right:10px;z-index:6;width:36px;height:36px;display:grid;place-items:center;padding:0;border:1px solid rgba(255,255,255,.16);border-radius:10px;background:rgba(12,17,27,.74);backdrop-filter:blur(10px);color:#dfe7f5;font:400 22px/1 system-ui,sans-serif;cursor:pointer;transition:background .16s,border-color .16s}
    .cm-prompt-close:hover{background:rgba(28,38,56,.92);border-color:rgba(255,255,255,.28)}
    .cm-prompt-close:focus-visible{outline:2px solid #91b5ff;outline-offset:2px}
    .cm-prompt-loading{position:absolute;inset:0;z-index:3;display:grid;place-items:center;background:radial-gradient(circle at 50% 28%,#16243a,#080c13 72%);transition:opacity .24s ease}
    .cm-prompt-loading[hidden]{display:none}
    .cm-prompt-loading-card{display:flex;flex-direction:column;align-items:center;gap:10px;max-width:380px;padding:26px;text-align:center}
    .cm-prompt-mark{display:flex;align-items:center;gap:8px;color:#9cbbff;font:800 15px/1 Manrope,system-ui,sans-serif;letter-spacing:-.03em}
    .cm-prompt-mark svg{width:25px;height:25px;fill:none;stroke:currentColor;stroke-width:1.8}
    .cm-prompt-kicker{margin:2px 0 0;color:#8fa3c4;font-size:9px;font-weight:750;letter-spacing:.16em;text-transform:uppercase}
    .cm-prompt-loading h2{margin:0;color:#f1f5fb;font-size:20px;font-weight:800;letter-spacing:-.025em;line-height:1.2}
    .cm-prompt-loading-note{margin:0;color:#9aa7b9;font-size:11px;line-height:1.55}
    .cm-prompt-shimmer{position:relative;width:180px;height:4px;overflow:hidden;border-radius:99px;background:rgba(255,255,255,.08);margin-top:2px}
    .cm-prompt-shimmer::after{content:"";position:absolute;inset:0;background:linear-gradient(90deg,transparent,#7e9bff,transparent);transform:translateX(-100%);animation:cm-prompt-shimmer 1.2s infinite}
    .cm-prompt-fallback{position:absolute;inset:0;z-index:5;display:grid;place-items:center;padding:24px;background:rgba(8,12,19,.94)}
    .cm-prompt-fallback[hidden]{display:none}
    .cm-prompt-fallback-card{display:flex;flex-direction:column;align-items:center;gap:10px;max-width:360px;text-align:center}
    .cm-prompt-fallback-card p{margin:0;color:#9aa7b9;font-size:12px;line-height:1.6}
    .cm-prompt-fallback-card button{padding:9px 13px;border:1px solid #315fd1;border-radius:9px;background:#315fd1;color:#fff;font:700 12px/1.2 Manrope,system-ui,sans-serif;cursor:pointer}
    .cm-prompt-fallback-card button:hover{background:#3c6ee1}
    @keyframes cm-prompt-in{to{opacity:1;transform:none}}
    @keyframes cm-prompt-out{to{opacity:0;transform:translateY(6px) scale(.99)}}
    @keyframes cm-prompt-fade{from{opacity:0}to{opacity:1}}
    @keyframes cm-prompt-fade-out{to{opacity:0}}
    @keyframes cm-prompt-shimmer{to{transform:translateX(100%)}}
    @media(max-width:900px){.cm-prompt-dialog{width:100vw;height:100dvh;border:0;border-radius:0}}
    @media(prefers-reduced-motion:reduce){.cm-prompt-dialog,.cm-prompt-dialog[open],.cm-prompt-dialog.closing{animation:none;opacity:1;transform:none}.cm-prompt-dialog::backdrop,.cm-prompt-dialog.closing::backdrop,.cm-prompt-shimmer::after{animation:none}}
  `);

  let host, shadow, dialog, frame, loading, fallback, closeButton, readyTimer, lastFocus, previousOverflow, closing = false;
  let inlineButton = null, enabled = false, observer = null, scanTimer = null;

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
      <dialog class="cm-prompt-dialog" aria-modal="true" aria-labelledby="cm-prompt-title" tabindex="-1">
        <button class="cm-prompt-close" type="button" aria-label="Chiudi CineMatch">×</button>
        <div class="cm-prompt-loading" role="status" aria-live="polite">
          <div class="cm-prompt-loading-card">
            <div class="cm-prompt-mark">${icon}<span>CineMatch</span></div>
            <p class="cm-prompt-kicker">Correnti e prompt</p>
            <h2 id="cm-prompt-title">Preparo il catalogo…</h2>
            <div class="cm-prompt-shimmer" aria-hidden="true"></div>
            <p class="cm-prompt-loading-note">74 correnti, filtri e generatore di prompt restano sul tuo browser.</p>
          </div>
        </div>
        <iframe class="cm-prompt-frame" title="CineMatch · Correnti e prompt" allow="clipboard-write" hidden></iframe>
        <div class="cm-prompt-fallback" hidden>
          <div class="cm-prompt-fallback-card">
            <div class="cm-prompt-mark">${icon}<span>CineMatch</span></div>
            <h2>Pannello non disponibile qui</h2>
            <p>Puoi aprire CineMatch in una scheda dedicata e usare correnti e prompt da lì.</p>
            <button type="button" data-open-panel>Apri in una scheda</button>
          </div>
        </div>
      </dialog>`;
    document.documentElement.append(host);
    dialog = shadow.querySelector("dialog");
    frame = shadow.querySelector("iframe");
    loading = shadow.querySelector(".cm-prompt-loading");
    fallback = shadow.querySelector(".cm-prompt-fallback");
    closeButton = shadow.querySelector(".cm-prompt-close");
    closeButton.addEventListener("click", close);
    dialog.addEventListener("cancel", event => {event.preventDefault();close();});
    dialog.addEventListener("click", event => {if (event.target === dialog) close();});
    dialog.addEventListener("close", cleanup);
    fallback.querySelector("[data-open-panel]").addEventListener("click", () => {
      chrome.runtime.sendMessage({type:"open-panel"}).catch(() => {});
      close();
    });
    window.addEventListener("message", event => {
      if (!dialog.open || event.source !== frame?.contentWindow || event.origin !== EXTENSION_ORIGIN) return;
      const data = event.data;
      if (!data || typeof data.type !== "string") return;
      if (data.type === "cinematch:prompt-ready") {
        clearTimeout(readyTimer);
        loading.hidden = true;
        fallback.hidden = true;
      } else if (data.type === "cinematch:prompt-close") close();
    });
  }

  function setButtonState(open) {
    if (!inlineButton) return;
    inlineButton.setAttribute("aria-expanded", String(open));
    inlineButton.classList.toggle("ds-toggle-button--selected", open);
  }

  function cleanup() {
    clearTimeout(readyTimer);
    readyTimer = null;
    closing = false;
    setButtonState(false);
    if (frame) frame.src = "about:blank";
    if (previousOverflow !== null) {
      document.documentElement.style.overflow = previousOverflow;
      previousOverflow = null;
    }
    const focusTarget = lastFocus;
    lastFocus = null;
    if (focusTarget?.isConnected) requestAnimationFrame(() => {
      if (focusTarget.isConnected) focusTarget.focus({preventScroll:true});
    });
  }

  function close() {
    if (!dialog?.open || closing) return;
    closing = true;
    dialog.classList.add("closing");
    const finish = () => {
      if (dialog.open) dialog.close();
      dialog.classList.remove("closing");
      cleanup();
    };
    if (REDUCED_MOTION.matches) finish();
    else setTimeout(finish, 190);
  }

  function showFallback(message) {
    clearTimeout(readyTimer);
    loading.hidden = true;
    frame.hidden = true;
    fallback.hidden = false;
    const note = fallback.querySelector("p");
    if (message && note) note.textContent = message;
  }

  function open() {
    if (!enabled || !inlineButton?.isConnected || dialog?.open) return;
    ensureShell();
    lastFocus = inlineButton;
    previousOverflow = document.documentElement.style.overflow;
    document.documentElement.style.overflow = "hidden";
    loading.hidden = false;
    fallback.hidden = true;
    frame.hidden = true;
    frame.src = "about:blank";
    setButtonState(true);
    try {dialog.showModal();} catch {
      document.documentElement.style.overflow = previousOverflow;
      previousOverflow = null;
      setButtonState(false);
      return;
    }
    dialog.focus({preventScroll:true});
    requestAnimationFrame(() => {
      if (!dialog.open) return;
      frame.src = chrome.runtime.getURL("panel.html?view=embed");
      frame.hidden = false;
      clearTimeout(readyTimer);
      readyTimer = setTimeout(() => {
        if (dialog.open && !loading.hidden) showFallback("Il pannello sta impiegando troppo tempo ad avviarsi.");
      }, 12000);
    });
  }

  function findComposer() {
    const preferred = document.querySelector("textarea#chat-input");
    const textarea = preferred && visible(preferred) ? preferred
      : [...document.querySelectorAll("textarea,[contenteditable='true']")].find(visible);
    if (!textarea) return null;
    const all = [...document.querySelectorAll(".ds-toggle-button")].filter(element => !element.classList.contains(BUTTON_CLASS) && visible(element));
    if (!all.length) return null;
    const editorRect = textarea.getBoundingClientRect();
    const nearby = all.filter(element => {
      const rect = element.getBoundingClientRect();
      return Math.abs(rect.bottom - editorRect.bottom) < 260 || Math.abs(rect.top - editorRect.top) < 260;
    });
    const candidates = nearby.length ? nearby : all;
    return {textarea, toggles:candidates.slice(0, 2)};
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
      open();
    });
    button.addEventListener("keydown", event => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      event.stopPropagation();
      open();
    });
    return button;
  }

  function scan() {
    if (!enabled) return;
    const composer = findComposer();
    if (!composer?.toggles.length) {
      inlineButton?.remove();
      inlineButton = null;
      return;
    }
    const anchor = composer.toggles[composer.toggles.length - 1];
    const row = anchor.parentElement;
    if (!row) return;
    if (inlineButton?.isConnected && inlineButton.parentElement === row && inlineButton.previousElementSibling === anchor) return;
    inlineButton?.remove();
    inlineButton = createInlineButton();
    anchor.after(inlineButton);
    setButtonState(Boolean(dialog?.open));
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
      inlineButton?.remove();
      inlineButton = null;
      if (dialog?.open) close();
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

  globalThis.CineMatchDeepSeekCurrentsApi = {open, close, isOpen:() => Boolean(dialog?.open)};
})();
