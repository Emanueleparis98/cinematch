/* MAIN-world guard for CB01. It never touches the page's data: it only stops
   popunders and keeps the normal links usable. No network access. */
(() => {
  if (globalThis.__cineMatchCb01Main) return;
  globalThis.__cineMatchCb01Main = true;

  const AD_HOSTS = [
    'adsco.re', 'cdn4ads.com', 'rapidadexchange.com', 'acscdn.com', 'sharethis.com',
    'histats.com', 'polldaddy.com', 'crowdsignal.com', 'doubleclick.net',
    'googlesyndication.com', 'googleadservices.com', 'googletagmanager.com',
    'google-analytics.com', 'usrpubtrck.com', 'vidwestxyz.com', 'soakysecrets.com',
    'mdstats.info', 'popads.net', 'propellerads.com', 'onclickads.net',
    'adcash.com', 'exoclick.com'
  ];
  const SAFE_HOSTS = [
    'cb01one.org', 'www.cb01one.org', 'stayonline.pro', 'uprot.net',
    't.me', 'telegram.me', 'youtube.com', 'www.youtube.com', 'youtu.be',
    'filmtv.it', 'www.filmtv.it', 'cb01official.com', 'www.cb01official.com',
    'cb01.uno', 'www.cb01.uno', 'feedly.com', 'x.com', 'twitter.com',
    'facebook.com', 'www.facebook.com'
  ];
  const hostOf = value => {
    try {
      const url = new URL(value, location.href);
      if (!['https:', 'http:'].includes(url.protocol)) return null;
      return url.hostname.toLowerCase();
    } catch { return null; }
  };
  const matches = (host, domains) => Boolean(host && domains.some(domain => host === domain || host.endsWith(`.${domain}`)));
  const isAd = value => {
    const host = hostOf(value);
    return matches(host, AD_HOSTS);
  };
  const isSafe = value => {
    if (typeof value !== 'string' || !value.trim()) return false;
    try {
      const url = new URL(value, location.href);
      if (url.origin === location.origin) return true;
      return matches(url.hostname.toLowerCase(), SAFE_HOSTS);
    } catch { return false; }
  };

  const originalOpen = globalThis.open;
  const guardOpen = function (url, ...rest) {
    if (!isSafe(url)) return null;
    return originalOpen.call(this, url, ...rest);
  };
  const guardClick = event => {
    if (!event.isTrusted || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const anchor = event.target instanceof Element ? event.target.closest('a[href]') : null;
    if (!anchor || !isAd(anchor.href)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };
  const install = () => {
    globalThis.open = guardOpen;
    document.addEventListener('click', guardClick, true);
  };
  install();
  window.addEventListener('cinematch:cb01-disabled', () => {
    globalThis.open = originalOpen;
    document.removeEventListener('click', guardClick, true);
  });
  window.addEventListener('cinematch:cb01-enabled', install);

  try {
    if (typeof globalThis.adsLoaded === 'undefined') globalThis.adsLoaded = true;
  } catch {}
})();
