use std::process::Command;

#[test]
fn help_explains_actions() {
    let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    for action in ["setup", "remove", "doctor", "report"] {
        assert!(text.contains(action), "missing {action}: {text}");
    }
}

#[test]
fn setup_and_remove_preserve_user_config() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let config = dir.path().join("config.toml");
    let original = "# user settings\n[hooks]\nsession_start = [\"user-observer\"] # keep me\nturn_end = \"notify\"\n";
    std::fs::write(&config, original).unwrap();
    let run = |action| {
        Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
            .args([action, "--config"])
            .arg(&config)
            .output()
            .unwrap()
    };
    let setup = run("setup");
    assert!(setup.status.success(), "{:?}", setup);
    let installed = std::fs::read_to_string(&config).unwrap();
    assert!(installed.contains(" report"), "hook was not installed");
    assert!(installed.contains("# keep me"));
    assert!(run("setup").status.success());
    assert_eq!(std::fs::read_to_string(&config).unwrap(), installed);
    assert!(run("remove").status.success());
    // Removal restores original hook content, but does not revert
    // scalar-to-array conversions performed during installation.
    let removed = std::fs::read_to_string(&config).unwrap();
    let expected = "# user settings\n[hooks]\nsession_start = [\"user-observer\"] # keep me\nturn_end = [\"notify\"]\n";
    assert_eq!(removed, expected, "removal preserved original hooks");
}

#[test]
fn marketplace_manifest_has_explicit_safe_actions() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/herdr-plugin.toml"))
        .expect("marketplace manifest required");
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(doc["id"].as_str(), Some("leonardoacosta.herdr-jcode"));
    assert_eq!(doc["version"].as_str(), Some(env!("CARGO_PKG_VERSION")));
    assert_eq!(doc["platforms"].as_array().unwrap().len(), 1);
    assert_eq!(doc["platforms"][0].as_str(), Some("linux"));
    assert!(
        doc.get("startup").is_none(),
        "must not silently configure Jcode at startup"
    );
    let actions = doc["actions"].as_array_of_tables().unwrap();
    assert_eq!(actions.len(), 5);
    for id in ["model", "setup", "remove", "doctor", "help"] {
        let action = actions
            .iter()
            .find(|a| a["id"].as_str() == Some(id))
            .unwrap();
        assert_eq!(action["command"][1].as_str(), Some(id));
    }
}

#[test]
fn report_outside_herdr_is_fail_open_and_does_not_echo_payload() {
    let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
        .arg("report")
        .env_remove("HERDR_ENV")
        .env("JCODE_HOOK_PAYLOAD", "PRIVATE-PROMPT-MARKER")
        .output()
        .unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["status"], "skipped");
    assert!(!text.contains("PRIVATE-PROMPT-MARKER"));
}

#[test]
fn doctor_missing_host_is_machine_readable() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
        .arg("doctor")
        .env_clear()
        .env("HOME", dir.path())
        .env("HERDR_BIN_PATH", dir.path().join("missing-herdr"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["status"], "unavailable");
}

#[test]
fn setup_refuses_environment_override_without_writing() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let path = dir.path().join("config.toml");
    let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
        .args(["setup", "--config"])
        .arg(&path)
        .env("JCODE_HOOK_SESSION_START", "user override")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!path.exists());
}

#[test]
fn setup_refuses_any_lifecycle_override_without_writing() {
    for event in ["TURN_START", "TURN_END", "SESSION_END"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
            .args(["setup", "--config"])
            .arg(&path)
            .env(format!("JCODE_HOOK_{event}"), "override")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(2),
            "override ignored for {event}"
        );
        assert!(!path.exists());
    }
}
