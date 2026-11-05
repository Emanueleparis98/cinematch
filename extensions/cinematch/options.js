const $=id=>document.getElementById(id);
const send=message=>chrome.runtime.sendMessage(message);
const TMDB_SITE_ORIGIN='https://www.themoviedb.org/*';
const CB01_ORIGINS=[
  'https://cb01one.org/*','https://www.cb01one.org/*','https://stayonline.pro/*','https://uprot.net/*',
  'https://*.mxdrop.click/*','https://*.m1xdrop.net/*','https://*.mixdrop.co/*','https://*.mixdrop.ag/*',
  'https://*.mxdrop.to/*','https://*.mixdrop.to/*','https://*.mxcontent.net/*'
];
const DEEPSEEK_CURRENTS_KEY='deepseekCurrentsButton';
let siteConfigured=false;
async function refreshSite(){
  const result=await send({type:'tmdb-site-status'});
  if(!result?.ok)throw new Error(result?.error??'Stato del pulsante TMDB non disponibile.');
  siteConfigured=Boolean(result.configured);
  $('tmdb-site-toggle').checked=Boolean(result.enabled);
  $('tmdb-site-toggle').disabled=!siteConfigured;
  $('tmdb-site-status').textContent=!siteConfigured
    ?'Collega prima TMDB. Il pulsante sui poster sarà disponibile dopo la verifica.'
    :result.enabled
      ?'Attivo su themoviedb.org. I pulsanti compaiono sui poster di film e serie TV.'
      :result.requested&&!result.allowed
        ?'Il permesso per themoviedb.org non è più attivo. Riattiva l’opzione per concederlo di nuovo.'
        :'Disattivo. I poster di TMDB non mostrano il pulsante Flussi.';
}
async function refreshCb01(){
  const result=await send({type:'cb01-status'});
  if(!result?.ok)throw new Error(result?.error??'Stato della modalità CB01 non disponibile.');
  $('cb01-toggle').checked=Boolean(result.enabled);
  $('cb01-toggle').disabled=false;
  $('cb01-status').textContent=result.enabled
    ?result.rules===false
      ?'Attivo su cb01one.org, ma i filtri non sono ancora pronti. Disattiva e riattiva l’opzione.'
      :'Attivo su cb01one.org. Pubblicità bloccata e player CineMatch pronto per film e serie TV.'
    :result.requested&&!result.allowed
      ?'Il permesso per CB01 o per i suoi player non è più attivo. Riattiva l’opzione per concederlo di nuovo.'
      :'Disattivo. CB01 si apre senza bloccare la pubblicità e senza il pulsante CineMatch.';
}
async function refreshDeepSeekCurrents(){
  const data=await chrome.storage.local.get(DEEPSEEK_CURRENTS_KEY);
  const enabled=data[DEEPSEEK_CURRENTS_KEY]===true;
  $('deepseek-currents-toggle').checked=enabled;
  $('deepseek-currents-status').textContent=enabled
    ?'Attivo. Il pulsante Correnti compare nel composer di DeepSeek accanto agli altri due pulsanti.'
    :'Disattivo. DeepSeek non mostra il pulsante Correnti.';
}
async function refresh(){const result=await send({type:'tmdb-status'});$('connection').textContent=result?.configured?'TMDB è configurato. Le immagini sono automatiche.':'Collega TMDB per attivare le immagini.';$('remove').disabled=!result?.configured;await refreshSite().catch(error=>{$('tmdb-site-status').textContent=error.message;});await refreshCb01().catch(error=>{$('cb01-status').textContent=error.message;});await refreshDeepSeekCurrents().catch(error=>{$('deepseek-currents-status').textContent=error.message;});}
$('tmdb-site-toggle').addEventListener('change',async()=>{
  const toggle=$('tmdb-site-toggle'),status=$('tmdb-site-status');
  if(!siteConfigured){toggle.checked=false;status.textContent='Collega prima TMDB dalle impostazioni qui sopra.';return;}
  toggle.disabled=true;
  try{
    if(toggle.checked){
      status.textContent='Richiesta accesso a themoviedb.org…';
      const granted=await chrome.permissions.request({origins:[TMDB_SITE_ORIGIN]});
      if(!granted){toggle.checked=false;throw new Error('Permesso per themoviedb.org non concesso.');}
      await chrome.storage.local.set({tmdbSiteButton:true});
      const result=await send({type:'tmdb-site-sync'});
      if(!result?.ok)throw new Error(result?.error??'Attivazione non riuscita.');
      status.textContent='Attivo su themoviedb.org. I pulsanti compaiono sui poster di film e serie TV; ricarica una scheda già aperta se non li vedi.';
    }else{
      status.textContent='Disattivazione sui poster di TMDB…';
      const result=await send({type:'tmdb-site-disable'});
      if(!result?.ok)throw new Error(result?.error??'Disattivazione non riuscita.');
      status.textContent='Disattivo. I poster di TMDB non mostrano il pulsante Flussi.';
      await Promise.race([chrome.permissions.remove({origins:[TMDB_SITE_ORIGIN]}).catch(()=>false),new Promise(resolve=>setTimeout(resolve,1200))]);
    }
  }catch(error){
    await refreshSite().catch(()=>{});
    status.textContent=error.message;
  }finally{
    toggle.disabled=!siteConfigured;
  }
});
$('cb01-toggle').addEventListener('change',async()=>{
  const toggle=$('cb01-toggle'),status=$('cb01-status');
  toggle.disabled=true;
  try{
    if(toggle.checked){
      status.textContent='Richiesta accesso a CB01 e ai player…';
      const granted=await chrome.permissions.request({origins:CB01_ORIGINS});
      if(!granted){toggle.checked=false;throw new Error('Permesso per CB01 o per i suoi player non concesso.');}
      const result=await send({type:'cb01-sync'});
      if(!result?.ok)throw new Error(result?.error??'Attivazione non riuscita.');
      status.textContent='Attivo su cb01one.org. Pubblicità bloccata e player CineMatch pronto per film e serie TV; ricarica una scheda CB01 già aperta se non vedi i pulsanti.';
    }else{
      status.textContent='Disattivazione della modalità CB01…';
      const result=await send({type:'cb01-disable'});
      if(!result?.ok)throw new Error(result?.error??'Disattivazione non riuscita.');
      status.textContent='Disattivo. CB01 si apre senza bloccare la pubblicità e senza il pulsante CineMatch.';
      await Promise.race([chrome.permissions.remove({origins:CB01_ORIGINS}).catch(()=>false),new Promise(resolve=>setTimeout(resolve,1200))]);
    }
  }catch(error){
    await refreshCb01().catch(()=>{});
    status.textContent=error.message;
  }finally{
    toggle.disabled=false;
  }
});
$('deepseek-currents-toggle').addEventListener('change',async()=>{
  const toggle=$('deepseek-currents-toggle'),status=$('deepseek-currents-status');
  toggle.disabled=true;
  try{
    await chrome.storage.local.set({[DEEPSEEK_CURRENTS_KEY]:toggle.checked});
    const result=await send({type:'deepseek-currents-sync'});
    if(!result?.ok)throw new Error(result?.error??'Aggiornamento delle schede DeepSeek non riuscito.');
    await refreshDeepSeekCurrents();
    status.textContent=toggle.checked
      ?'Attivo. Apri o ricarica DeepSeek: il pulsante Correnti compare accanto a Pensiero Profondo e Ricerca.'
      :'Disattivo. DeepSeek non mostra il pulsante Correnti.';
  }catch(error){
    status.textContent=error.message;
    await refreshDeepSeekCurrents().catch(()=>{});
  }finally{
    toggle.disabled=false;
  }
});
$('reveal').addEventListener('click',()=>{const show=$('credential').type==='password';$('credential').type=show?'text':'password';$('reveal').textContent=show?'Nascondi':'Mostra';$('reveal').setAttribute('aria-pressed',String(show));});
$('credential-form').addEventListener('submit',async event=>{
  event.preventDefault();$('save').disabled=true;$('remove').disabled=true;$('settings-status').className='';$('settings-status').textContent='Verifico la connessione a TMDB…';
  try{const result=await send({type:'tmdb-save',credential:$('credential').value,remember:$('remember').checked});if(!result?.ok)throw new Error(result?.error??'Salvataggio non riuscito. Riprova.');$('credential').value='';$('credential').type='password';$('reveal').textContent='Mostra';$('reveal').setAttribute('aria-pressed','false');$('settings-status').textContent='Credenziale verificata e salvata. Le chat aperte si aggiornano da sole.';}
  catch(error){$('settings-status').className='error';$('settings-status').textContent=error.message;}
  finally{$('save').disabled=false;await refresh();}
});
$('remove').addEventListener('click',async()=>{const result=await send({type:'tmdb-remove'});$('settings-status').textContent=result?.ok?'Credenziale rimossa. Le immagini automatiche non vengono più caricate.':'Rimozione non riuscita. Riprova.';await refresh();});
refresh().catch(()=>{$('connection').textContent='Chiudi e riapri questa pagina per controllare la configurazione.';});
