use cinematch::domain::{constraints, model::Facets, taxonomy::Taxonomy};

#[test]
fn i_temi_scelti_non_vengono_troncati_a_tre() {
    let input = Facets {
        min_votes: Some(2_000_000),
        keywords: vec![
            "friendship".into(),
            "dog".into(),
            "cat".into(),
            "journey".into(),
        ],
        ..Default::default()
    };
    let (clean, notes) = constraints::sanitize(&Taxonomy::load(), &input);
    assert_eq!(clean.keywords, input.keywords);
    assert_eq!(clean.min_votes, input.min_votes);
    assert!(notes.is_empty());
}

#[test]
fn la_tassonomia_ha_identita_e_riferimenti_coerenti() {
    use std::collections::HashSet;
    let tax = Taxonomy::load();
    assert!(tax.countries.len() >= 251);
    assert!(tax.country_name("CA").is_some() && tax.country_name("AU").is_some());
    assert!(
        tax.decade("1880s").is_some()
            && tax.decade("1910s").is_some()
            && tax.decade("2030s").is_some()
    );
    let mut ids = HashSet::new();
    for movement in &tax.movements {
        assert!(
            ids.insert(&movement.id),
            "Corrente duplicata: {}",
            movement.id
        );
        assert!(
            movement.period[0] <= movement.period[1],
            "Periodo invertito: {}",
            movement.id
        );
        for genre in movement
            .genres_core
            .iter()
            .chain(&movement.genres_allowed)
            .chain(&movement.genres_forbidden)
        {
            assert!(
                tax.genre_name(*genre).is_some(),
                "Genere sconosciuto {genre}: {}",
                movement.id
            );
        }
        for country in &movement.countries {
            assert!(
                tax.country_name(country).is_some(),
                "Paese sconosciuto {country}: {}",
                movement.id
            );
        }
    }
    for mood in &tax.moods {
        for movement in &mood.incompatible_movements {
            assert!(
                tax.movement(movement).is_some(),
                "Riferimento sconosciuto {movement}: {}",
                mood.id
            );
        }
    }
}
