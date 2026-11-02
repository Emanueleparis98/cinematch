use cinematch::domain::constraints::{self, to_discover};
use cinematch::domain::model::Facets;
use cinematch::domain::taxonomy::Taxonomy;

fn facets(decades: &[&str], movements: &[&str], genres: &[u32]) -> Facets {
    Facets {
        decades: decades.iter().map(|s| s.to_string()).collect(),
        movements: movements.iter().map(|s| s.to_string()).collect(),
        genres: genres.to_vec(),
        ..Default::default()
    }
}

#[test]
fn criteri_fuori_dalla_ricetta_restano_scelte_esplicite() {
    let tax = Taxonomy::load();
    let f = facets(&["1980s"], &["neorealismo"], &[27]);
    let (clean, messages) = constraints::sanitize(&tax, &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn anni_80_piu_giallo_e_legittimo_ma_ristretto() {
    let tax = Taxonomy::load();
    let f = facets(&["1980s"], &["giallo_italiano"], &[27]);
    let (clean, _) = constraints::sanitize(&tax, &f);
    assert_eq!(clean.movements, vec!["giallo_italiano".to_string()]);
    let range = constraints::year_range(&tax, &clean).unwrap().unwrap();
    assert_eq!(range, (1980, 1982), "il movimento chiude il periodo a 1982");
    let q = to_discover(&tax, &clean, &[], 1).unwrap();
    assert_eq!(
        q.countries,
        vec!["IT".to_string()],
        "paese implicato dal movimento"
    );
    assert!(
        q.without_genres.is_empty(),
        "nessuna esclusione editoriale implicita"
    );
    assert_eq!(q.genres, vec![27], "Horror esplicito resta obbligatorio");
    assert!(!q.genres_or, "il movimento non allarga un genere richiesto");
}

#[test]
fn genere_estraneo_al_movimento_resta_un_filtro_stretto() {
    let tax = Taxonomy::load();
    // La commedia (35) non e un'etichetta del neorealismo (che ha solo Dramma):
    // resta quindi una condizione in AND, non viene aperta a OR.
    let f = facets(&[], &["neorealismo"], &[35]);
    let (clean, _) = constraints::sanitize(&tax, &f);
    let q = to_discover(&tax, &clean, &[], 1).unwrap();
    assert_eq!(
        q.genres,
        vec![35],
        "nessuna etichetta del movimento aggiunta"
    );
    assert!(!q.genres_or, "il genere estraneo non allarga la query");
}

#[test]
fn il_movimento_non_aggiunge_generi_impliciti() {
    let q = to_discover(
        &Taxonomy::load(),
        &facets(&[], &["giallo_italiano"], &[]),
        &[],
        1,
    )
    .unwrap();
    assert!(q.genres.is_empty());
    assert!(q.without_genres.is_empty());
}

#[test]
fn un_genere_non_viene_eliminato_dalla_corrente() {
    let mut f = facets(&[], &["neorealismo"], &[27]);
    f.moods = vec![];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn il_decennio_non_rimuove_una_corrente() {
    let mut f = facets(&["1970s"], &["nouvelle_vague"], &[]);
    f.moods = vec![];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn la_tassonomia_non_vieta_fantasy() {
    let mut f = facets(&[], &["dogme95"], &[14]);
    f.moods = vec![];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn un_mood_non_viene_cancellato_da_una_generalizzazione() {
    let mut f = facets(&[], &["neorealismo"], &[]);
    f.moods = vec!["soprannaturale".into()];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn si_possono_selezionare_correnti_di_periodi_diversi() {
    let mut f = facets(&[], &["neorealismo", "new_hollywood"], &[]);
    f.moods = vec![];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}

#[test]
fn le_correnti_a_parola_chiave_dichiarano_la_keyword() {
    let tax = Taxonomy::load();
    let m = tax
        .movement("slasher_era")
        .expect("slasher_era e in tassonomia");
    assert_eq!(
        m.keyword, "slasher",
        "la corrente slasher dichiara la sua keyword"
    );
    // le correnti senza keyword si cercano per geometria (periodo e paese)
    let plain = tax
        .movement("neorealismo")
        .expect("neorealismo e in tassonomia");
    assert!(plain.keyword.is_empty());
}

#[test]
fn una_corrente_a_parola_chiave_interroga_tmdb_con_la_keyword() {
    let tax = Taxonomy::load();
    let Some(id) = tax
        .movements
        .iter()
        .find(|m| !m.keyword.is_empty())
        .map(|m| m.id.clone())
    else {
        return; // nessuna corrente a keyword in questo build
    };
    let f = facets(&[], &[id.as_str()], &[]);
    // con l'id TMDB risolto la keyword entra nella query, in OR
    let known = vec![(id.clone(), 12339u64)];
    let q = to_discover(&tax, &f, &known, 1).unwrap();
    assert!(
        q.keywords.contains(&12339),
        "la keyword della corrente deve entrare nella query"
    );
    assert!(
        q.keywords_or,
        "la keyword della corrente e un'identita, non un requisito alternativo"
    );
    assert!(
        q.countries.contains(&"US".to_string()),
        "il paese della corrente resta un filtro"
    );
    // senza id risolto la ricerca ripiega sulla geometria, senza keyword
    let q2 = to_discover(&tax, &f, &[], 1).unwrap();
    assert!(q2.keywords.is_empty());
}

#[test]
fn le_correnti_a_keyword_si_risolvono_come_i_mood() {
    let tax = Taxonomy::load();
    let Some(id) = tax
        .movements
        .iter()
        .find(|m| !m.keyword.is_empty())
        .map(|m| m.id.clone())
    else {
        return;
    };
    let f = facets(&[], &[id.as_str()], &[]);
    let unresolved = constraints::unresolved_movement_keywords(&tax, &f, &[]);
    assert_eq!(unresolved.len(), 1, "la corrente va risolta una volta sola");
    assert_eq!(unresolved[0].0, id);
    // una volta risolta non si richiede piu
    let known = vec![(id.clone(), 42u64)];
    assert!(constraints::unresolved_movement_keywords(&tax, &f, &known).is_empty());
}

#[test]
fn le_parole_chiave_tematiche_restano_strette_vicino_a_una_corrente() {
    let tax = Taxonomy::load();
    let Some(id) = tax
        .movements
        .iter()
        .find(|m| !m.keyword.is_empty())
        .map(|m| m.id.clone())
    else {
        return;
    };
    let mut f = facets(&[], &[id.as_str()], &[]);
    f.moods.push("malinconico".to_string());
    let known = vec![("malinconico".to_string(), 12345u64), (id.clone(), 999u64)];
    let q = to_discover(&tax, &f, &known, 1).unwrap();
    assert!(q.keywords.contains(&12345), "il tema entra");
    assert!(q.keywords.contains(&999), "la keyword della corrente entra");
    assert!(
        !q.keywords_or,
        "con un tema in mezzo si torna all'AND: il tema e un requisito, non un'alternativa"
    );
}

#[test]
fn due_decenni_non_contigui_vengono_dichiarati() {
    let tax = Taxonomy::load();
    assert!(constraints::decade_gap(
        &tax,
        &facets(&["1970s", "1990s"], &[], &[])
    ));
    assert!(!constraints::decade_gap(
        &tax,
        &facets(&["1970s", "1980s"], &[], &[])
    ));
}

#[test]
fn un_genere_escluso_dall_utente_arriva_a_tmdb() {
    let tax = Taxonomy::load();
    let f = Facets {
        without_genres: vec![35],
        ..Default::default()
    };
    let q = to_discover(&tax, &f, &[], 1).unwrap();
    assert!(
        q.without_genres.contains(&35),
        "«senza commedia» deve finire in without_genres"
    );
}

#[test]
fn voto_minimo_massimo_e_voti_arrivano_alla_query() {
    let tax = Taxonomy::load();
    let f = Facets {
        genres: vec![18],
        min_rating: Some(6.5),
        max_rating: Some(8.5),
        min_votes: Some(5000),
        ..Default::default()
    };
    let q = to_discover(&tax, &f, &[], 1).unwrap();
    assert_eq!(q.min_rating, Some(6.5));
    assert_eq!(q.max_rating, Some(8.5));
    assert_eq!(q.min_votes, Some(5000));
}

#[test]
fn voto_invertito_viene_rimesso_in_ordine() {
    let tax = Taxonomy::load();
    let f = Facets {
        min_rating: Some(9.0),
        max_rating: Some(5.0),
        ..Default::default()
    };
    let (clean, msgs) = constraints::sanitize(&tax, &f);
    assert_eq!(clean.min_rating, Some(5.0));
    assert_eq!(clean.max_rating, Some(9.0));
    assert!(!msgs.is_empty(), "lo scambio va spiegato");
}

#[test]
fn tema_libero_e_un_requisito_e_non_un_alternativa() {
    let tax = Taxonomy::load();
    let f = Facets {
        keywords: vec!["resistenza".to_string()],
        moods: vec!["critica_sociale".to_string()],
        ..Default::default()
    };
    let (clean, _) = constraints::sanitize(&tax, &f);
    let known = vec![
        ("critica_sociale".to_string(), 111u64),
        ("resistenza".to_string(), 222u64),
    ];
    let q = to_discover(&tax, &clean, &known, 1).unwrap();
    assert!(q.keywords.contains(&111), "il mood entra");
    assert!(q.keywords.contains(&222), "il tema libero entra");
    assert!(!q.keywords_or, "due argomenti insieme si chiedono in AND");
}

#[test]
fn i_temi_validi_restano_anche_quando_sono_piu_di_tre() {
    let tax = Taxonomy::load();
    let f = Facets {
        keywords: vec![
            "resistenza".to_string(),
            "resistenza".to_string(),
            " ".to_string(),
            "motociclismo".to_string(),
            "vendetta".to_string(),
            "prigione".to_string(),
        ],
        ..Default::default()
    };
    let (clean, msgs) = constraints::sanitize(&tax, &f);
    assert_eq!(
        clean.keywords.len(),
        4,
        "nessun tema valido viene eliminato"
    );
    assert!(!msgs.is_empty(), "la potatura va spiegata");
}

#[test]
fn la_ricetta_del_neorealismo_sono_gli_anni_40_e_50_italiani() {
    let tax = Taxonomy::load();
    let m = tax
        .movement("neorealismo")
        .expect("il neorealismo e in tassonomia");
    let (decades, countries, genres) = constraints::movement_recipe(&tax, m);
    assert_eq!(decades, vec!["1940s".to_string(), "1950s".to_string()]);
    assert_eq!(countries, vec!["IT".to_string()]);
    assert!(!genres.is_empty(), "la ricetta mostra i generi tipici");
}

#[test]
fn si_conservano_tutte_le_correnti_selezionate() {
    let mut f = facets(
        &[],
        &["new_hollywood", "nouvelle_vague", "slasher_era"],
        &[],
    );
    f.moods = vec![];
    let (clean, messages) = constraints::sanitize(&Taxonomy::load(), &f);
    assert_eq!(clean, f);
    assert!(messages.is_empty());
}
