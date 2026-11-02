const $=id=>document.getElementById(id);
const send=message=>chrome.runtime.sendMessage(message);
async function refresh(){const result=await send({type:'tmdb-status'});$('connection').textContent=result?.configured?'TMDB è configurato. Le immagini sono automatiche.':'Collega TMDB per attivare le immagini.';$('remove').disabled=!result?.configured;}
$('reveal').addEventListener('click',()=>{const show=$('credential').type==='password';$('credential').type=show?'text':'password';$('reveal').textContent=show?'Nascondi':'Mostra';$('reveal').setAttribute('aria-pressed',String(show));});
$('credential-form').addEventListener('submit',async event=>{
  event.preventDefault();$('save').disabled=true;$('remove').disabled=true;$('settings-status').className='';$('settings-status').textContent='Verifico la connessione a TMDB…';
  try{const result=await send({type:'tmdb-save',credential:$('credential').value,remember:$('remember').checked});if(!result?.ok)throw new Error(result?.error??'Salvataggio non riuscito. Riprova.');$('credential').value='';$('credential').type='password';$('reveal').textContent='Mostra';$('reveal').setAttribute('aria-pressed','false');$('settings-status').textContent='Credenziale verificata e salvata. Le chat aperte si aggiornano da sole.';}
  catch(error){$('settings-status').className='error';$('settings-status').textContent=error.message;}
  finally{$('save').disabled=false;await refresh();}
});
$('remove').addEventListener('click',async()=>{const result=await send({type:'tmdb-remove'});$('settings-status').textContent=result?.ok?'Credenziale rimossa. Le immagini automatiche non vengono più caricate.':'Rimozione non riuscita. Riprova.';await refresh();});
refresh().catch(()=>{$('connection').textContent='Chiudi e riapri questa pagina per controllare la configurazione.';});
