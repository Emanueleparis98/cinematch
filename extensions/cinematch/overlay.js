/* CineMatch in-chat cinema shell: a native <dialog> in a shadow root that hosts
   the extension player page. The page never gets access to extension internals:
   the iframe is a chrome-extension:// document and messages are origin-checked. */
(() => {
  if (globalThis.CineMatchCinema) return;

  const HOST_ID = "cinematch-cinema-host";
  const EXTENSION_ORIGIN = new URL(chrome.runtime.getURL("")).origin;
  const REDUCED_MOTION = matchMedia("(prefers-reduced-motion: reduce)");

  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    :host{all:initial}
    .cm-dialog{position:fixed;inset:0;margin:auto;width:min(1180px,96vw);height:min(820px,92dvh);max-width:none;max-height:none;padding:0;border:1px solid rgba(255,255,255,.12);border-radius:24px;background:#0c111b;color:#eef3ff;box-shadow:0 40px 120px rgba(0,0,0,.66);overflow:hidden;font-family:Manrope,system-ui,-apple-system,"Segoe UI",sans-serif;opacity:0;transform:translateY(8px) scale(.985)}
    .cm-dialog[open]{animation:cm-in .34s cubic-bezier(.22,1,.36,1) forwards}
    .cm-dialog.closing{animation:cm-out .18s ease forwards}
    .cm-dialog[data-view="picker"]{width:min(940px,96vw);height:min(720px,92dvh)}
    .cm-dialog[data-view="addons"]{width:min(900px,96vw);height:min(760px,92dvh)}
    .cm-dialog::backdrop{background:rgba(3,6,11,.62);backdrop-filter:blur(18px) saturate(.8);animation:cm-fade .3s ease both}
    .cm-dialog.closing::backdrop{animation:cm-fade-out .18s ease forwards}
    .cm-frame{position:absolute;inset:0;display:block;width:100%;height:100%;border:0;background:#0c111b}
    .cm-frame[hidden]{display:none}
    .cm-close{position:absolute;top:10px;right:10px;z-index:6;width:36px;height:36px;display:grid;place-items:center;padding:0;border:1px solid rgba(255,255,255,.16);border-radius:10px;background:rgba(12,17,27,.74);backdrop-filter:blur(10px);color:#dfe7f5;font:400 22px/1 system-ui,sans-serif;cursor:pointer;transition:background .16s,border-color .16s}
    .cm-close:hover{background:rgba(28,38,56,.92);border-color:rgba(255,255,255,.28)}
    .cm-close:focus-visible{outline:2px solid #91b5ff;outline-offset:2px}
    .cm-loading{position:absolute;inset:0;z-index:3;display:grid;place-items:center;background:radial-gradient(circle at 50% 28%,#16243a,#080c13 72%);transition:opacity .24s ease}
    .cm-loading[hidden]{display:none}
    .cm-loading-card{display:flex;flex-direction:column;align-items:center;gap:10px;max-width:380px;padding:26px;text-align:center}
    .cm-loading-poster{width:104px;aspect-ratio:2/3;object-fit:cover;border-radius:10px;box-shadow:0 16px 38px rgba(0,0,0,.46);margin-bottom:5px}
    .cm-loading-poster[hidden]{display:none}
    .cm-kicker{margin:0;color:#8fa3c4;font-size:9px;font-weight:750;letter-spacing:.16em;text-transform:uppercase}
    .cm-loading h2{margin:0;color:#f1f5fb;font-size:20px;font-weight:800;letter-spacing:-.025em;line-height:1.2}
    .cm-loading-note{margin:0;color:#9aa7b9;font-size:11px;line-height:1.55}
    .cm-shimmer{position:relative;width:180px;height:4px;overflow:hidden;border-radius:99px;background:rgba(255,255,255,.08);margin-top:2px}
    .cm-shimmer::after{content:"";position:absolute;inset:0;background:linear-gradient(90deg,transparent,#7e9bff,transparent);transform:translateX(-100%);animation:cm-shimmer 1.2s infinite}
    .cm-fallback{position:absolute;inset:0;z-index:5;display:grid;place-items:center;padding:24px;background:rgba(8,12,19,.94)}
    .cm-fallback[hidden]{display:none}
    .cm-fallback-card{display:flex;flex-direction:column;align-items:center;gap:10px;max-width:360px;text-align:center}
    .cm-fallback-card p{margin:0;color:#9aa7b9;font-size:12px;line-height:1.6}
    .cm-fallback-card button{padding:9px 13px;border:1px solid #315fd1;border-radius:9px;background:#315fd1;color:#fff;font:700 12px/1.2 inherit;cursor:pointer}
    .cm-fallback-card button:hover{background:#3c6ee1}
    @keyframes cm-in{to{opacity:1;transform:none}}
    @keyframes cm-out{to{opacity:0;transform:translateY(6px) scale(.99)}}
    @keyframes cm-fade{from{opacity:0}to{opacity:1}}
    @keyframes cm-fade-out{to{opacity:0}}
    @keyframes cm-shimmer{to{transform:translateX(100%)}}
    @media(max-width:900px){.cm-dialog{width:100vw;height:100dvh;border:0;border-radius:0}}
    @media(prefers-reduced-motion:reduce){.cm-dialog,.cm-dialog[open],.cm-dialog.closing{animation:none;opacity:1;transform:none}.cm-dialog::backdrop,.cm-dialog.closing::backdrop{animation:none}.cm-shimmer::after{animation:none}}
  `);

  let host, shadow, dialog, frame, loading, fallback, closeButton, titleEl, posterEl, fallbackMessageEl, retryButton;
  let readyTimer, currentId = 0, currentKind = "movie", currentSeason = null, currentEpisode = null, lastFocus = null, previousOverflow = null, closing = false, activeTheme = null;
  let currentDirect = false, directRetry = null, directResult = null, directManualUrl = null, frameReady = false;
  let themeObserver = null, themeMedia = null, themeTimer = null;

  function ensureShell() {
    if (host) return;
    host = document.createElement("div");
    host.id = HOST_ID;
    host.setAttribute("data-cinematch-ui", "cinema");
    host.style.cssText = "all:initial";
    shadow = host.attachShadow({ mode: "open" });
    shadow.adoptedStyleSheets = [sheet];
    shadow.innerHTML = `
      <dialog class="cm-dialog" aria-modal="true" aria-labelledby="cm-cinema-title" tabindex="-1">
        <button class="cm-close" type="button" aria-label="Chiudi il player">×</button>
        <div class="cm-loading" role="status" aria-live="polite">
          <div class="cm-loading-card">
            <img class="cm-loading-poster" alt="" hidden>
            <p class="cm-kicker">CineMatch · Sorgenti</p>
            <h2 id="cm-cinema-title">Preparo il film…</h2>
            <div class="cm-shimmer" aria-hidden="true"></div>
            <p class="cm-loading-note">Cerco le sorgenti e preparo il player. La pagina resta qui dietro.</p>
          </div>
        </div>
        <iframe class="cm-frame" title="Player CineMatch" allow="fullscreen; autoplay; encrypted-media; picture-in-picture" hidden></iframe>
        <div class="cm-fallback" hidden>
          <div class="cm-fallback-card">
            <p class="cm-kicker">CineMatch</p>
            <h2 id="cm-cinema-fallback-title">Player non disponibile qui</h2>
            <p data-fallback-message>Il player non può essere aperto dentro questa pagina.</p>
            <button type="button" data-retry hidden>Riprova</button>
            <button type="button" data-open-tab>Apri in una scheda</button>
          </div>
        </div>
      </dialog>`;
    document.documentElement.append(host);

    dialog = shadow.querySelector("dialog");
    frame = shadow.querySelector("iframe");
    loading = shadow.querySelector(".cm-loading");
    fallback = shadow.querySelector(".cm-fallback");
    closeButton = shadow.querySelector(".cm-close");
    titleEl = shadow.querySelector("#cm-cinema-title");
    posterEl = shadow.querySelector(".cm-loading-poster");
    fallbackMessageEl = shadow.querySelector("[data-fallback-message]");
    retryButton = shadow.querySelector("[data-retry]");

    closeButton.addEventListener("click", close);
    dialog.addEventListener("cancel", event => {
      event.preventDefault();
      close();
    });
    dialog.addEventListener("click", event => {
      if (event.target === dialog) close();
    });
    dialog.addEventListener("close", cleanup);
    fallback.querySelector("[data-open-tab]").addEventListener("click", () => {
      if (currentDirect) {
        const token = directResult?.token;
        close();
        if (token) chrome.runtime.sendMessage({ type: "cb01-open", token }).catch(() => {});
        else if (directManualUrl) window.open(directManualUrl, "_blank", "noopener,noreferrer");
        return;
      }
      const id = currentId, kind = currentKind, season = currentSeason, episode = currentEpisode;
      close();
      if (id > 0) chrome.runtime.sendMessage({ type: "watch-open", id, kind, season, episode }).catch(() => {});
    });
    retryButton.addEventListener("click", () => {
      if (!currentDirect || typeof directRetry !== "function") return;
      fallback.hidden = true;
      loading.hidden = false;
      frame.hidden = false;
      clearTimeout(readyTimer);
      readyTimer = setTimeout(() => {
        if (dialog.open && !loading.hidden) showFallback("La sorgente sta impiegando troppo tempo. Riprova o apri la scheda.");
      }, 12000);
      directRetry();
    });
  }

  function cleanup() {
    stopThemeWatch();
    clearTimeout(readyTimer);
    readyTimer = null;
    closing = false;
    currentId = 0;
    currentKind = "movie";
    currentSeason = null;
    currentEpisode = null;
    currentDirect = false;
    directRetry = null;
    directResult = null;
    directManualUrl = null;
    frameReady = false;
    if (retryButton) retryButton.hidden = true;
    if (frame) frame.src = "about:blank";
    if (previousOverflow !== null) {
      document.documentElement.style.overflow = previousOverflow;
      previousOverflow = null;
    }
    const focusTarget = lastFocus;
    lastFocus = null;
    // Native dialog.close() restores focus after the close event, so defer our
    // restoration one frame to win over that default and return to the poster.
    if (focusTarget?.isConnected) requestAnimationFrame(() => {
      if (focusTarget.isConnected) focusTarget.focus({ preventScroll: true });
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
    if (!dialog?.open) return;
    clearTimeout(readyTimer);
    loading.hidden = true;
    frame.hidden = true;
    fallback.hidden = false;
    fallbackMessageEl.textContent = message || "Il player non può essere aperto dentro questa pagina. Puoi continuare nella scheda dedicata.";
    if (retryButton) retryButton.hidden = !(currentDirect && typeof directRetry === "function");
  }

  function chatTheme() {
    const accents = {
      "chat.deepseek.com": "#4d6bfe",
      "chatgpt.com": "#10a37f",
      "claude.ai": "#d97757",
      "gemini.google.com": "#4285f4"
    };
    let dark = matchMedia("(prefers-color-scheme: dark)").matches;
    const candidates = [document.querySelector("main"), document.body, document.documentElement].filter(Boolean);
    for (const candidate of candidates) {
      const color = getComputedStyle(candidate).backgroundColor;
      const values = color?.match(/\d+(?:\.\d+)?/g);
      if (values && values.length >= 3 && !/^rgba\([^)]*,\s*0\)$/.test(color)) {
        const [r, g, b] = values.map(Number);
        dark = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 < 0.52;
        break;
      }
    }
    return { theme: dark ? "dark" : "light", accent: accents[location.hostname] || "#7e9bff" };
  }

  function sendTheme() {
    if (!dialog?.open || !frame?.contentWindow) return;
    activeTheme = chatTheme();
    try { frame.contentWindow.postMessage({ type: "cinematch:theme", ...activeTheme }, EXTENSION_ORIGIN); } catch {}
  }

  function startThemeWatch() {
    stopThemeWatch();
    const targets = [document.documentElement, document.body, document.querySelector("main")].filter(Boolean);
    if (targets.length) {
      themeObserver = new MutationObserver(() => {
        clearTimeout(themeTimer);
        themeTimer = setTimeout(sendTheme, 120);
      });
      for (const target of targets) {
        themeObserver.observe(target, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-color-mode", "data-mode"] });
      }
    }
    themeMedia = matchMedia("(prefers-color-scheme: dark)");
    themeMedia.addEventListener?.("change", sendTheme);
  }

  function stopThemeWatch() {
    themeObserver?.disconnect();
    themeObserver = null;
    clearTimeout(themeTimer);
    themeTimer = null;
    themeMedia?.removeEventListener?.("change", sendTheme);
    themeMedia = null;
  }

  function open(options = {}) {
    const id = Number(options.id);
    if (!Number.isSafeInteger(id) || id < 1) return false;
    const kind = options.kind === "tv" ? "tv" : "movie";
    const season = kind === "tv" && Number.isSafeInteger(options.season) && options.season >= 0 ? options.season : null;
    const episode = kind === "tv" && Number.isSafeInteger(options.episode) && options.episode > 0 ? options.episode : null;
    ensureShell();
    const wasOpen = dialog.open;
    currentDirect = false;
    directRetry = null;
    directResult = null;
    directManualUrl = null;
    frameReady = false;
    retryButton.hidden = true;
    currentId = id;
    currentKind = kind;
    currentSeason = season;
    currentEpisode = episode;
    lastFocus = options.posterEl instanceof HTMLElement ? options.posterEl : (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    if (!wasOpen) {
      activeTheme = chatTheme();
      previousOverflow = document.documentElement.style.overflow;
      document.documentElement.style.overflow = "hidden";
      try { dialog.showModal(); } catch {
        document.documentElement.style.overflow = previousOverflow;
        previousOverflow = null;
        return false;
      }
      startThemeWatch();
    }
    closing = false;
    dialog.classList.remove("closing");
    dialog.dataset.view = "picker";
    loading.hidden = false;
    fallback.hidden = true;
    frame.hidden = true;
    frame.src = "about:blank";
    titleEl.textContent = options.title ? `Preparo ${options.title}…` : kind === "tv" ? "Preparo la serie…" : "Preparo il film…";
    if (options.poster) {
      posterEl.src = options.poster;
      posterEl.alt = options.title ? `Locandina di ${options.title}` : "";
      posterEl.hidden = false;
    } else {
      posterEl.removeAttribute("src");
      posterEl.alt = "";
      posterEl.hidden = true;
    }
    dialog.focus({ preventScroll: true });
    requestAnimationFrame(() => {
      if (!dialog.open || currentId !== id) return;
      const params = new URLSearchParams(kind === "tv" ? { kind: "tv", id: String(id) } : { movie: String(id) });
      if (kind === "tv" && season !== null) params.set("season", String(season));
      if (kind === "tv" && episode !== null) params.set("episode", String(episode));
      frame.src = chrome.runtime.getURL(`cinema.html?${params}`);
      frame.hidden = false;
      clearTimeout(readyTimer);
      readyTimer = setTimeout(() => {
        if (dialog.open && !loading.hidden) {
          showFallback("Il player sta impiegando troppo tempo ad avviarsi. Apri la scheda dedicata per continuare.");
        }
      }, 12000);
    });
    return true;
  }

  function sendDirect() {
    if (!frameReady || !directResult || !frame?.contentWindow) return;
    try {
      frame.contentWindow.postMessage({ type: "cinematch:direct", ...directResult }, EXTENSION_ORIGIN);
    } catch {}
  }

  function openDirect(options = {}) {
    ensureShell();
    const wasOpen = dialog.open;
    currentDirect = true;
    currentId = 0;
    currentKind = "movie";
    currentSeason = null;
    currentEpisode = null;
    directRetry = typeof options.retry === "function" ? options.retry : null;
    directResult = null;
    directManualUrl = null;
    frameReady = false;
    lastFocus = options.posterEl instanceof HTMLElement ? options.posterEl : (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    if (!wasOpen) {
      activeTheme = chatTheme();
      previousOverflow = document.documentElement.style.overflow;
      document.documentElement.style.overflow = "hidden";
      try { dialog.showModal(); } catch {
        document.documentElement.style.overflow = previousOverflow;
        previousOverflow = null;
        return false;
      }
      startThemeWatch();
    }
    closing = false;
    dialog.classList.remove("closing");
    dialog.dataset.view = "picker";
    loading.hidden = false;
    fallback.hidden = true;
    retryButton.hidden = true;
    frame.hidden = true;
    frame.src = "about:blank";
    titleEl.textContent = options.title ? `Preparo ${options.title}…` : "Preparo la riproduzione…";
    if (options.poster) {
      posterEl.src = options.poster;
      posterEl.alt = options.title ? `Locandina di ${options.title}` : "";
      posterEl.hidden = false;
    } else {
      posterEl.removeAttribute("src");
      posterEl.alt = "";
      posterEl.hidden = true;
    }
    dialog.focus({ preventScroll: true });
    requestAnimationFrame(() => {
      if (!dialog.open || !currentDirect) return;
      frame.src = chrome.runtime.getURL("cinema.html?direct=1");
      frame.hidden = false;
      clearTimeout(readyTimer);
      readyTimer = setTimeout(() => {
        if (dialog.open && !loading.hidden) {
          showFallback("La sorgente sta impiegando troppo tempo ad avviarsi. Riprova o apri la scheda dedicata.");
        }
      }, 12000);
    });
    return true;
  }

  function resolveDirect(result) {
    if (!dialog?.open || !currentDirect) return false;
    directResult = result;
    directManualUrl = result?.media?.key ?? null;
    fallback.hidden = true;
    if (frameReady) {
      loading.hidden = true;
      frame.hidden = false;
      sendDirect();
    }
    return true;
  }

  function failDirect(message, options = {}) {
    if (!dialog?.open || !currentDirect) return false;
    directManualUrl = options.manualUrl ?? directManualUrl;
    showFallback(typeof message === "string" ? message : undefined);
    return true;
  }

  window.addEventListener("message", event => {
    if (!dialog?.open || event.source !== frame?.contentWindow) return;
    if (event.origin !== EXTENSION_ORIGIN) return;
    const data = event.data;
    if (!data || typeof data.type !== "string") return;
    if (data.type === "cinematch:ready") {
      frameReady = true;
      clearTimeout(readyTimer);
      loading.hidden = true;
      fallback.hidden = true;
      sendTheme();
      if (currentDirect && directResult) sendDirect();
    } else if (data.type === "cinematch:view" && ["picker","player","addons"].includes(data.view)) {
      dialog.dataset.view = data.view;
    } else if (data.type === "cinematch:close") {
      close();
    } else if (data.type === "cinematch:error") {
      showFallback(typeof data.message === "string" ? data.message.slice(0, 300) : undefined);
    }
  });

  globalThis.CineMatchCinema = {
    open,
    openDirect,
    resolveDirect,
    failDirect,
    close,
    isOpen: () => Boolean(dialog?.open)
  };
})();
