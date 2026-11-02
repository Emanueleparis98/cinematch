//! Benchmark della comprensione live (200 richieste + 50 conversazioni).
//! Il giudizio semantico dei film è distinto e si esegue con chat_probe.
use cinematch::{ai::build_provider, infra::config::Config, search};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

fn main() {
    let cfg = Config::load();
    if let Some(llm) =
        cinematch::ai::llm_config(&cfg.ai).filter(|c| c.base_url == cinematch::ai::OPENROUTER_BASE)
    {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("HTTP client");
        if let Ok(response) = client
            .get("https://openrouter.ai/api/v1/credits")
            .bearer_auth(&llm.api_key)
            .send()
        {
            if response.status().is_success() {
                if let Ok(v) = response.json::<Value>() {
                    if v["data"]["total_credits"]
                        .as_f64()
                        .zip(v["data"]["total_usage"].as_f64())
                        .is_some_and(|(a, b)| a <= b)
                    {
                        eprintln!("Benchmark non avviato: saldo del provider esaurito.");
                        std::process::exit(2);
                    }
                }
            }
        }
    }
    let provider = build_provider(&cfg.ai);
    let corpus: Value =
        serde_json::from_str(include_str!("../tests/fixtures/search-corpus.json")).expect("corpus");
    let scenarios: Vec<Value> = corpus["requests"]
        .as_array()
        .unwrap()
        .iter()
        .chain(corpus["conversations"].as_array().unwrap())
        .cloned()
        .collect();
    let results = Mutex::new(Vec::new());
    let stop = AtomicBool::new(false);
    let max = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--max=")
                .and_then(|s| s.parse::<usize>().ok())
        })
        .unwrap_or(scenarios.len());
    let workers = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--workers=")
                .and_then(|n| n.parse::<usize>().ok())
        })
        .unwrap_or(1)
        .clamp(1, 4);
    for batch in scenarios
        .iter()
        .take(max)
        .collect::<Vec<_>>()
        .chunks(workers)
    {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        std::thread::scope(|scope| {
            for scenario in batch {
                let provider = &provider;
                let results = &results;
                let stop = &stop;
                scope.spawn(move || {
                    let mut state=search::ConversationState::default();
                    let mut turns=Vec::new();
                    for (index,user) in scenario["turns"].as_array().unwrap().iter().enumerate() {
                        match search::understand(provider.as_ref(),user.as_str().unwrap(),&state,None,&AtomicBool::new(false)) {
                            Ok(plan)=> {
                                let action=serde_json::to_value(plan.action).unwrap();
                                let expected_action=&scenario["expected_actions"][index];
                                let expected_count=&scenario["expected_counts"][index];
                                let actual_count=serde_json::to_value(plan.intent.count).unwrap();
                                let action_ok=action==*expected_action;
                                let count_ok=expected_action=="answer" || actual_count==*expected_count;
                                state=search::transition(&state,&plan);
                                state.turns.push(search::Turn {user:user.as_str().unwrap().into(),response:plan.reply.clone(),titles:Vec::new()});
                                turns.push(json!({"index":index,"user":user,"action":action,"intent":plan.intent,"expected_action":expected_action,"expected_count":expected_count,"action_ok":action_ok,"count_ok":count_ok}));
                            }
                            Err(error)=> {
                                if error.contains("402") { stop.store(true,Ordering::Relaxed); }
                                turns.push(json!({"index":index,"user":user,"error":error,"action_ok":false,"count_ok":false}));
                                if stop.load(Ordering::Relaxed) { break; }
                            },
                        }
                    }
                    let passed=turns.iter().all(|t|t["action_ok"]==true && t["count_ok"]==true);
                    let result=json!({"id":scenario["id"],"split":scenario["split"],"passed":passed,"turns":turns});
                    println!("{} {}",scenario["id"],if passed {"PASS"}else{"FAIL"});
                    results.lock().unwrap().push(result);
                });
            }
        });
    }
    let mut results = results.into_inner().unwrap();
    results.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let passed = results.iter().filter(|r| r["passed"] == true).count();
    let report = json!({"scenarios":results.len(),"passed":passed,"results":results});
    let path = Config::dir().join("new-search-understanding-benchmark.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).expect("report");
    println!(
        "{passed}/{} scenari superano azione e quantità; pertinenza dei film non misurata qui.",
        report["scenarios"]
    );
    if passed != report["scenarios"].as_u64().unwrap() as usize {
        std::process::exit(1);
    }
}
