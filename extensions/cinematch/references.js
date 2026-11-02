/* Shared pure parsing helpers. This namespace lives in the isolated extension
 * world, never in the AI site's JavaScript world. No network or DOM access. */
(() => {
  const normalize = text => String(text).normalize('NFD').replace(/\p{Diacritic}/gu, '').toLocaleLowerCase('it').replace(/[^\p{L}\p{N}]+/gu, ' ').trim();
  const trimTitle = value => {
    let text=value.replace(/[*_`]/g,'').trim();
    // Decoration may precede the list number: "🌊 2. Flow". Do not strip
    // leading digits from real titles such as "2001: Odissea nello spazio".
    for(let i=0;i<6;i++){
      const before=text;
      text=text.replace(/^\s*(?:#{1,6}\s*|\d{1,3}[.)]\s+|[-•–—]\s+)/,'')
        .replace(/^(?:\s|\p{Extended_Pictographic}|\p{Regional_Indicator}|[\u200B\uFE0F\uFEFF\u200D])+/u,'');
      if(text===before)break;
    }
    return text.replace(/^titolo(?: originale| italiano)?\s*:\s*/i,'').replace(/^["“«]+|["”»]+$/g,'').replace(/[\s:—–|,]+$/g,'').trim();
  };
  function extract(text) {
    if (typeof text !== 'string' || text.length > 200000) throw new Error('Risposta troppo lunga: cerca un titolo manualmente.');
    const lines = text.split(/\r?\n/).map(line => line.replace(/\[(\d+)\]/g, '').trim()).filter(Boolean);
    const refs = [], seen = new Set();
    const add = ref => {const key=`${ref.type}:${normalize(ref.title)}:${ref.year??''}`;if(!seen.has(key)){seen.add(key);refs.push(ref);}};
    for (let i=0;i<lines.length;i++) {
      let line=lines[i], title='', alias='', year;
      if (line.startsWith('|')) {
        const cells=line.split('|').map(s=>s.trim()).filter(Boolean);
        const y=cells.findIndex(s=>/^(18\d{2}|19\d{2}|20\d{2}|2100)$/.test(s));
        if(y>0){title=cells[y-1];year=Number(cells[y]);}
      }
      if(!title){
        const match=line.match(/^(.{1,160}?)\s*\(([^()\n]*?)(18\d{2}|19\d{2}|20\d{2}|2100)\s*\)/)
          ??line.match(/^(.{1,160}?)\s+[—–|]\s*()(18\d{2}|19\d{2}|20\d{2}|2100)\b/);
        if(match){title=match[1];alias=match[2].replace(/[,\s]+$/g,'');year=Number(match[3]);}
      }
      if(!title&&/^(?:#{1,6}\s|\d+[.)]\s|\*\*)/.test(line)){
        const next=lines[i+1]?.match(/^(?:anno(?: di uscita)?\s*:\s*|\(?)(18\d{2}|19\d{2}|20\d{2}|2100)\b/i);
        if(next){title=line;year=Number(next[1]);}
      }
      title=trimTitle(title);alias=trimTitle(alias);
      if(title&&title.length<=150&&title.split(/\s+/).length<=22&&!/^(?:nel|tra il|dal|durante|la corrente|il movimento|regia|regista|anno|durata)\b/i.test(title)){
        const variants=[...new Set([title,...title.split(/\s+\/\s+/),alias,...title.split(/\s+[–—-]\s+/).slice(0,1)].filter(Boolean))];
        const director=(line+'\n'+(lines[i+1]??'')).match(/(?:regia|regista|director|diretto da)\s*:?\s*([^\n;|]+?)(?=\s+[—–]\s|\s*\*\*|$|\n)/i)?.[1]?.trim().replace(/[.]+$/,'')??'';
        add({type:'movie',title:variants[0],variants,year,director});
      }
      const person=line.replace(/[*_]/g,'').match(/^(?:regista|attore|attrice|director|actor)\s*:\s*([^;|\n]{2,100})$/i);
      if(person&&!title)add({type:'person',title:person[1].replace(/[.]+$/,'').trim(),variants:[person[1].trim()],year:null,director:''});
    }
    return refs.filter(ref=>ref.type!=='person'||!refs.some(movie=>movie.type==='movie'&&normalize(movie.director)===normalize(ref.title)));
  }
  globalThis.CineMatchReferences=Object.freeze({extract,normalize,trimTitle});
})();
