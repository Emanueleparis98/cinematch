use std::{
    io::Write,
    process::{Command, Stdio},
};
fn generate(raw: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cinema-currents"))
        .arg("generate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(raw.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn executable_is_usable_by_another_application() {
    let output = generate(include_str!("../request.example.json"));
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    let p = value["prompt"].as_str().unwrap();
    assert!(p.contains("5 film complessivi") && p.contains("Ladri di biciclette"));
    assert!(output.stderr.is_empty());
}
#[test]
fn errors_do_not_masquerade_as_valid_responses() {
    for raw in [
        "not json",
        r#"{"movements":["unknown"]}"#,
        r#"{"movements":["neorealismo"],"count":0}"#,
        r#"{"movements":["neorealismo"],"api_key":"unused"}"#,
    ] {
        let output = generate(raw);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
#[test]
fn catalog_command_returns_the_revised_reference() {
    let output = Command::new(env!("CARGO_BIN_EXE_cinema-currents"))
        .arg("catalog")
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["entries"].as_array().unwrap().len(), 74);
}
