/* Small, reversible popunder guard. It reads no credentials or media data. */
(() => {
  if(globalThis.__cineMatchLiveGuard)return;
  globalThis.__cineMatchLiveGuard=true;
  const ads=['adsco.re','cdn4ads.com','rapidadexchange.com','acscdn.com','doubleclick.net','googlesyndication.com','googleadservices.com','popads.net','popcash.net','propellerads.com','onclickads.net','adcash.com','exoclick.com','adsterra.com'];
  const original=window.open;
  const blocked=value=>{
    if(!value||String(value)==='about:blank')return true;
    try{const url=new URL(String(value),location.href);return ads.some(host=>url.hostname===host||url.hostname.endsWith('.'+host));}catch{return false;}
  };
  function guarded(url,...rest){if(blocked(url))return null;return original.call(this,url,...rest);}
  const install=()=>{window.open=guarded;};
  install();
  window.addEventListener('cinematch:live-disabled',()=>{if(window.open===guarded)window.open=original;});
  window.addEventListener('cinematch:live-enabled',install);
})();
