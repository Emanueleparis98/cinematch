const $=id=>document.getElementById(id);
const send=message=>chrome.runtime.sendMessage(message);
const TMDB_SITE_ORIGIN='https://www.themoviedb.org/*';
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
async function refresh(){const result=await send({type:'tmdb-status'});$('connection').textContent=result?.configured?'TMDB è configurato. Le immagini sono automatiche.':'Collega TMDB per attivare le immagini.';$('remove').disabled=!result?.configured;await refreshSite().catch(error=>{$('tmdb-site-status').textContent=error.message;});}
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
$('reveal').addEventListener('click',()=>{const show=$('credential').type==='password';$('credential').type=show?'text':'password';$('reveal').textContent=show?'Nascondi':'Mostra';$('reveal').setAttribute('aria-pressed',String(show));});
$('credential-form').addEventListener('submit',async event=>{
  event.preventDefault();$('save').disabled=true;$('remove').disabled=true;$('settings-status').className='';$('settings-status').textContent='Verifico la connessione a TMDB…';
  try{const result=await send({type:'tmdb-save',credential:$('credential').value,remember:$('remember').checked});if(!result?.ok)throw new Error(result?.error??'Salvataggio non riuscito. Riprova.');$('credential').value='';$('credential').type='password';$('reveal').textContent='Mostra';$('reveal').setAttribute('aria-pressed','false');$('settings-status').textContent='Credenziale verificata e salvata. Le chat aperte si aggiornano da sole.';}
  catch(error){$('settings-status').className='error';$('settings-status').textContent=error.message;}
  finally{$('save').disabled=false;await refresh();}
});
$('remove').addEventListener('click',async()=>{const result=await send({type:'tmdb-remove'});$('settings-status').textContent=result?.ok?'Credenziale rimossa. Le immagini automatiche non vengono più caricate.':'Rimozione non riuscita. Riprova.';await refresh();});
refresh().catch(()=>{$('connection').textContent='Chiudi e riapri questa pagina per controllare la configurazione.';});
