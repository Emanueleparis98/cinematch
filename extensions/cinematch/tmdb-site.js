/* CineMatch on themoviedb.org: a small, optional stream button on movie and TV posters.
   The content script only reads the visible TMDB link, title and poster. It never
   reads account data and never calls TMDB itself. */
(() => {
  if (globalThis.cineMatchTmdbSiteLoaded) return;
  globalThis.cineMatchTmdbSiteLoaded = true;
  const BUTTON = 'cm-tmdb-stream-button';
  let configured = false, enabled = true, observer = null, scanTimer = null;
  const schedule = () => {
    if (!configured || !enabled) return;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(scan, 180);
  };
  function mediaFromPath(value, base = location.origin) {
    let url;
    try { url = new URL(value, base); } catch { return null; }
    if (url.origin !== location.origin) return null;
    const match = url.pathname.match(/^\/(movie|tv)\/(\d+)(?:-[^/]*)?(?:\/season\/(\d+)\/episode\/(\d+))?/);
    if (!match) return null;
    return {
      kind: match[1],
      id: Number(match[2]),
      season: match[3] === undefined ? null : Number(match[3]),
      episode: match[4] === undefined ? null : Number(match[4])
    };
  }
  function mediaFromAnchor(anchor) {
    const media = mediaFromPath(anchor.getAttribute('href') || '');
    if (!media) return null;
    const image = anchor.querySelector('img');
    if (!image) return null;
    const poster = image.currentSrc || image.src;
    if (!poster) return null;
    return {...media, title: (image.alt || anchor.getAttribute('aria-label') || anchor.getAttribute('title') || '').trim(), poster};
  }
  function openMedia(media, trigger) {
    if (globalThis.CineMatchCinema?.open({...media, posterEl: trigger}) === true) return;
    chrome.runtime.sendMessage({type: 'watch-open', ...media}).catch(() => {});
  }
  function install(host, media) {
    if (!(host instanceof HTMLElement) || host.tagName === 'IMG') return;
    const key = `${media.kind}:${media.id}:${media.season ?? ''}:${media.episode ?? ''}`;
    const existing = host.querySelector(`:scope > .${BUTTON}`);
    if (existing?.dataset.cmMedia === key) return;
    existing?.remove();
    host.classList.add('cm-tmdb-poster-host');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = BUTTON;
    button.dataset.cmMedia = key;
    button.title = `Cerca i flussi per ${media.title || 'questo titolo'}`;
    button.setAttribute('aria-label', `Cerca i flussi per ${media.title || 'questo titolo'}`);
    button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z"/></svg>Flussi';
    button.addEventListener('click', event => {
      if (!event.isTrusted) return;
      event.preventDefault();
      event.stopPropagation();
      openMedia(media, button);
    });
    host.append(button);
  }
  function scan() {
    if (!configured || !enabled) return;
    for (const anchor of document.querySelectorAll('a[href*="/movie/"], a[href*="/tv/"]')) {
      const media = mediaFromAnchor(anchor);
      if (media?.title) install(anchor.parentElement, media);
    }
    const page = mediaFromPath(location.pathname);
    if (!page) return;
    const poster = document.querySelector('.images.inner img.poster, .poster_wrapper img.poster');
    if (!poster) return;
    const image = poster.currentSrc || poster.src;
    if (!image) return;
    const title = (poster.alt || document.querySelector('.images.inner h2, .poster_wrapper h2')?.textContent || '').trim();
    if (!title) return;
    const host = poster.parentElement?.closest('.poster') || poster.parentElement;
    install(host, {...page, title, poster: image});
  }
  function start() {
    if (!configured || !enabled || observer) return;
    observer = new MutationObserver(mutations => {
      if (mutations.every(mutation => mutation.target instanceof Element && mutation.target.closest(`.${BUTTON}`))) return;
      schedule();
    });
    observer.observe(document.body, {childList: true, subtree: true, attributes: true, attributeFilter: ['src', 'srcset', 'alt', 'href']});
    schedule();
  }
  function stop() {
    observer?.disconnect();
    observer = null;
    clearTimeout(scanTimer);
    for (const button of document.querySelectorAll(`.${BUTTON}`)) button.remove();
    for (const host of document.querySelectorAll('.cm-tmdb-poster-host')) host.classList.remove('cm-tmdb-poster-host');
  }
  async function refreshStatus() {
    try {
      const result = await chrome.runtime.sendMessage({type: 'tmdb-site-configured'});
      configured = Boolean(result?.configured);
    } catch { configured = false; }
    if (configured) start(); else stop();
  }
  window.addEventListener('popstate', schedule);
  window.addEventListener('hashchange', schedule);
  chrome.runtime.onMessage.addListener(message => {
    if (message?.type === 'tmdb-preferences-changed') {
      configured = Boolean(message.configured);
      if (configured) start(); else stop();
    } else if (message?.type === 'cinematch:tmdb-site-enabled') {
      enabled = true; start();
    } else if (message?.type === 'cinematch:tmdb-site-disabled') {
      enabled = false;
      try { if (globalThis.CineMatchCinema?.isOpen()) globalThis.CineMatchCinema.close(); } catch {}
      stop();
    }
  });
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', refreshStatus, {once: true});
  else refreshStatus();
})();
