export const KINDS = Object.freeze({movement:"Movimento storico", tendency:"Tendenza / scena", cycle:"Genere / ciclo", tradition:"Tradizione", technique:"Tecnica / ambito produttivo"});
export const MODES = Object.freeze({recommendations:"Consigli di visione", introduction:"Introduzione e percorso", discoveries:"Opere meno conosciute", compare:"Confronta correnti", free:"Richiesta libera"});
export const PROVIDERS = Object.freeze([
  {id:"chatgpt", name:"ChatGPT", host:"chatgpt.com", url:"https://chatgpt.com/"},
  {id:"deepseek", name:"DeepSeek", host:"chat.deepseek.com", url:"https://chat.deepseek.com/"},
  {id:"claude", name:"Claude", host:"claude.ai", url:"https://claude.ai/"},
  {id:"gemini", name:"Gemini", host:"gemini.google.com", url:"https://gemini.google.com/"}
]);
export function providerForUrl(value) {
  try { const url = new URL(value); return url.protocol === "https:" ? PROVIDERS.find(p => p.host === url.hostname) ?? null : null; } catch { return null; }
}
export function normalize(value) { return String(value).normalize("NFD").replace(/\p{Diacritic}/gu, "").toLocaleLowerCase("it"); }
export function period(m) { return m.end === null ? `dal ${m.start} circa · sviluppi e continuità` : `circa ${m.start}–${m.end}`; }
export function matches(m, query) {
  const hay = normalize([m.label,m.original_name,...m.aliases,...m.regions,...m.authors,m.summary,...m.works.map(w => w.title)].join(" "));
  return normalize(query).split(/\s+/).filter(Boolean).every(word => hay.includes(word));
}
export function defaultRequest() { return {schema_version:1,movements:["neorealismo"],mode:"recommendations",count:null,depth:"concise",relation:"historical",preferences:"",exclusions:"",watched:[],question:""}; }
export function signature(request) { return JSON.stringify(request); }
export function generatePrompt(request, catalog) {
  if (!request || ![0,1,undefined].includes(request.schema_version)) throw new Error("Versione della richiesta non supportata.");
  if (!Array.isArray(request.movements) || !request.movements.length) throw new Error("Scegli almeno una corrente.");
  if (!Object.hasOwn(MODES,request.mode)) throw new Error("Scegli un obiettivo valido.");
  if (request.count !== null && (!Number.isSafeInteger(request.count) || request.count < 1 || request.count > 4294967295)) throw new Error("Il numero di film deve essere un intero positivo oppure automatico.");
  if (!["concise","detailed"].includes(request.depth) || !["historical","legacy"].includes(request.relation)) throw new Error("Impostazioni della richiesta non valide.");
  for (const key of ["question","preferences","exclusions"]) if (typeof request[key] !== "string") throw new Error("La richiesta deve contenere campi di testo validi.");
  if (!Array.isArray(request.watched) || request.watched.some(s => typeof s !== "string")) throw new Error("Elenco dei titoli non valido.");
  if (request.mode === "free" && !request.question.trim()) throw new Error("Scrivi la richiesta libera.");
  const selected = [...new Set(request.movements)].map(id => {
    const m = catalog.find(m => m.id === id); if (!m) throw new Error(`Voce non riconosciuta: ${id}`); return m;
  });
  if (request.mode === "compare" && selected.length < 2) throw new Error("Per il confronto scegli almeno due correnti.");
  let text = "Aiutami a esplorare il cinema con attenzione storica. Rispondi in italiano.\n\n";
  text += `OBIETTIVO: ${MODES[request.mode]}.\n`;
  if (request.count !== null) text += `Quantità richiesta: ${request.count} film complessivi, senza duplicati. Se non trovi abbastanza opere pertinenti, spiega il limite invece di aggiungere titoli estranei.\n`;
  else if (request.mode === "free") text += "Rispondi alla richiesta libera rispettandone il formato; proponi film soltanto se richiesti.\n";
  else text += "Quantità: proponi un numero motivato e adeguato alla richiesta, senza un limite predefinito di cinque film.\n";
  text += "\nCONTESTO DELLE VOCI SELEZIONATE\n";
  for (const m of selected) {
    text += `- ${m.label} / ${m.original_name} — ${KINDS[m.kind]}; ${period(m)}; ${m.regions.join(", ")}.\n  ${m.summary}\n  Confini: ${m.scope_note}\n`;
    if (request.depth === "detailed") text += `  Caratteristiche: ${m.traits.join("; ")}. Figure di riferimento: ${m.authors.join(", ")}.\n`;
    if (m.sources[0]) text += `  Riferimento da consultare se puoi navigare: ${m.sources[0].title} — ${m.sources[0].url}\n`;
  }
  text += request.relation === "historical"
    ? "\nPrivilegia opere riconosciute come appartenenti alla corrente o al ciclo richiesto. Una coincidenza di anno, paese o genere non basta. Se includi un precursore o un caso discusso, dichiaralo.\n"
    : "\nIncludi anche eredità e influenze successive, distinguendole chiaramente dalle appartenenze storiche. Motiva il collegamento concreto.\n";
  if (request.mode === "introduction") text += "Costruisci un percorso accessibile dal contesto storico alle opere; spiega come guardarle e in quale ordine.\n";
  if (request.mode === "discoveries") text += "Privilegia opere meno note rispetto ai titoli canonici; non dichiararle rare o disponibili in streaming senza verifica.\n";
  if (request.mode === "compare") text += "Confronta le voci senza fonderle: differenze, punti di contatto ed esempi distinti; ripartisci la quantità complessiva in modo esplicito.\n";
  if (request.preferences.trim()) text += `\nPREFERENZE PERSONALI\n${request.preferences.trim()}\n`;
  if (request.exclusions.trim()) text += `\nCRITERI DA ESCLUDERE\n${request.exclusions.trim()}\n`;
  const watched = request.watched.map(s => s.trim()).filter(Boolean);
  if (watched.length) text += `\nGIÀ VISTI / GIÀ PROPOSTI — non riproporre questi titoli, nemmeno con un titolo alternativo:\n${watched.join("; ")}\n`;
  if (request.question.trim()) text += `\nLA MIA RICHIESTA\n${request.question.trim()}\n`;
  text += "\nCOME RISPONDERE\nPer ogni film che proponi usa un'intestazione chiara nel formato Titolo originale (anno) — Regia: Nome del regista; aggiungi il titolo italiano se noto e spiega l'attinenza specifica e come rispetta i miei criteri. ";
  text += request.depth === "concise" ? "Usa spiegazioni brevi e concrete. " : "Aggiungi un'analisi storica e formale più approfondita. ";
  text += "Distingui fatti verificati, attribuzioni discusse e tue inferenze. Se puoi navigare, controlla i riferimenti e cita pagine pertinenti; altrimenti dichiara che non hai verificato online. Non inventare fonti, URL, film, dati o disponibilità sulle piattaforme. Evita spoiler importanti. Se alcuni criteri sono incompatibili, spiegalo prima di proporre alternative.\n";
  return {schema_version:1,prompt:text,warnings:selected.some(m => m.kind !== "movement") ? ["La selezione comprende categorie diverse dai movimenti storici; il prompt le distingue."] : []};
}
