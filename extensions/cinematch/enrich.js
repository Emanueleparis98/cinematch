/* Seamless media for AI answers. No controls, forms or settings are injected. */
(() => {
  if (globalThis.cinematchEnrichmentLoaded) return;
  globalThis.cinematchEnrichmentLoaded = true;
  const selectors = {
    'chatgpt.com': ['[data-message-author-role="assistant"]'],
    'chat.deepseek.com': ['.ds-markdown', '[data-role="assistant"]'],
    'claude.ai': ['[data-testid="assistant-message"]', '.font-claude-response'],
    'gemini.google.com': ['model-response message-content', 'model-response .model-response-text', 'model-response']
  }[location.hostname];
  if (!selectors) return;
  const {extract, normalize} = globalThis.CineMatchReferences;
  const configuration = {ready: false, configured: false};
  const roots = new Map(), anchors = new Map();
  const imageCache = new Map(), imagePending = new Map();
  let imageBytes = 0, scanTimer;
  const element = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  async function send(message) {
    const result = await chrome.runtime.sendMessage(message);
    if (!result?.ok) throw new Error(result?.error ?? 'TMDB non disponibile.');
    return result;
  }
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    :host{display:block;container-type:inline-size;margin:12px 0 22px;color:inherit;font-family:inherit;line-height:1.4;clear:both}
    :host([hidden]){display:none}*{box-sizing:border-box}.media{display:grid;grid-template-columns:108px minmax(0,1fr);gap:24px;align-items:start;margin:0;animation:arrive 180ms cubic-bezier(.22,1,.36,1)}
    .media.no-poster{grid-template-columns:1fr}.poster-wrap{position:relative;width:108px;flex:none;line-height:0}.poster-wrap>a{display:block;line-height:0;font-size:0}.poster{display:block;width:108px;aspect-ratio:2/3;object-fit:cover;border-radius:11px;background:rgba(127,127,127,.12)}
    .streams-button{position:absolute;right:6px;bottom:6px;z-index:2;display:inline-flex;align-items:center;gap:4px;padding:5px 8px;border:1px solid rgba(255,255,255,.5);border-radius:999px;background:rgba(8,12,19,.84);color:#fff;font-family:inherit;font-size:9px;line-height:1;font-weight:700;letter-spacing:.01em;cursor:pointer;backdrop-filter:blur(7px);box-shadow:0 4px 14px rgba(0,0,0,.28)}.streams-button:hover,.streams-button:focus-visible{background:#315fd1;border-color:#9cbbff;color:#fff}.streams-button svg{width:9px;height:9px;fill:currentColor;flex:none}
    a{color:inherit;text-decoration:none;min-width:0}a:focus-visible{outline:2px solid #3878ed;outline-offset:5px;border-radius:8px}a:hover .name{text-decoration:underline;text-underline-offset:3px}
    .people{display:flex;flex-wrap:wrap;gap:18px 16px;align-items:start}.person{display:flex;flex-direction:column;width:104px;flex:none;font-size:12px}.portrait{width:76px;height:100px;object-fit:cover;object-position:center 25%;border-radius:9px;background:rgba(127,127,127,.12)}
    .name{font-size:12px;font-weight:550;line-height:1.45;margin-top:7px;overflow-wrap:normal;word-break:normal}.role{font-size:11px;line-height:1.4;opacity:.76;margin-top:2px}.source{display:inline-block;margin-top:9px;font-size:11px;letter-spacing:.01em;opacity:.76;text-decoration:underline;text-underline-offset:3px}
    @container(max-width:430px){.media{grid-template-columns:78px minmax(0,1fr);gap:17px}.poster-wrap{width:78px}.poster{width:78px;border-radius:9px}.streams-button{right:4px;bottom:4px;padding:4px 6px;font-size:8px}.people{flex-wrap:nowrap;overflow-x:auto;padding:0 3px 8px 0;gap:13px;scrollbar-width:thin;scrollbar-color:rgba(127,127,127,.35) transparent;scroll-snap-type:x proximity}.person{width:94px;scroll-snap-align:start}.portrait{width:58px;height:78px;border-radius:8px}.name{font-size:11px}.role{font-size:11px}}
    @keyframes arrive{from{opacity:.4;transform:translateY(3px)}to{opacity:1;transform:none}}@media(prefers-reduced-motion:reduce){.media{animation:none}}
  `);
  function tmdbLink(type, id, label) {
    const link = element('a');
    link.href = `https://www.themoviedb.org/${type}/${id}`;
    link.target = '_blank'; link.rel = 'noopener noreferrer';
    if (label) link.setAttribute('aria-label', label);
    return link;
  }
  async function imageData(path, size) {
    const key = `${size}:${path}`;
    if (imageCache.has(key)) return imageCache.get(key);
    if (imagePending.has(key)) return imagePending.get(key);
    const request = send({type: 'tmdb-image', path, size}).then(result => {
      if (result.data.length < 500000) {
        while (imageCache.size >= 64 || imageBytes + result.data.length > 6000000) {
          const oldest = imageCache.keys().next().value;
          if (oldest === undefined) break;
          imageBytes -= imageCache.get(oldest).length; imageCache.delete(oldest);
        }
        imageCache.set(key, result.data); imageBytes += result.data.length;
      }
      return result.data;
    });
    imagePending.set(key, request);
    try {return await request;} finally {imagePending.delete(key);}
  }
  function photo(data, alt, className) {
    const image = element('img', undefined, className);
    image.src = data; image.alt = alt; image.decoding = 'async';
    return image;
  }
  function openMedia(item, poster, trigger) {
    const kind = item.type === 'tv' ? 'tv' : 'movie';
    try {
      if (globalThis.CineMatchCinema?.open({kind, id: item.id, title: item.title, poster, posterEl: trigger}) === true) return;
    } catch {}
    send({type: 'watch-open', kind, id: item.id}).catch(() => {
      window.open(`https://www.themoviedb.org/${kind}/${item.id}`, '_blank', 'noopener,noreferrer');
    });
  }
  function streamsButton(item, poster) {
    const button = element('button', undefined, 'streams-button');
    button.type = 'button';
    button.title = `Cerca i flussi per ${item.title}`;
    button.setAttribute('aria-label', `Cerca i flussi per ${item.title}`);
    button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z"/></svg>Flussi';
    button.addEventListener('click', event => {
      if (!event.isTrusted) return;
      event.preventDefault(); event.stopPropagation();
      openMedia(item, poster, button);
    });
    return button;
  }
  async function mediaFor(item) {
    const isTitle = item.type === 'movie' || item.type === 'tv';
    const isMovie = item.type === 'movie';
    const people = isTitle
      ? [...(item.directors ?? []).map(p => ({...p, kind: isMovie ? 'Regia' : 'Creazione'})), ...(item.cast ?? []).filter(p => !(item.directors ?? []).some(d => d.id === p.id)).slice(0,4).map(p => ({...p, kind: 'Cast'}))]
      : [];
    const mainPath = isTitle ? item.poster : item.photo;
    const main = mainPath ? await imageData(mainPath, 'w342').catch(() => null) : null;
    const portraits = await Promise.all(people.filter(p => p.photo).map(async person => {
      const data = await imageData(person.photo, 'w185').catch(() => null);
      return data ? {...person, data} : null;
    }));
    const visiblePeople = portraits.filter(Boolean);
    if (!main && !visiblePeople.length) return null;
    const figure = element('figure', undefined, `media${main ? '' : ' no-poster'}`);
    figure.setAttribute('aria-label', `Immagini di ${item.title ?? item.name}`);
    if (main) {
      const poster = tmdbLink(item.type, item.id, isTitle ? `Guarda ${item.title} con CineMatch` : `Foto di ${item.name}`);
      if (isTitle) {
        poster.title = `Guarda ${item.title} con CineMatch`;
        poster.addEventListener('click', event => {
          if (!event.isTrusted) return;
          event.preventDefault();
          openMedia(item, main, poster);
        });
      }
      poster.append(photo(main, isTitle ? `Locandina di ${item.title}` : `Foto di ${item.name}`, 'poster'));
      if (isTitle) {
        const wrap = element('div', undefined, 'poster-wrap');
        wrap.append(poster, streamsButton(item, main));
        figure.append(wrap);
      } else figure.append(poster);
    }
    const caption = element('figcaption'), strip = element('div', undefined, 'people');
    for (const person of visiblePeople) {
      const link = tmdbLink('person', person.id, `${person.name} · ${person.kind}`);
      link.className = 'person'; link.title = `${person.name} · ${person.kind}`;
      link.append(photo(person.data, `Foto di ${person.name}`, 'portrait'), element('span', person.name, 'name'), element('span', person.kind, 'role'));
      strip.append(link);
    }
    caption.append(strip);
    const source = tmdbLink(item.type, item.id, 'Fonte immagini: TMDB');
    source.textContent = 'TMDB'; source.className = 'source'; caption.append(source);
    figure.append(caption); return figure;
  }
  function responseText(root) {
    const walk = node => {
      if (node.nodeType === Node.TEXT_NODE) return node.textContent;
      if (node.nodeType !== Node.ELEMENT_NODE || node.matches('[data-cinematch-ui],button,form,textarea,input,script,style,[aria-hidden="true"]')) return '';
      if (node.tagName === 'BR') return '\n';
      const text = [...node.childNodes].map(walk).join('');
      if (node.tagName === 'STRONG' || node.tagName === 'B') return `**${text}**`;
      if (/^H[1-6]$/.test(node.tagName)) return `\n# ${text}\n`;
      if (node.tagName === 'TD' || node.tagName === 'TH') return ` | ${text}`;
      return /^(P|DIV|LI|TR|UL|OL|BLOCKQUOTE)$/.test(node.tagName) ? `\n${text}\n` : text;
    };
    return walk(root);
  }
  const referenceKey = ref => `${ref.type}:${normalize(ref.title)}:${ref.year ?? ''}`;
  function referencesFor(root) {
    const refs = extract(responseText(root)), locations = new Map();
    const sameReference = (text, ref) => extract(text).some(other => referenceKey(other) === referenceKey(ref));
    for (const node of root.querySelectorAll('h1,h2,h3,h4,h5,h6,[role="heading"],p,li,tr,strong,b')) {
      if (node.closest('[data-cinematch-ui],[aria-hidden="true"]')) continue;
      for (const ref of extract(responseText(node))) {
        const key = referenceKey(ref), previous = locations.get(key);
        const heading = node.closest('h1,h2,h3,h4,h5,h6,[role="heading"]');
        let anchor = heading ?? node, priority = heading ? 4 : /^(STRONG|B)$/.test(node.tagName) ? 2 : 1;
        // Some chats put title and description in the SAME paragraph. Anchor at
        // its first title-line break, not after the entire description paragraph.
        if (!heading) for (const br of node.querySelectorAll('br')) {
          const range = document.createRange(); range.setStart(node, 0); range.setEndBefore(br);
          if (sameReference(range.cloneContents().textContent, ref)) {anchor = br; priority = 3; break;}
        }
        if (!previous || priority > previous.priority || (priority === previous.priority && previous.node.contains(node))) locations.set(key, {node, anchor, priority});
      }
    }
    return refs.map(ref => {const anchor = locations.get(referenceKey(ref))?.anchor; return {ref, anchor: anchor?.closest('table') ?? anchor ?? root};});
  }
  const observer = new IntersectionObserver(entries => {
    for (const item of entries) for (const entry of anchors.get(item.target) ?? []) {
      entry.near = item.isIntersecting; if (entry.near) maybeLoad(entry);
    }
  }, {rootMargin: '250px'});
  function unwatch(entry) {
    const set = anchors.get(entry.anchor); set?.delete(entry);
    if (set && !set.size) {observer.unobserve(entry.anchor); anchors.delete(entry.anchor);}
  }
  function watch(entry, anchor) {
    if (entry.anchor === anchor) return;
    unwatch(entry); entry.anchor = anchor; entry.near = [...(anchors.get(anchor) ?? [])].some(other => other.near);
    if (!anchors.has(anchor)) {anchors.set(anchor, new Set()); observer.observe(anchor);}
    anchors.get(anchor).add(entry);
  }
  function removeEntry(entry) {clearTimeout(entry.timer); entry.version++; unwatch(entry); entry.host.remove();}
  function clearState(state) {for (const entry of state.entries.values()) removeEntry(entry); state.entries.clear();}
  async function maybeLoad(entry) {
    if (!configuration.configured || !entry.host.isConnected || !entry.near || !entry.stable || entry.phase !== 'idle') return;
    entry.phase = 'loading'; const version = entry.version;
    const current = () => entry.version === version && entry.host.isConnected && configuration.configured;
    try {
      const result = await send({type: 'tmdb-search', reference: entry.ref});
      if (!current()) return;
      if (result.matched) {
        const media = await mediaFor(result.matched);
        if (current() && media) {entry.shadow.replaceChildren(media); entry.host.hidden = false;}
      }
    } catch (error) {
      // Never put software errors or configuration forms inside an AI answer.
      // One bounded retry handles transient service/network errors automatically.
      if (current() && entry.attempts < 1 && /raggiungibile|connessione|pausa|al momento/.test(error.message)) {
        entry.attempts++; entry.phase = 'idle'; entry.timer = setTimeout(() => maybeLoad(entry), 1800); return;
      }
    }
    if (current()) entry.phase = 'done';
  }
  function reconcile(state) {
    if (!configuration.configured) {clearState(state); return;}
    let references; try {references = referencesFor(state.root);} catch {return;}
    const keep = new Set(), tails = new Map();
    for (const {ref, anchor} of references) {
      const key = referenceKey(ref); keep.add(key);
      let entry = state.entries.get(key);
      if (!entry) {
        // A span remains valid inside paragraphs containing an inline title;
        // the shadow host's CSS provides the block layout for the images.
        const host = element('span'); host.setAttribute('data-cinematch-ui', 'film'); host.hidden = true;
        const shadow = host.attachShadow({mode: 'open'}); shadow.adoptedStyleSheets = [sheet];
        entry = {host, shadow, ref, anchor: null, near: false, stable: false, phase: 'idle', version: 0, attempts: 0, fingerprint: '', timer: null};
        state.entries.set(key, entry);
      }
      const fingerprint = JSON.stringify(ref);
      if (entry.fingerprint !== fingerprint) {
        entry.fingerprint = fingerprint; entry.ref = ref; entry.version++; entry.phase = 'idle'; entry.attempts = 0; entry.stable = false; entry.host.hidden = true;
        clearTimeout(entry.timer); entry.timer = setTimeout(() => {entry.stable = true; maybeLoad(entry);}, 650);
      }
      const tail = tails.get(anchor) ?? anchor;
      if (anchor === state.root) {if (!entry.host.isConnected) state.root.append(entry.host);}
      else if (tail.nextSibling !== entry.host) tail.after(entry.host);
      tails.set(anchor, entry.host); watch(entry, anchor.tagName === 'BR' ? anchor.parentElement : anchor);
    }
    for (const [key, entry] of state.entries) if (!keep.has(key)) {removeEntry(entry); state.entries.delete(key);}
  }
  function scan() {
    scanTimer = null; if (!configuration.ready) return;
    let nodes = []; for (const selector of selectors) {nodes = [...document.querySelectorAll(selector)]; if (nodes.length) break;}
    nodes = nodes.filter(root => !root.closest('[data-message-author-role="user"],[data-role="user"],[data-testid="user-message"]') && !nodes.some(other => other !== root && other.contains(root)));
    const current = new Set(nodes);
    for (const [root, state] of roots) if (!root.isConnected || !current.has(root)) {clearState(state); roots.delete(root);}
    for (const root of nodes) {
      let state = roots.get(root);
      if (!state) {state = {root, entries: new Map(), dirty: true}; roots.set(root, state);}
      if (state.dirty) {reconcile(state); state.dirty = false;}
    }
  }
  const requestScan = () => {if (!scanTimer) scanTimer = setTimeout(scan, 250);};
  const ownNode = node => (node.nodeType === Node.ELEMENT_NODE ? node : node.parentElement)?.closest?.('[data-cinematch-ui]');
  new MutationObserver(mutations => {
    const changed = mutations.filter(m => !ownNode(m.target) && (m.type === 'characterData' || m.removedNodes.length || [...m.addedNodes].some(node => !ownNode(node))));
    if (changed.length) {for (const [root, state] of roots) if (changed.some(m => root.contains(m.target))) state.dirty = true; requestScan();}
  }).observe(document.body, {childList: true, subtree: true, characterData: true});
  function configure(value) {
    configuration.ready = true; configuration.configured = Boolean(value.configured);
    for (const state of roots.values()) {clearState(state); state.dirty = true;} requestScan();
  }
  chrome.runtime.onMessage.addListener(message => {if (message?.type === 'tmdb-preferences-changed') configure(message);});
  send({type: 'tmdb-status'}).then(configure).catch(() => {});
})();
