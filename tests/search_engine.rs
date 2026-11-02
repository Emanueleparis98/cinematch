//! Prove delle garanzie del coordinatore, con fonti e modello controllati.
//! Non sono misure della pertinenza semantica del modello live.
use cinematch::{
    ai::{Intent, IntentProvider},
    domain::{
        model::{Details, Movie},
        taxonomy::Taxonomy,
    },
    search::*,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

struct Films {
    unavailable: bool,
}
impl MovieSource for Films {
    fn resolve(&self, seed: &Seed) -> Result<Vec<Movie>, String> {
        let id = seed
            .title
            .strip_prefix("Film ")
            .unwrap_or(&seed.title)
            .parse()
            .unwrap_or(0);
        Ok(vec![Movie {
            id,
            title: format!("Film {id}"),
            year: Some(1995),
            ..Default::default()
        }])
    }
    fn discover(
        &self,
        _: &Discovery,
        _: u32,
        _: &SearchIntent,
        _: &AtomicBool,
    ) -> Result<Vec<Movie>, String> {
        Ok(Vec::new())
    }
    fn evidence(&self, id: u64, _: &AtomicBool) -> Result<CandidateEvidence, String> {
        if self.unavailable {
            return Err("fonte offline".into());
        }
        let movie = Movie {
            id,
            title: format!("Film {id}"),
            year: Some(1995),
            overview: "A dog and a cat become close friends during their journey.".into(),
            votes: 1000,
            rating: 7.6,
            ..Default::default()
        };
        Ok(CandidateEvidence {
            documents: vec![Document::tmdb(&movie)],
            details: Details {
                movie,
                runtime: Some(90),
                director: Some(format!("Director {}", id % 4)),
                ..Default::default()
            },
            warnings: Vec::new(),
        })
    }
}
struct Model {
    plan: Mutex<Value>,
    mode: &'static str,
    calls: AtomicUsize,
}
impl Model {
    fn new(count: usize) -> Self {
        Self {
            plan: Mutex::new(plan("new", count)),
            mode: "valid",
            calls: AtomicUsize::new(0),
        }
    }
}
fn plan(action: &str, count: usize) -> Value {
    json!({"action":action,"intent":{"request":"Amicizia significativa fra un cane e un gatto","count":count,
        "requirements":[{"text":"Cane e gatto amici","origin":"amicizia cane-gatto","essential":true}],"constraints":[]},
        "reply":"Una risposta testuale utile.","seeds":(1..=60).map(|i|json!({"title":i.to_string(),"year":1995})).collect::<Vec<_>>(),"discovery":[]})
}
impl IntentProvider for Model {
    fn label(&self) -> String {
        "modello controllato".into()
    }
    fn parse(&self, _: &str, _: &Taxonomy) -> Result<Intent, String> {
        unreachable!()
    }
    fn research_json(
        &self,
        system: &str,
        data: &Value,
        _: u32,
        _: &AtomicBool,
    ) -> Result<Value, String> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        if system.contains("Schema:") {
            if self.mode == "bad_protocol" || (self.mode == "repair_once" && call == 0) {
                return Ok(json!({"action":"new","intent":{}}));
            }
            return Ok(self.plan.lock().unwrap().clone());
        }
        if system.contains("ulteriori candidati") {
            return Ok(json!({"seeds":[],"discovery":[]}));
        }
        if system.contains("introduzione naturale") {
            return Ok(json!({"intro":"Questi film sviluppano il legame richiesto."}));
        }
        if system.contains("risposta documentata") {
            return Ok(json!({"reply":"I dati verificati indicano una durata di 90 minuti."}));
        }
        if system.contains("revisore finale") {
            return Ok(
                json!({"reviews":data["candidates"].as_array().unwrap().iter().map(|c|json!({"id":c["id"],"decision":if self.mode=="partial_fit" {"partial"} else {"fit"},"explanation":"Il rapporto di amicizia fra il cane e il gatto guida la storia, come richiesto."})).collect::<Vec<_>>()}),
            );
        }
        if self.mode == "failure" {
            return Err("servizio AI offline".into());
        }
        let mut judgments = Vec::new();
        for film in data["candidates"].as_array().unwrap() {
            let id = film["id"].as_u64().unwrap();
            let quote = match self.mode {
                "fabricated" => "A fabricated event that is not present in the source",
                _ => "A dog and a cat become close friends",
            };
            let status = match self.mode {
                "unknown" => "unknown",
                "contradicted" => "contradicted",
                _ => "supported",
            };
            let criterion = if self.mode == "missing" { 99 } else { 0 };
            let source = if self.mode == "invalid_source" { 9 } else { 0 };
            let j = json!({"id":id,"assessments":[{"criterion":criterion,"status":status,"source":source,"quote":quote}],"why":"Il rapporto di amicizia tra le specie guida il viaggio.","caveat":""});
            judgments.push(j.clone());
            judgments.push(j);
        }
        judgments.push(json!({"id":99999,"assessments":[],"why":"Titolo inventato"}));
        Ok(json!({"judgments":judgments}))
    }
}
fn execute(model: &Model, state: &ConversationState) -> Result<Response, String> {
    run(
        model,
        &Films { unavailable: false },
        "richiesta",
        state,
        None,
        &AtomicBool::new(false),
        &mut |_| {},
    )
}

#[test]
fn requested_quantities_are_exact_and_not_limited_to_five() {
    for count in [1, 2, 5, 6, 8, 12, 20, 24] {
        let r = execute(&Model::new(count), &ConversationState::default()).unwrap();
        assert_eq!(r.selections.len(), count);
        assert_eq!(r.state.shown.len(), count);
        assert!(r.notice.is_none());
    }
}

#[test]
fn literal_evidence_is_not_sufficient_when_the_blind_reviewer_finds_only_a_partial_match() {
    let mut m = Model::new(5);
    m.mode = "partial_fit";
    let r = execute(&m, &ConversationState::default()).unwrap();
    assert!(r.selections.is_empty());
    assert!(r.state.verified.is_empty());
    assert!(r.notice.is_some());
}

#[test]
fn an_unchanged_goal_cannot_erase_seen_ids_through_a_wrong_new_label() {
    let m = Model::new(1);
    let first = execute(&m, &ConversationState::default()).unwrap();
    for action in ["new", "refine"] {
        *m.plan.lock().unwrap() = plan(action, 1);
        let r = execute(&m, &first.state).unwrap();
        assert_eq!(r.selections.len(), 1);
        assert!(!first
            .state
            .shown
            .contains(&r.selections[0].evidence.details.movie.id));
        assert!(first.state.shown.is_subset(&r.state.shown));
    }
}

#[test]
fn quality_is_a_default_tie_breaker_after_semantic_admission_and_explicit_sort_is_respected() {
    let high = Details {
        movie: Movie {
            rating: 8.5,
            votes: 10000,
            year: Some(1985),
            ..Default::default()
        },
        ..Default::default()
    };
    let low = Details {
        movie: Movie {
            rating: 5.0,
            votes: 10000,
            year: Some(2024),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut intent = SearchIntent::default();
    assert!(ranking_score(&intent, &high) > ranking_score(&intent, &low));
    intent.ranking = Some(Ranking::QualityLow);
    assert!(ranking_score(&intent, &low) > ranking_score(&intent, &high));
    intent.ranking = Some(Ranking::Newest);
    assert!(ranking_score(&intent, &low) > ranking_score(&intent, &high));
}
#[test]
fn large_requests_are_progressive_with_original_total_preserved() {
    let m = Model::new(50);
    let first = execute(&m, &ConversationState::default()).unwrap();
    assert_eq!(first.selections.len(), 24);
    assert_eq!(first.state.pending, 26);
    *m.plan.lock().unwrap() = plan("more", 50);
    let second = execute(&m, &first.state).unwrap();
    assert_eq!(second.selections.len(), 24);
    assert_eq!(second.state.pending, 2);
    let third = execute(&m, &second.state).unwrap();
    assert_eq!(third.selections.len(), 2);
    assert_eq!(third.state.pending, 0);
    assert_eq!(third.state.shown.len(), 50);
}
#[test]
fn semantic_more_excludes_history_after_interpretation_for_every_paraphrase() {
    let m = Model::new(5);
    let mut state = execute(&m, &ConversationState::default()).unwrap().state;
    *m.plan.lock().unwrap() = plan("more", 5);
    for user in [
        "altre proposte?",
        "altre ancora?",
        "ne conosci altri?",
        "cambia completamente i titoli",
        "cos'altro mi consiglieresti?",
    ] {
        let r = run(
            &m,
            &Films { unavailable: false },
            user,
            &state,
            None,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(r.selections.len(), 5);
        for s in &r.selections {
            assert!(!state.shown.contains(&s.evidence.details.movie.id));
        }
        state = r.state;
    }
    assert_eq!(state.shown.len(), 30);
    assert_eq!(state.turns.len(), 6);
}
#[test]
fn unsupported_or_fabricated_evidence_never_becomes_a_recommendation() {
    for mode in [
        "fabricated",
        "unknown",
        "contradicted",
        "missing",
        "invalid_source",
    ] {
        let mut m = Model::new(5);
        m.mode = mode;
        let r = execute(&m, &ConversationState::default()).unwrap();
        assert!(r.selections.is_empty(), "{mode}");
        assert!(r.notice.is_some());
        assert_eq!(r.state.active.count, Some(5));
    }
}
#[test]
fn ai_failure_does_not_fall_back_to_popularity() {
    let mut m = Model::new(5);
    m.mode = "failure";
    assert!(execute(&m, &ConversationState::default())
        .unwrap_err()
        .contains("offline"));
}
#[test]
fn questions_and_comparisons_do_not_search_or_add_cards() {
    let m = Model::new(5);
    *m.plan.lock().unwrap() = plan("answer", 5);
    m.plan.lock().unwrap()["seeds"] = json!([]);
    let r = execute(&m, &ConversationState::default()).unwrap();
    assert!(r.selections.is_empty());
    assert_eq!(r.text, "Una risposta testuale utile.");
    assert!(r.understood.is_empty());
    assert_eq!(m.calls.load(Ordering::Relaxed), 1);
}
#[test]
fn factual_questions_resolve_identities_and_sources_without_recommendation_cards() {
    let m = Model::new(5);
    *m.plan.lock().unwrap() = plan("answer", 5);
    m.plan.lock().unwrap()["seeds"] = json!([{"title":"1","year":1995}]);
    let r = execute(&m, &ConversationState::default()).unwrap();
    assert!(r.selections.is_empty());
    assert!(!r.references.is_empty());
    assert!(r.text.contains("90 minuti"));
    assert_eq!(m.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn factual_comparisons_are_not_silently_limited_to_four_titles() {
    let m = Model::new(5);
    *m.plan.lock().unwrap() = plan("answer", 5);
    m.plan.lock().unwrap()["seeds"] = json!((1..=5)
        .map(|id| json!({"title":id.to_string(),"year":1995}))
        .collect::<Vec<_>>());
    let r = execute(&m, &ConversationState::default()).unwrap();
    assert_eq!(r.references.len(), 5);
    assert!(r.selections.is_empty());
}
#[test]
fn cancellation_prevents_even_the_first_model_call() {
    let m = Model::new(5);
    assert!(run(
        &m,
        &Films { unavailable: false },
        "test",
        &ConversationState::default(),
        None,
        &AtomicBool::new(true),
        &mut |_| {}
    )
    .is_err());
    assert_eq!(m.calls.load(Ordering::Relaxed), 0);
}
#[test]
fn new_topic_resets_ids_but_refinement_preserves_the_conversation() {
    let m = Model::new(5);
    let state = execute(&m, &ConversationState::default()).unwrap().state;
    *m.plan.lock().unwrap() = plan("new", 1);
    let new = execute(&m, &state).unwrap();
    assert_eq!(new.state.shown.len(), 1);
    assert_eq!(new.state.turns.len(), 2);
    *m.plan.lock().unwrap() = plan("refine", 1);
    let refined = execute(&m, &state).unwrap();
    assert!(state.shown.is_subset(&refined.state.shown));
    assert_eq!(refined.selections.len(), 1);
    *m.plan.lock().unwrap() = plan("revisit", 1);
    let revisit = execute(&m, &state).unwrap();
    assert!(state
        .shown
        .contains(&revisit.selections[0].evidence.details.movie.id));
}
#[test]
fn unavailable_sources_are_not_reported_as_exhaustion_or_relaxed() {
    let m = Model::new(5);
    let r = run(
        &m,
        &Films { unavailable: true },
        "test",
        &ConversationState::default(),
        None,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert!(r.selections.is_empty());
    assert!(!r.notes.is_empty());
    assert!(r.notice.unwrap().contains("continua"));
    assert_eq!(r.state.active.count, Some(5));
}
#[test]
fn numeric_constraints_keep_source_comparators_and_exact_years() {
    let mut d = Details {
        movie: Movie {
            year: Some(1995),
            rating: 8.5,
            votes: 100,
            ..Default::default()
        },
        runtime: Some(90),
        ..Default::default()
    };
    let mut c = Constraint {
        any_group: None,
        field: Field::ImdbRating,
        op: Operator::Gt,
        number: Some(8.0),
        value: String::new(),
        origin: "IMDb sopra 8".into(),
    };
    assert!(!matches_constraints(&d, &[c.clone()]));
    d.imdb = Some(cinematch::domain::model::ImdbRating {
        rating: 8.0,
        votes: 100,
    });
    assert!(!matches_constraints(&d, &[c.clone()]));
    c.op = Operator::Gte;
    assert!(matches_constraints(&d, &[c.clone()]));
    c.field = Field::Year;
    c.number = Some(1996.0);
    assert!(!matches_constraints(&d, &[c.clone()]));
    c.number = Some(1995.0);
    c.op = Operator::Eq;
    assert!(matches_constraints(&d, &[c]));
}

#[test]
fn quantity_survives_long_conversations_until_explicitly_changed_or_cleared() {
    let m = Model::new(12);
    let mut state = execute(&m, &ConversationState::default()).unwrap().state;
    for _ in 0..20 {
        let mut p = plan("refine", 12);
        p["intent"]["count"] = Value::Null;
        let parsed: Plan = serde_json::from_value(p).unwrap();
        state = transition(&state, &parsed);
        assert_eq!(state.active.count, Some(12));
    }
    let mut p = plan("refine", 12);
    p["intent"]["count"] = Value::Null;
    p["clear_count"] = json!(true);
    let parsed: Plan = serde_json::from_value(p).unwrap();
    assert_eq!(transition(&state, &parsed).active.count, None);
}

#[test]
fn malformed_protocol_gets_one_repair_then_fails_explicitly() {
    let mut m = Model::new(5);
    m.mode = "repair_once";
    assert_eq!(
        execute(&m, &ConversationState::default())
            .unwrap()
            .selections
            .len(),
        5
    );
    m.mode = "bad_protocol";
    m.calls.store(0, Ordering::Relaxed);
    assert!(execute(&m, &ConversationState::default()).is_err());
    assert_eq!(m.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn explicitly_excluded_titles_are_resolved_to_canonical_ids() {
    let m = Model::new(12);
    m.plan.lock().unwrap()["intent"]["constraints"] =
        json!([{"field":"title","op":"excludes","value":"1","origin":"Ho già visto questo film"}]);
    let r = execute(&m, &ConversationState::default()).unwrap();
    assert_eq!(r.selections.len(), 12);
    assert!(r
        .selections
        .iter()
        .all(|s| s.evidence.details.movie.id != 1));
}

#[test]
fn eligible_candidates_not_yet_shown_remain_available_for_later_alternatives() {
    let m = Model::new(1);
    m.plan.lock().unwrap()["seeds"] = json!((1..=6)
        .map(|id| json!({"title":id.to_string(),"year":1995}))
        .collect::<Vec<_>>());
    let mut state = ConversationState::default();
    for i in 0..6 {
        if i > 0 {
            m.plan.lock().unwrap()["action"] = json!("more");
        }
        let r = execute(&m, &state).unwrap();
        assert_eq!(r.selections.len(), 1, "alternativa {}", i + 1);
        assert!(!state
            .shown
            .contains(&r.selections[0].evidence.details.movie.id));
        state = r.state;
    }
    assert_eq!(state.shown.len(), 6);
}

#[test]
fn omitted_quantities_are_read_without_confusing_years_or_characters() {
    let m = Model::new(5);
    m.plan.lock().unwrap()["intent"]["count"] = Value::Null;
    for (user, count) in [
        ("Consigliami un film", Some(1)),
        ("Dodici film sui paradossi temporali", Some(12)),
        ("Una dozzina di film di avventura", Some(12)),
        ("24 titoli", Some(24)),
        ("Film su un detective nel 1997", None),
        ("Dal 1993 al 1997 film italiani", None),
    ] {
        let p = understand(
            &m,
            user,
            &ConversationState::default(),
            None,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(p.intent.count, count, "{user}");
    }
}

#[test]
fn optional_null_fields_and_numeric_genre_identifiers_are_handled_safely() {
    let c: Constraint = serde_json::from_value(
        json!({"field":"runtime","op":"gte","number":60,"value":null,"origin":"film"}),
    )
    .unwrap();
    assert_eq!(c.value, "");
    let d = Details {
        genres: vec![(27, "Nome localizzato".into())],
        ..Default::default()
    };
    let exclusion = Constraint {
        any_group: None,
        field: Field::Genre,
        op: Operator::Excludes,
        number: Some(27.0),
        value: "Horror".into(),
        origin: "non horror".into(),
    };
    assert!(!matches_constraints(&d, &[exclusion]));
    let unknown = Constraint {
        any_group: None,
        field: Field::Country,
        op: Operator::Excludes,
        number: None,
        value: "Italy".into(),
        origin: "non italiani".into(),
    };
    assert!(!matches_constraints(
        &Details {
            country_codes: vec!["IT".into()],
            ..Default::default()
        },
        &[unknown]
    ));
}

#[test]
fn changed_filters_are_refinements_even_if_the_model_calls_them_more() {
    let m = Model::new(5);
    let first = execute(&m, &ConversationState::default()).unwrap();
    *m.plan.lock().unwrap() = plan("more", 5);
    m.plan.lock().unwrap()["intent"]["constraints"] = json!([{"field":"runtime","op":"lte","number":100,"value":null,"origin":"sotto 100 minuti"}]);
    let p = understand(
        &m,
        "Aggiorna il limite",
        &first.state,
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(p.action, Action::Refine);
    assert!(transition(&first.state, &p).verified.is_empty());
}

#[test]
fn verified_unshown_results_can_be_reused_without_network_or_another_judgment() {
    struct NoNetwork;
    impl MovieSource for NoNetwork {
        fn resolve(&self, _: &Seed) -> Result<Vec<Movie>, String> {
            panic!("no new resolution necessary")
        }
        fn discover(
            &self,
            _: &Discovery,
            _: u32,
            _: &SearchIntent,
            _: &AtomicBool,
        ) -> Result<Vec<Movie>, String> {
            panic!("no new discovery necessary")
        }
        fn evidence(&self, _: u64, _: &AtomicBool) -> Result<CandidateEvidence, String> {
            panic!("no new evidence necessary")
        }
    }
    let m = Model::new(1);
    let first = execute(&m, &ConversationState::default()).unwrap();
    assert!(!first.state.verified.is_empty());
    *m.plan.lock().unwrap() = plan("more", 1);
    let r = run(
        &m,
        &NoNetwork,
        "altre ancora?",
        &first.state,
        None,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(r.selections.len(), 1);
    assert!(!first
        .state
        .shown
        .contains(&r.selections[0].evidence.details.movie.id));
}

#[test]
fn alternatives_in_metadata_are_or_while_independent_requirements_remain_and() {
    let mut d = Details {
        movie: Movie {
            rating: 8.5,
            votes: 1000,
            lang: "en".into(),
            ..Default::default()
        },
        country_codes: vec!["IT".into()],
        ..Default::default()
    };
    let constraints:Vec<Constraint>=serde_json::from_value(json!([
        {"field":"country","op":"includes","value":"IT","origin":"italiani o francesi","any_group":"countries"},
        {"field":"country","op":"includes","value":"FR","origin":"italiani o francesi","any_group":"countries"},
        {"field":"imdb_rating","op":"gt","number":8,"origin":"IMDb o TMDB sopra 8","any_group":"rating"},
        {"field":"tmdb_rating","op":"gt","number":8,"origin":"IMDb o TMDB sopra 8","any_group":"rating"}
    ])).unwrap();
    assert!(matches_constraints(&d, &constraints));
    d.movie.rating = 7.0;
    assert!(!matches_constraints(&d, &constraints));
    d.movie.rating = 8.5;
    d.country_codes = vec!["US".into()];
    assert!(!matches_constraints(&d, &constraints));
}

#[test]
fn supporting_cast_and_joint_directors_are_searchable_beyond_the_display_summary() {
    let d = Details {
        cast: vec!["Protagonista".into()],
        cast_full: vec!["Protagonista".into(), "Interprete secondario".into()],
        director: Some("Regista uno".into()),
        directors: vec!["Regista uno".into(), "Regista due".into()],
        ..Default::default()
    };
    let constraints:Vec<Constraint>=serde_json::from_value(json!([
        {"field":"actor","op":"includes","value":"Interprete secondario","origin":"con Interprete secondario"},
        {"field":"director","op":"includes","value":"Regista due","origin":"diretto da Regista due"}
    ])).unwrap();
    assert!(matches_constraints(&d, &constraints));
}

#[test]
fn strict_tmdb_thresholds_do_not_round_away_real_decimal_differences() {
    let d = Details {
        movie: Movie {
            rating: 8.04,
            votes: 1000,
            ..Default::default()
        },
        ..Default::default()
    };
    let c: Constraint = serde_json::from_value(
        json!({"field":"tmdb_rating","op":"gt","number":8,"origin":"TMDB sopra 8"}),
    )
    .unwrap();
    assert!(matches_constraints(&d, &[c]));
}

#[test]
fn discovery_lanes_are_not_starved_by_a_long_model_seed_list() {
    struct ExtraLane;
    impl MovieSource for ExtraLane {
        fn resolve(&self, s: &Seed) -> Result<Vec<Movie>, String> {
            Films { unavailable: false }.resolve(s)
        }
        fn discover(
            &self,
            _: &Discovery,
            _: u32,
            _: &SearchIntent,
            _: &AtomicBool,
        ) -> Result<Vec<Movie>, String> {
            Ok(vec![Movie {
                id: 100,
                title: "Film 100".into(),
                ..Default::default()
            }])
        }
        fn evidence(&self, id: u64, c: &AtomicBool) -> Result<CandidateEvidence, String> {
            Films { unavailable: false }.evidence(id, c)
        }
    }
    let m = Model::new(1);
    m.plan.lock().unwrap()["discovery"] = json!([{"kind":"keyword","query":"friendship"}]);
    let r = run(
        &m,
        &ExtraLane,
        "richiesta",
        &ConversationState::default(),
        None,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert!(
        r.state.verified.contains_key(&100),
        "La seconda fonte deve essere verificata già nel primo gruppo, non dopo 60 seed"
    );
}

#[test]
fn evaluation_corpus_has_independent_topics_and_unique_conversations() {
    use std::collections::HashSet;
    let corpus: Value = serde_json::from_str(include_str!("fixtures/search-corpus.json")).unwrap();
    assert_eq!(corpus["requests"].as_array().unwrap().len(), 200);
    let conversations = corpus["conversations"].as_array().unwrap();
    assert_eq!(conversations.len(), 50);
    let unique: HashSet<_> = conversations
        .iter()
        .map(|c| c["turns"].to_string())
        .collect();
    assert_eq!(unique.len(), 50);
    let scenarios: Vec<_> = corpus["requests"]
        .as_array()
        .unwrap()
        .iter()
        .chain(conversations)
        .collect();
    let topics = |split: &str| {
        scenarios
            .iter()
            .filter(|s| s["split"] == split)
            .map(|s| s["semantic_required"].as_str().unwrap())
            .collect::<HashSet<_>>()
    };
    assert!(topics("development").is_disjoint(&topics("held_out")));
}
