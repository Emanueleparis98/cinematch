//! Esegue il motore della UI, con credenziali locali mai stampate.
//! cargo run --example chat_probe -- "5 film ..." "altre ancora?"
//! --plan-only misura la comprensione, --critical esegue il corpus critico.
use cinematch::{ai::build_provider, infra::config::Config, search, tmdb::Tmdb};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn main() {
    if let Err(error) = execute() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn execute() -> Result<(), String> {
    let cfg = Config::load();
    if std::env::args().any(|a| a == "--provider-status") {
        let llm = cinematch::ai::llm_config(&cfg.ai).ok_or("Nessun provider AI configurato")?;
        if llm.base_url != cinematch::ai::OPENROUTER_BASE {
            return Err("Il controllo crediti è disponibile per OpenRouter.".into());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        for endpoint in ["key", "credits"] {
            let response = client
                .get(format!("https://openrouter.ai/api/v1/{endpoint}"))
                .bearer_auth(&llm.api_key)
                .send()
                .map_err(|_| "Provider non raggiungibile")?;
            if !response.status().is_success() {
                println!("{endpoint}: HTTP {}", response.status().as_u16());
                continue;
            }
            let v: Value = response.json().map_err(|_| "Stato account non leggibile")?;
            let data = &v["data"];
            if endpoint == "key" {
                println!("{}", json!({"key_limit_remaining":data["limit_remaining"]}));
            } else {
                println!(
                    "{}",
                    json!({"account_credits_remaining":data["total_credits"].as_f64().zip(data["total_usage"].as_f64()).map(|(a,b)|a-b)})
                );
            }
        }
        return Ok(());
    }
    let provider = build_provider(&cfg.ai);
    let tmdb = Tmdb::new(cfg.tmdb_api_key.clone(), cfg.language.clone());
    let source = search::TmdbSource::new(&tmdb)?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let plan_only = args.iter().any(|s| s == "--plan-only");
    let critical = args.iter().any(|s| s == "--critical");
    let scenarios: Vec<Value> = if critical {
        serde_json::from_str(include_str!("../tests/fixtures/search-critical.json"))
            .map_err(|e| e.to_string())?
    } else {
        vec![
            json!({"id":"manual","turns":args.iter().filter(|a|!a.starts_with("--")).collect::<Vec<_>>()}),
        ]
    };
    let mut report = Vec::new();
    let mut failures = Vec::new();
    for scenario in scenarios {
        let mut seen = std::collections::HashSet::new();
        let mut state = search::ConversationState::default();
        for user in scenario["turns"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let started = Instant::now();
            let cancel = AtomicBool::new(false);
            if plan_only {
                let plan = search::understand(provider.as_ref(), user, &state, None, &cancel)?;
                state = search::transition(&state, &plan);
                state.turns.push(search::Turn {
                    user: user.into(),
                    response: plan.reply.clone(),
                    titles: Vec::new(),
                });
                let result = json!({"scenario":scenario["id"],"user":user,"action":plan.action,"intent":plan.intent,"reply":plan.reply,"seeds":plan.seeds,"seconds":started.elapsed().as_secs_f32()});
                println!("{result}");
                report.push(result);
            } else {
                let response = search::run(
                    provider.as_ref(),
                    &source,
                    user,
                    &state,
                    None,
                    &cancel,
                    &mut |_| {},
                )?;
                let id = scenario["id"].as_str().unwrap_or_default();
                if let Some(expected) = scenario["count"].as_u64() {
                    if response.selections.len() != expected as usize {
                        failures.push(format!(
                            "{id}: attesi {expected} film, ottenuti {}",
                            response.selections.len()
                        ));
                    }
                }
                for s in &response.selections {
                    let movie = &s.evidence.details.movie;
                    if id == "halloween-real" && !seen.insert(movie.id) {
                        failures.push(format!("{id}: film ripetuto {}", movie.id));
                    }
                    if id == "halloween-real"
                        && s.evidence.details.genres.iter().any(|(g, _)| *g == 27)
                    {
                        failures.push(format!("{id}: genere escluso Horror per {}", movie.id));
                    }
                    if id == "source"
                        && (!movie.year.is_some_and(|y| (1993..=1997).contains(&y))
                            || !s.evidence.details.imdb.is_some_and(|r| r.rating > 8.0))
                    {
                        failures.push(format!(
                            "{id}: anno/fonte IMDb non rispettati per {}",
                            movie.id
                        ));
                    }
                }
                let result = json!({"scenario":scenario["id"],"user":user,"intent":response.state.active,"text":response.text,"notice":response.notice,"notes":response.notes,
                    "films":response.selections.iter().map(|s|json!({"id":s.evidence.details.movie.id,"title":s.evidence.details.movie.title,"year":s.evidence.details.movie.year,
                        "runtime":s.evidence.details.runtime,"why":s.judgment.why,"assessments":s.judgment.assessments,
                        "sources":s.evidence.documents.iter().map(|d|json!({"url":d.url,"license":d.license})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"seconds":started.elapsed().as_secs_f32()});
                println!("{result}");
                report.push(result);
                state = response.state;
            }
        }
    }
    let path = Config::dir().join("last-search-benchmark.json");
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }
    Ok(())
}
