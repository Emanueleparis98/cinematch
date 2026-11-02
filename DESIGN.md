# CineMatch — identità LUMEN conservata

Modalità: Operate per ricerca e generazione; Read per le schede storiche.
La nuova sezione eredita l'identità già presente nell'app, senza sostituirla.

- Fondi scuri caldi, testo avorio, accento ember, palette in `src/ui/lumen.rs`.
- Fraunces per i titoli; Instrument Sans per interfaccia e lettura;
  JetBrains Mono limitato a dati come versione e misure.
- Estensione: pannello laterale con due sezioni, Le correnti e Il tuo prompt.
  Ricerca sopra un elenco con righe e separatori; filtri espandibili, schede con
  ritorno esplicito all'elenco. Vista ampia facoltativa in una scheda del browser.
- Layout verificato da 320×568 a 900×850. Intestazione e sezioni rimangono
  visibili; scorre il contenuto. Sotto 340 px i campi dei filtri passano a una colonna.
- Generazione, copia e inserimento restano nel piede del pannello. La scelta
  per una bozza già presente è inline e non sovrascrive testo senza scelta esplicita.
  Preferenze avanzate espandibili, prompt modificabile, nessun invio automatico.
- Le categorie storiche sono testo esplicativo, non una codifica affidata solo
  al colore. I periodi indicativi e le note di attribuzione sono parte del contenuto.
- Il prompt superato da nuovi criteri richiede una rigenerazione prima della copia.
  Errori e stato di copia sono inline. Le fonti si aprono su azione dell'utente.
- Risposte AI: schede automatiche direttamente sotto ciascuna intestazione del film,
  stili isolati in Shadow DOM. Locandina accanto a titolo/anno; ritratto del regista
  e primi tre interpreti già visibili. Cast completo espandibile, foto con proporzioni
  stabili e icona neutra accessibile per immagini mancanti nei ritratti compatti.
  Caricamento progressivo vicino al viewport e stabilizzazione durante lo streaming,
  abbinamenti incerti da scegliere e modulo per cercare/correggere senza modali.
- Impostazioni TMDB in scheda dedicata: credenziale mascherata, verifica prima del
  salvataggio, rimozione e scelta della persistenza. Crediti e logo ufficiale TMDB.
