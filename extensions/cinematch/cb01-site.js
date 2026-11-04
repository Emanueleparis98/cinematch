/* CineMatch on cb01one.org. Adds a discreet Play control to film and TV
   posters, replaces the ad-heavy player block with a clean card, and asks the
   background to resolve the real video URL. The page never receives credentials. */
(() => {
  if (globalThis.cineMatchCb01Loaded) return;
  globalThis.cineMatchCb01Loaded = true;
  if (!/^https:\/\/(?:www\.)?cb01one\.org\//.test(location.href)) return;

  let enabled = true, observer = null, scanTimer = null;
  const pending = new Set();

  const text = value => String(value ?? '').replace(/\s+/g, ' ').trim();
  const safePoster = value => {
    try {
      const url = new URL(value, location.href);
      return url.protocol === 'https:' ? url.href : '';
    } catch { return ''; }
  };
  const cleanTitle = value => text(value)
    .replace(/\s+Streaming\s*[-–—].*$/i, '')
    .replace(/\s*[-–—]\s*(?:FILM|SERIE TV)\s+GRATIS.*$/i, '')
    .replace(/\s*[-–—]\s*Stagione\s+\d+.*$/i, '')
    .replace(/\s*[-–—]\s*\d+\s*[×x]\s*\d+.*$/i, '')
    .replace(/\s*\[(?:HD|4K|UHD)\]\s*/gi, ' ')
    .trim();
  const articleUrl = () => location.origin + location.pathname;
  const isTvArticle = () => /^\/serietv\//i.test(location.pathname) && !/^\/serietv\/(?:category|top-rated|page|feed)(?:\/|$)/i.test(location.pathname) && Boolean(document.querySelector('.sp-wrap, .sp-head'));

  function articleInfo() {
    const poster = document.querySelector('img.responsive-locandina');
    if (!poster) return null;
    const title = cleanTitle(poster.alt || document.querySelector('h1')?.textContent || document.title);
    return {title: title || 'Titolo', poster: safePoster(poster.currentSrc || poster.src), tv: isTvArticle()};
  }

  function articleKind(href) {
    try {
      const url = new URL(href, location.href);
      if (url.origin !== location.origin) return null;
      if (/^\/serietv\/[^/]+\/?$/i.test(url.pathname) && !/^\/serietv\/(?:category|top-rated|page|feed)(?:\/|$)/i.test(url.pathname)) return 'tv';
      if (/^\/[^/]+-(?:19|20)\d{2}\/?$/i.test(url.pathname)) return 'movie';
    } catch {}
    return null;
  }

  const icon = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z"/></svg>';
  const loadingTarget = trigger => trigger?.closest('a') ?? trigger?.closest('p') ?? trigger;

  function setLoading(trigger, value) {
    const target = loadingTarget(trigger);
    target?.classList?.toggle('cm-cb01-loading', value);
  }

  async function send(message) {
    try { return await chrome.runtime.sendMessage(message); }
    catch { return {ok:false, error:'CineMatch non è disponibile in questa pagina.'}; }
  }

  function openArticle(url, title, poster, options = {}) {
    const key = `${url}:${options.season ?? ''}:${options.episode ?? ''}`;
    if (pending.has(key)) return;
    pending.add(key);
    const {trigger = null} = options;
    const cinema = globalThis.CineMatchCinema;
    let opened = false;
    const attempt = async () => {
      setLoading(trigger, true);
      const response = await send({type:'cb01-resolve', articleUrl:url, season:options.season ?? null, episode:options.episode ?? null});
      setLoading(trigger, false);
      if (response?.ok) {
        if (opened) {
          if (cinema?.isOpen?.()) cinema.resolveDirect(response);
        } else if (response.token) {
          chrome.runtime.sendMessage({type:'cb01-open', token:response.token}).catch(() => {});
        }
      } else if (opened) {
        if (cinema?.isOpen?.()) cinema.failDirect(response?.error ?? 'Sorgente CB01 non disponibile.', {manualUrl:response?.manualUrl, code:response?.code});
      } else if (response?.manualUrl) {
        window.open(response.manualUrl, '_blank', 'noopener,noreferrer');
      }
      pending.delete(key);
    };
    const retry = () => { pending.delete(key); attempt(); };
    if (typeof cinema?.openDirect === 'function') {
      try { opened = cinema.openDirect({title, poster, posterEl:trigger, retry}) === true; }
      catch { opened = false; }
    }
    attempt();
  }

  function addPosterControl(info) {
    const poster = document.querySelector('img.responsive-locandina');
    const wrap = poster?.closest('.sequex-featured-img') || poster?.parentElement;
    if (!poster || !wrap || wrap.querySelector('.cm-cb01-poster-play')) return;
    wrap.classList.add('cm-cb01-poster-wrap');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'cm-cb01-poster-play';
    button.innerHTML = `${icon}<span>Guarda con CineMatch</span>`;
    button.setAttribute('aria-label', `Guarda ${info.title} con CineMatch`);
    button.addEventListener('click', event => {
      if (!event.isTrusted) return;
      event.preventDefault();
      event.stopPropagation();
      openArticle(articleUrl(), info.title, info.poster, {trigger:button});
    });
    poster.addEventListener('click', event => {
      if (!event.isTrusted || event.target.closest('a')) return;
      event.preventDefault();
      openArticle(articleUrl(), info.title, info.poster, {trigger:poster});
    });
    wrap.append(button);
  }

  function makeCard(info, mode) {
    const card = document.createElement('section');
    card.className = 'cm-cb01-card';
    card.setAttribute('data-cinematch-ui', 'cb01');
    card.setAttribute('aria-label', 'CineMatch');
    const mark = document.createElement('span');
    mark.className = 'cm-cb01-card-mark';
    mark.textContent = 'CM';
    const copy = document.createElement('div');
    copy.className = 'cm-cb01-card-copy';
    const title = document.createElement('strong');
    title.textContent = mode === 'tv' ? 'Guarda una serie con CineMatch' : 'Guarda con CineMatch';
    const subtitle = document.createElement('span');
    subtitle.textContent = mode === 'tv'
      ? 'Pubblicità bloccata · scegli stagione ed episodio'
      : 'Pubblicità bloccata · la sorgente migliore parte da sola';
    copy.append(title, subtitle);
    const actions = document.createElement('div');
    actions.className = 'cm-cb01-card-actions';
    const primary = document.createElement('button');
    primary.type = 'button';
    primary.className = 'cm-cb01-primary';
    primary.textContent = mode === 'tv' ? 'Scegli episodio' : 'Guarda ora';
    primary.addEventListener('click', event => {
      if (!event.isTrusted) return;
      event.preventDefault();
      openArticle(articleUrl(), info.title, info.poster, {trigger:primary});
    });
    const original = document.createElement('button');
    original.type = 'button';
    original.className = 'cm-cb01-ghost';
    original.textContent = 'Player originale';
    original.addEventListener('click', event => {
      if (!event.isTrusted) return;
      event.preventDefault();
      revealOriginal();
    });
    actions.append(primary, original);
    card.append(mark, copy, actions);
    return card;
  }

  function hideOriginal() {
    for (const node of document.querySelectorAll('#fancy-tabs, .tabs-catch-all')) node.classList.add('cm-cb01-original-hidden');
    for (const table of document.querySelectorAll('table.cbtable')) {
      if (/Guarda il Film Completo in HD/i.test(table.textContent)) table.classList.add('cm-cb01-original-hidden');
    }
    for (const iframe of document.querySelectorAll('.tabs-catch-all iframe')) {
      if (iframe.src && iframe.src !== 'about:blank' && !iframe.dataset.cmCb01Original) {
        iframe.dataset.cmCb01Original = iframe.src;
        iframe.src = 'about:blank';
      }
    }
  }

  function revealOriginal() {
    for (const node of document.querySelectorAll('.cm-cb01-original-hidden')) node.classList.remove('cm-cb01-original-hidden');
    for (const iframe of document.querySelectorAll('iframe[data-cm-cb01-original]')) {
      iframe.src = iframe.dataset.cmCb01Original;
      iframe.removeAttribute('data-cm-cb01-original');
    }
  }

  function decorateMovie(info) {
    const fancy = document.querySelector('#fancy-tabs') ?? document.querySelector('.tabs-catch-all');
    if (fancy && !document.querySelector('.cm-cb01-card')) {
      const card = makeCard(info, 'movie');
      fancy.parentElement?.insertBefore(card, fancy);
      hideOriginal();
    }
    for (const anchor of document.querySelectorAll('.tableinside a[href*="stayonline.pro"], .tableinside a[href*="uprot.net"]')) {
      anchor.dataset.cmCb01Source = '1';
    }
  }

  function tvEpisode(row) {
    const value = text(row.textContent).replace(/Ã—/g, '×');
    const match = value.match(/(\d+)\s*(?:×|x)\s*(\d+)/i);
    const season = row.closest('.sp-wrap')?.querySelector('.sp-head')?.textContent.match(/STAGIONE\s+(\d+)/i)?.[1];
    return match && season ? {season:Number(season), episode:Number(match[2])} : null;
  }

  function decorateTv(info) {
    const first = document.querySelector('.sp-wrap');
    if (!first) return;
    if (!document.querySelector('.cm-cb01-card')) {
      const holder = first.closest('table') ?? first;
      holder.parentElement?.insertBefore(makeCard(info, 'tv'), holder);
    }
    for (const row of document.querySelectorAll('.sp-body p')) {
      if (row.querySelector('.cm-cb01-episode-play')) continue;
      const target = tvEpisode(row);
      if (!target) continue;
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'cm-cb01-episode-play';
      button.innerHTML = icon;
      button.setAttribute('aria-label', `Guarda stagione ${target.season} episodio ${target.episode} con CineMatch`);
      button.addEventListener('click', event => {
        if (!event.isTrusted) return;
        event.preventDefault();
        event.stopPropagation();
        openArticle(articleUrl(), info.title, info.poster, {...target, trigger:button});
      });
      row.append(button);
    }
  }

  function decorateArticle() {
    const info = articleInfo();
    if (!info) return;
    addPosterControl(info);
    if (info.tv) decorateTv(info);
    else decorateMovie(info);
  }

  function decorateCards() {
    for (const anchor of document.querySelectorAll('a[href]')) {
      if (anchor.dataset.cmCb01Card || anchor.closest('[data-cinematch-ui]')) continue;
      const image = anchor.querySelector('img');
      if (!image || !image.alt || /CB01 OFFICIAL/i.test(image.alt)) continue;
      const kind = articleKind(anchor.href);
      if (!kind) continue;
      if (!anchor.classList.contains('wa_chpcs_post_link') && !anchor.closest('.rpwe-li, .post, article, .wa_featured_img')) {
        const width = image.naturalWidth || image.getBoundingClientRect().width;
        if (width && width < 100) continue;
      }
      const poster = safePoster(image.currentSrc || image.src);
      if (!poster) continue;
      anchor.dataset.cmCb01Card = '1';
      anchor.classList.add('cm-cb01-card-link');
      const badge = document.createElement('span');
      badge.className = 'cm-cb01-card-badge';
      badge.textContent = '▶';
      badge.setAttribute('aria-hidden', 'true');
      anchor.append(badge);
      anchor.addEventListener('click', event => {
        if (!event.isTrusted || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        if (!event.target.closest('img, .cm-cb01-card-badge')) return;
        event.preventDefault();
        event.stopPropagation();
        openArticle(anchor.href, cleanTitle(image.alt) || 'Titolo', poster, {trigger:anchor});
      }, true);
    }
  }

  function decorateAll() {
    if (!enabled) return;
    decorateCards();
    decorateArticle();
  }

  function schedule() {
    if (!enabled) return;
    clearTimeout(scanTimer);
    scanTimer = setTimeout(decorateAll, 180);
  }

  function startObserver() {
    if (observer) return;
    observer = new MutationObserver(mutations => {
      if (mutations.every(mutation => mutation.target instanceof Element && mutation.target.closest('[data-cinematch-ui], .cm-cb01-card, .cm-cb01-poster-play'))) return;
      schedule();
    });
    observer.observe(document.body, {childList:true, subtree:true});
  }

  function cleanup() {
    enabled = false;
    observer?.disconnect();
    observer = null;
    clearTimeout(scanTimer);
    for (const node of document.querySelectorAll('.cm-cb01-card, .cm-cb01-poster-play, .cm-cb01-card-badge, .cm-cb01-episode-play')) node.remove();
    for (const node of document.querySelectorAll('.cm-cb01-card-link, .cm-cb01-poster-wrap, .cm-cb01-original-hidden')) node.classList.remove('cm-cb01-card-link', 'cm-cb01-poster-wrap', 'cm-cb01-original-hidden');
    revealOriginal();
  }

  document.addEventListener('click', event => {
    if (!enabled || !event.isTrusted || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const anchor = event.target instanceof Element ? event.target.closest('.tableinside a[href*="stayonline.pro"], .tableinside a[href*="uprot.net"], .sp-body a[href*="stayonline.pro"], .sp-body a[href*="uprot.net"]') : null;
    if (!anchor) return;
    const info = articleInfo();
    if (!info) return;
    event.preventDefault();
    event.stopPropagation();
    const row = anchor.closest('.sp-body p');
    const target = row ? tvEpisode(row) : null;
    openArticle(articleUrl(), info.title, info.poster, {...(target ?? {}), trigger:anchor});
  }, true);

  chrome.runtime.onMessage.addListener(message => {
    if (message?.type === 'cinematch:cb01-disabled') {
      try { if (globalThis.CineMatchCinema?.isOpen?.()) globalThis.CineMatchCinema.close(); } catch {}
      window.dispatchEvent(new Event('cinematch:cb01-disabled'));
      cleanup();
      return;
    }
    if (message?.type === 'cinematch:cb01-enabled' && !enabled) {
      enabled = true;
      window.dispatchEvent(new Event('cinematch:cb01-enabled'));
      init();
      return;
    }
    if (message?.type === 'cinematch:cb01-enabled') schedule();
  });

  function init() {
    if (!document.body) return;
    decorateAll();
    startObserver();
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init, {once:true});
  else init();
})();
