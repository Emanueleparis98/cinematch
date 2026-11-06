import {manifestUrl,permissionOrigin,parseManifest,fetchJson} from './streams.js';
import {DEBRID} from './debrid.js';
const $=id=>document.getElementById(id);
const status=(id,text)=>{$(id).textContent=text;};
async function send(message){const result=await chrome.runtime.sendMessage(message);if(!result?.ok)throw new Error(result?.error??'Operazione non riuscita.');return result;}
async function render(){
  const data=await chrome.storage.local.get(null),list=$('stream-addon-list');list.replaceChildren();
  const entries=Object.entries(data).filter(([key])=>key.startsWith('streamAddon:'));
  for(const [key,addon]of entries){
    if(!addon||typeof addon.name!=='string')continue;
    const row=document.createElement('div');row.className='ss-addon';
    const enabled=document.createElement('input');enabled.type='checkbox';enabled.checked=addon.enabled!==false;enabled.setAttribute('aria-label','Attiva '+addon.name);
    enabled.onchange=async()=>{await chrome.storage.local.set({[key]:{...addon,enabled:enabled.checked}});};
    const name=document.createElement('span');name.textContent=addon.name;
    const remove=document.createElement('button');remove.type='button';remove.textContent='Rimuovi';remove.setAttribute('aria-label','Rimuovi '+addon.name);remove.onclick=async()=>{await chrome.storage.local.remove(key);await render();};
    row.append(enabled,name,remove);list.append(row);
  }
  if(!entries.length)list.textContent='Nessun addon collegato.';
}
$('stream-addon-form').onsubmit=async event=>{
  event.preventDefault();const button=event.currentTarget.querySelector('button');button.disabled=true;
  try{
    const url=manifestUrl($('stream-addon-url').value);
    if(!await chrome.permissions.request({origins:[permissionOrigin(url)]}))throw new Error('Accesso all’addon non concesso.');
    const data=await chrome.storage.local.get(null);
    const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(url));
    const key='streamAddon:'+Array.from(new Uint8Array(digest),byte=>byte.toString(16).padStart(2,'0')).join('');
    if(!data[key]&&Object.keys(data).filter(name=>name.startsWith('streamAddon:')).length>=20)throw new Error('Limite di 20 addon raggiunto.');
    const addon=parseManifest(await fetchJson(url),url);await chrome.storage.local.set({[key]:addon});
    $('stream-addon-url').value='';status('stream-addon-status',addon.name+' collegato.');await render();
  }catch(error){status('stream-addon-status',error.message);}finally{button.disabled=false;}
};
async function refreshDebrid(){
  const result=await send({type:'debrid-status'});
  status('debrid-status',result.enabled?DEBRID[result.provider].name+' collegato: chiave condivisa per i torrent degli addon.':'Nessun servizio collegato.');
  if(result.provider)$('debrid-provider').value=result.provider;
}
$('debrid-form').onsubmit=async event=>{
  event.preventDefault();const button=event.currentTarget.querySelector('button[type="submit"]');button.disabled=true;
  try{
    const provider=$('debrid-provider').value,key=$('debrid-key').value.trim();
    if(!await chrome.permissions.request({origins:[DEBRID[provider].origin]}))throw new Error('Accesso al servizio non concesso.');
    status('debrid-status','Verifico la chiave…');await send({type:'debrid-save',provider,key});$('debrid-key').value='';await refreshDebrid();
  }catch(error){status('debrid-status',error.message);}finally{button.disabled=false;}
};
$('debrid-remove').onclick=async()=>{try{await send({type:'debrid-remove'});$('debrid-key').value='';await refreshDebrid();}catch(error){status('debrid-status',error.message);}};
chrome.storage.onChanged.addListener((changes,area)=>{if(area==='local'&&Object.keys(changes).some(key=>key.startsWith('streamAddon:')))render().catch(()=>{});});
render().catch(()=>status('stream-addon-status','Impossibile leggere gli addon.'));
refreshDebrid().catch(error=>status('debrid-status',error.message));
