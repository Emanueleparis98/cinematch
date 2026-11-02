const API='https://api.themoviedb.org/3';
const normalize=text=>String(text).normalize('NFD').replace(/\p{Diacritic}/gu,'').toLowerCase().replace(/[^\p{L}\p{N}]+/gu,' ').trim();
export const safeImagePath=value=>typeof value==='string'&&/^\/[a-zA-Z0-9_-]+\.(?:jpg|png|webp)$/.test(value)?value:null;
export function credential(value){
  const key=typeof value==='string'?value.trim().replace(/^Bearer\s+/i,''):'';
  if(!/^[a-f0-9]{32}$/i.test(key)&&!/^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(key))throw new Error('Inserisci una chiave API v3 o un API Read Access Token TMDB valido.');
  return key;
}
export function validateReference(value){
  if(!value||!['movie','person'].includes(value.type)||typeof value.title!=='string'||!value.title.trim()||value.title.length>150)throw new Error('Titolo o nome non valido.');
  if(value.year!=null&&(!Number.isInteger(value.year)||value.year<1800||value.year>2100))throw new Error('Anno non valido.');
  const variants=(Array.isArray(value.variants)?value.variants:[value.title]).filter(v=>typeof v==='string'&&v.trim()&&v.length<=150).slice(0,3);
  return {type:value.type,title:value.title.trim(),variants:[...new Set([value.title.trim(),...variants])].slice(0,3),year:value.year??null,director:typeof value.director==='string'?value.director.slice(0,100):''};
}
function person(p){return {id:p.id,name:p.name??'',role:p.character??p.job??p.known_for_department??'',photo:safeImagePath(p.profile_path)};}
export function summarizeMovie(data){
  return {type:'movie',id:data.id,title:data.title??data.original_title??'',originalTitle:data.original_title??'',year:data.release_date?.slice(0,4)??'',poster:safeImagePath(data.poster_path),directors:(data.credits?.crew??[]).filter(p=>p.job==='Director').map(person),cast:(data.credits?.cast??[]).map(person)};
}
export function chooseMatch(results,ref){
  if(ref.type==='person'){
    const exact=results.filter(p=>ref.variants.some(name=>normalize(name)===normalize(p.name)));
    return exact.length===1?exact[0]:null;
  }
  if(ref.type!=='movie'||!ref.year)return null;
  const names=ref.variants.map(normalize);
  const exact=results.filter(m=>Number(m.release_date?.slice(0,4))===ref.year&&[m.title,m.original_title].some(t=>t&&names.includes(normalize(t))));
  return exact.length===1?exact[0]:null;
}
export function createTmdbClient({getCredential,fetcher=fetch,timeout=12000}){
  const cache=new Map(),pending=new Map();let active=0;const waiters=[];
  const queue=async fn=>{if(active>=3)await new Promise(resolve=>waiters.push(resolve));active++;try{return await fn();}finally{active--;waiters.shift()?.();}};
  async function request(path,params={}){
    const key=await getCredential();if(!key)throw new Error('Configura TMDB dalle impostazioni di CineMatch.');
    const cacheKey=JSON.stringify([path,params,key]);
    const stored=cache.get(cacheKey);if(stored&&Date.now()-stored.time<300000)return stored.data;
    if(pending.has(cacheKey))return pending.get(cacheKey);
    const operation=queue(async()=>{
      const url=new URL(API+path);for(const [name,value]of Object.entries(params))url.searchParams.set(name,String(value));
      const headers={accept:'application/json'};
      if(key.includes('.'))headers.Authorization=`Bearer ${key}`;else url.searchParams.set('api_key',key);
      let response;try{response=await fetcher(url.href,{headers,signal:AbortSignal.timeout(timeout),cache:'no-store',credentials:'omit',redirect:'error'});}catch{throw new Error('TMDB non raggiungibile. Controlla la connessione e riprova.');}
      if(!response.ok)throw new Error(({401:'Credenziale TMDB non valida. Controllala nelle impostazioni.',403:'TMDB ha rifiutato l’accesso.',404:'Scheda non disponibile su TMDB.',429:'TMDB richiede una pausa. Attendi qualche secondo e riprova.'})[response.status]??'TMDB non è disponibile al momento. Riprova.');
      const data=await response.json();if(cache.size>=100)cache.delete(cache.keys().next().value);cache.set(cacheKey,{time:Date.now(),data});return data;
    });
    pending.set(cacheKey,operation);try{return await operation;}finally{pending.delete(cacheKey);}
  }
  async function details(type,id){
    if(!['movie','person'].includes(type)||!Number.isSafeInteger(id)||id<1)throw new Error('Scheda non valida.');
    const data=await request(`/${type}/${id}`,{language:'it-IT',...(type==='movie'?{append_to_response:'credits'}:{})});
    if(type==='movie')return summarizeMovie(data);
    return {type:'person',...person(data),biography:(data.biography??'').slice(0,600),birthday:data.birthday??'',place:data.place_of_birth??''};
  }
  async function search(value){
    const ref=validateReference(value);let results=[];
    for(const query of ref.variants){
      const data=await request(`/search/${ref.type}`,{query,language:'it-IT',include_adult:false,...(ref.type==='movie'&&ref.year?{year:ref.year}:{})});
      for(const result of data.results??[])if(Number.isSafeInteger(result.id)&&!results.some(r=>r.id===result.id))results.push(result);
      if(chooseMatch(results,ref)||(ref.type==='person'&&results.length))break;
    }
    const exact=chooseMatch(results,ref);
    if(exact){const item=await details(ref.type,exact.id);if(ref.type==='person'||!ref.director||item.directors.some(p=>normalize(p.name)===normalize(ref.director)))return {matched:item,candidates:[]};}
    return {matched:null,candidates:results.map(r=>ref.type==='movie'?{id:r.id,type:'movie',title:r.title??r.original_title??'',originalTitle:r.original_title??'',year:r.release_date?.slice(0,4)??'',poster:safeImagePath(r.poster_path)}:{id:r.id,type:'person',title:r.name??'',year:r.known_for_department??'',poster:safeImagePath(r.profile_path)}),hasMore:results.length>=20};
  }
  async function image(path,size){
    if(!safeImagePath(path)||!['w92','w185','w342'].includes(size))throw new Error('Immagine non valida.');
    return queue(async()=>{
      let response;try{response=await fetcher(`https://image.tmdb.org/t/p/${size}${path}`,{signal:AbortSignal.timeout(timeout),credentials:'omit',redirect:'error',cache:'no-store'});}catch{throw new Error('Immagine non disponibile.');}
      const mime=response.headers.get('content-type')?.split(';')[0];
      if(!response.ok||!['image/jpeg','image/png','image/webp'].includes(mime))throw new Error('Immagine non disponibile.');
      const bytes=new Uint8Array(await response.arrayBuffer());if(bytes.length>2000000)throw new Error('Immagine troppo grande.');
      let raw='';for(let i=0;i<bytes.length;i+=8192)raw+=String.fromCharCode(...bytes.subarray(i,i+8192));
      return `data:${mime};base64,${btoa(raw)}`;
    });
  }
  return {search,details,image,clear:()=>cache.clear(),verify:()=>request('/configuration')};
}
