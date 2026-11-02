# CineMatch: nuova ricerca conversazionale

## Obiettivo e impegno

Sostituire il comportamento della ricerca AI, non aggiungere altre eccezioni al parser attuale. L'utente deve poter descrivere una situazione, una relazione narrativa, un'atmosfera o una combinazione di preferenze in linguaggio naturale. La risposta deve rispettare contenuto, quantita e forma della richiesta.

Il 99,9% di attinenza su ogni fantasia possibile non e una garanzia dimostrabile: dipende anche dalla presenza e correttezza delle informazioni sui film. L'impegno verificabile e non spacciare risultati incerti per corrispondenze esatte, non ignorare esclusioni, non ripetere titoli quando si chiedono alternative e non riempire una quota con film inadatti.

Piano implementato nel motore `src/search`, collegato alla chat AI. Risultati, prove e limiti della consegna sono documentati in [verifica del 2 novembre 2026](verifica-nuova-ricerca-ai-2026-11-02.md).

## 1. Contratto della conversazione

- La richiesta originale resta la fonte principale. La tassonomia del catalogo non limita cio che si puo chiedere alla chat.
- Un film significa una raccomandazione principale; cinque film significa cinque alternative distinte, se esistono abbastanza corrispondenze verificate. Non significa cinque gruppi da cinque film.
- Senza un numero esplicito, il sistema sceglie una lunghezza utile alla domanda; nessun limite globale fisso di cinque risultati.
- Una domanda, un confronto o una spiegazione riceve quel tipo di risposta, senza aggiungere automaticamente una lista di film.
- Se si chiedono molti risultati, la risposta puo arrivare progressivamente, conservando il totale richiesto e senza troncamenti silenziosi.
- Quando le corrispondenze certe sono meno di quelle chieste, il sistema lo dice. Le alternative meno aderenti compaiono solo se l'utente le accetta e rimangono separate.
- Una richiesta ambigua riceve una domanda breve solo quando l'ambiguita cambia materialmente il risultato. Il normale dialogo non deve diventare un modulo.
- Le risposte evitano spoiler, salvo richiesta esplicita.

## 2. Comprendere il significato prima di cercare

Costruire un'intenzione completa composta da: scopo della domanda, quantita, requisiti indispensabili, esclusioni, preferenze, relazioni narrative, contesto di visione e formato desiderato. Ogni requisito conserva il riferimento alle parole dell'utente, cosi si possono distinguere richieste esplicite e assunzioni.

I requisiti non sono soltanto parole chiave. Esempio: «amicizia fra un cane e un gatto» richiede due personaggi di quelle specie e un rapporto di amicizia significativo. La semplice presenza di entrambi gli animali non basta. Rivalita, comparsa marginale o due storie separate non sono equivalenti.

«Spooky per Halloween con la mia ragazza, ma non horror» richiede atmosfera adatta e un'esperienza compatibile con il tono desiderato. Non implica automaticamente film romantici, film per bambini o storie ambientate il 31 ottobre. L'assenza dell'etichetta Horror non dimostra da sola che un film sia una visione leggera.

Anno, durata, voto, fonte del voto, paese, lingua e disponibilita hanno significati separati. «IMDb sopra 8» non si sostituisce con TMDB sopra 8. I comparatori numerici mantengono il significato preciso, senza ridurre tutti gli anni a decenni.

## 3. Memoria esplicita della sessione

Conservare intenzione attiva, preferenze espresse, esclusioni, quantita richiesta, titoli gia mostrati con identificativi canonici, feedback dell'utente e cambi di argomento.

La memoria non si riduce agli ultimi tre messaggi. Un seguito eredita lo stato rilevante, anche dopo molti turni. Una nuova richiesta indipendente apre un nuovo contesto. Le proposte dell'assistente non diventano gusti approvati.

La comprensione del seguito viene completata prima di decidere le esclusioni. «Altri», «altre ancora?», «ne conosci altri?» e parafrasi analoghe devono escludere tutte le proposte precedenti di quella ricerca, anche quando la richiesta viene riconosciuta soltanto dal modello. Nessuna dipendenza da un elenco locale di frasi identiche.

Una richiesta esplicita di tornare a un film precedente puo recuperarlo. Una correzione di durata o tono aggiorna quei requisiti, senza azzerare arbitrariamente gli altri.

## 4. Ricerca aperta, con piu vie di scoperta

Un solo coordinatore AI guida una ricerca a passaggi verificabili:

1. Comprende la richiesta e sceglie le operazioni necessarie.
2. Propone anche possibili titoli dalla conoscenza cinematografica del modello.
3. Cerca in parallelo attraverso titoli, persone, filtri e keyword TMDB, piu il materiale narrativo disponibile.
4. Risolve titoli, anno e identita: il suggerimento del modello e un candidato, non un film gia verificato.
5. Raccoglie evidenze sui requisiti importanti.
6. Valuta i candidati e amplia o riformula la ricerca quando il gruppo e insufficiente o poco vario.
7. Compone la risposta usando soltanto i risultati ammessi.

Le keyword servono a scoprire candidati, non a certificare una relazione narrativa. La chat non e limitata ai primi venti film popolari o alla prima pagina di una ricerca.

Il lavoro resta finito: budget di tempo, richieste ai servizi, token e annullamento. Questi sono limiti operativi interni. Quando si raggiungono, la risposta distingue «ricerca interrotta/incompleta» da «nessuna corrispondenza trovata» e permette di continuare.

## 5. Evidenze e verifica della pertinenza

Per ogni film tenere una scheda di evidenza: identita, metadati, informazioni narrative disponibili, provenienza, data del controllo e valutazione di ogni requisito: soddisfatto, contraddetto o non verificato.

TMDB resta la fonte primaria per identita e metadati. Per relazioni e dettagli di trama servono anche informazioni narrative piu ricche: sinossi estese, fonti editoriali e altre fonti accessibili con condizioni d'uso compatibili. Integrare nell'app un servizio di ricerca documentale/web; gli strumenti disponibili nella chat di sviluppo non diventano automaticamente strumenti di CineMatch. La scelta operativa del fornitore richiede una verifica di disponibilita, licenza e credenziali prima della relativa implementazione.

Usare una cache locale delle informazioni consentite e delle valutazioni, con provenienza e invalidazione. Evitare la costruzione preventiva di un archivio mondiale completo: il sistema deve funzionare anche su film non ancora indicizzati.

La conoscenza del modello aiuta la scoperta. Una seconda valutazione del modello aiuta il controllo, ma non equivale a una fonte indipendente. Una percentuale dichiarata dal modello non prova l'attinenza.

Un requisito indispensabile contraddetto esclude il film. Se non verificabile, il film non viene presentato come corrispondenza certa. Le preferenze qualitative orientano la selezione; il loro grado di soggettivita resta riconoscibile.

## 6. Selezione e variabilita

Ordine di valutazione: requisiti indispensabili ed esclusioni; relazione o situazione richiesta; tono e contesto; qualita della corrispondenza; varieta della selezione. Popolarita e voto non possono compensare una trama estranea.

Il gruppo finale deve variare registi, saghe, epoche e approcci quando questo e compatibile con la richiesta. La diversita non impone di cambiare genere o abbassare l'attinenza. Una richiesta di film di uno stesso regista resta legittima.

Se mancano alternative adeguate, cercare altre fonti e altri candidati. Se la ricerca non ne trova abbastanza, restituire meno titoli con una spiegazione chiara. Nessun riciclo e nessuna perdita progressiva dei vincoli per produrre sempre cinque card.

## 7. Risposta naturale e verificabile

Separare il protocollo interno dalla presentazione. Internamente usare dati strutturati per strumenti, stato ed evidenze; esternamente mostrare prosa, confronti, elenchi o schede secondo la domanda. Eliminare le lunghezze globali imposte a introduzione e motivazioni come regola di prodotto; mantenere soltanto protezioni tecniche contro risposte accidentali smisurate.

Per una raccomandazione spiegare il collegamento specifico con la richiesta, senza ripetere anno e durata gia visibili. Aggiungere un'avvertenza sul tono solo se utile alla scelta. Eliminare consigli riempitivi ripetuti come «Metti in conto N minuti».

I criteri importanti restano consultabili e correggibili, senza costringere l'utente a configurare filtri prima di parlare. Mostrare progressi reali della ricerca e permettere Stop. Le spiegazioni descrivono evidenze e conclusioni utili, non un presunto ragionamento interno completo.

## 8. Applicazione al repository

- Conservare integrazioni TMDB, SIMKL, credenziali, immagini, aggiornamenti e filtri manuali del catalogo quando corretti.
- Sostituire il percorso della chat che riduce tutte le richieste a Facets/Theme e applica il tetto di cinque scelte.
- Spostare il coordinamento della ricerca fuori da App in un modulo dedicato, mantenendo nell'app stato grafico, messaggi e annullamento.
- Rendere espliciti ConversationState, SearchIntent e CandidateEvidence come dati del flusso. Nessuna infrastruttura di agenti multipli necessaria per principio.
- Conservare l'adattatore dei provider, ma verificare il modello con il nuovo benchmark. La qualita del modello e il supporto effettivo agli strumenti sono requisiti da misurare.
- Non sostituire silenziosamente una verifica semantica con il vecchio ordinamento per popolarita quando l'AI o una fonte fallisce.
- Tenere il nuovo motore separato dal percorso attuale durante lo sviluppo e il confronto; effettuare il passaggio quando supera i criteri di accettazione, poi eliminare il percorso superato.

## 9. Sequenza di consegna

1. Congelare i casi di accettazione e riprodurre il test Halloween con i suoi seguiti. Registrare anche esempi negativi e risultati attesi, prima di cambiare i prompt.
2. Implementare stato conversazionale, quantita variabile, corretta gestione delle alternative e risposte non obbligatoriamente a elenco.
3. Implementare scoperta aperta, risoluzione dei titoli e raccolta delle evidenze; verificare il funzionamento delle fonti esterne realmente disponibili.
4. Implementare verifica semantica, selezione, diversificazione e ricerca aggiuntiva quando necessaria.
5. Collegare la nuova presentazione, le correzioni conversazionali e gli stati di ricerca incompleta, dubbio ed errore.
6. Eseguire il confronto ripetibile, correggere le regressioni, verificare l'app nativa e sostituire il vecchio percorso.

Ogni fase deve produrre comportamento eseguibile e prove, non soltanto istruzioni piu lunghe per il modello.

## 10. Criteri di accettazione

Creare almeno 200 richieste e 50 conversazioni a piu turni, separando sviluppo e valutazione finale. Comprendere relazioni narrative, atmosfere, negazioni, preferenze soggettive, richieste molto rare, quantita differenti, correzioni, ambiguita, omonimi, fonti insufficienti e indisponibilita dei servizi.

Casi obbligatori: amicizia cane-gatto contro mera presenza o rivalita; Halloween piacevole senza horror contro inquietudine e dramma; 1, 5 e 12 film; richiesta senza quantita; cinque seguiti con parafrasi di «altri»; modifica di un solo requisito; cambio di argomento; nessuna corrispondenza; informazione mancante; domanda e confronto senza nuove raccomandazioni.

Controlli automatici: nessun duplicato nelle alternative, identita valide, quantita corretta quando ci sono abbastanza film idonei, esclusioni numeriche rispettate, contesto conservato e assenza di allargamenti silenziosi. Controlli semantici: giudizio umano sui risultati e sulle evidenze, confronto fra versioni e riesecuzioni del modello. Usare metriche di pertinenza sul set valutato, senza generalizzarle a ogni richiesta futura.

Il nuovo motore non si considera pronto soltanto perche passano i test Rust. Tutti i casi critici concordati devono superare la valutazione funzionale e semantica; in particolare non deve ripetersi il difetto osservato nel test reale.

## Riferimenti

- TMDB Finding Data: https://developer.themoviedb.org/docs/finding-data
- TMDB Discover Movie: https://developer.themoviedb.org/reference/discover-movie

La documentazione distingue ricerca per titoli, ricerca tramite filtri e risoluzione tramite identificativi. La necessita di una verifica narrativa piu ricca e una conclusione progettuale per i requisiti di CineMatch, non una garanzia fornita da TMDB.
