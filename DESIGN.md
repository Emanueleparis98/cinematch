# CineMatch — identità LUMEN conservata

Modalità: Operate per ricerca e generazione; Read per le schede storiche.
La nuova sezione eredita l'identità già presente nell'app, senza sostituirla.

- Fondi scuri caldi, testo avorio, accento ember, palette in `src/ui/lumen.rs`.
- Fraunces per i titoli; Instrument Sans per interfaccia e lettura;
  JetBrains Mono limitato a dati come versione e misure.
- Schermata a due aree: navigazione e ricerca a sinistra, scheda o compositore
  a destra. Elenco con righe e separatori, senza una griglia di schede decorative.
- Sotto 1000 pixel logici, navigazione, scheda e compositore hanno passaggi
  espliciti e usano l'intera larghezza; dimensione minima supportata 800×600.
- Azioni di generazione, copia e apertura chat restano sopra l'area scorrevole
  del compositore. I campi personali e il prompt sono modificabili.
- Le categorie storiche sono testo esplicativo, non una codifica affidata solo
  al colore. I periodi indicativi e le note di attribuzione sono parte del contenuto.
- Il prompt superato da nuovi criteri richiede una rigenerazione prima della copia.
  Errori e stato di copia sono inline. Le fonti si aprono su azione dell'utente.
