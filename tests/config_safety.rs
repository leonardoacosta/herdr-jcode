use herdr_jcode::config::{edit_config, hook_command, update};
use std::path::Path;

#[test]
fn scalar_hook_is_composed_not_ignored() {
    let s = "[hooks]\nsession_start = 'user' # keep\n";
    let result = edit_config(s, "plugin", true).unwrap();
    let doc = result.parse::<toml_edit::DocumentMut>().unwrap();
    let hooks = doc["hooks"]["session_start"].as_array().unwrap();
    assert_eq!(hooks.get(0).unwrap().as_str(), Some("user"));
    assert_eq!(hooks.get(1).unwrap().as_str(), Some("plugin"));
    assert!(result.contains("# keep"));
}
#[test]
fn reinstall_preserves_owned_entry_position_and_comments() {
    let s = "[hooks]\nsession_start = [\n 'plugin', # owned\n 'user', # user\n]\n";
    assert_eq!(edit_config(s, "plugin", true).unwrap(), s);
}
#[test]
fn mixed_array_is_rejected_without_mutation() {
    assert!(edit_config("[hooks]\nsession_start = ['user', 42]\n", "plugin", true).is_err());
}
#[test]
fn setup_creates_missing_config_directory() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let path = dir.path().join("new/config.toml");
    assert!(update(&path, "plugin", true).unwrap());
}
#[test]
fn hook_argv_escapes_backslashes() {
    assert_eq!(
        hook_command(Path::new("/dir/back\\slash\"quote")).unwrap(),
        "\"/dir/back\\\\slash\\\"quote\" report"
    );
}

#[test]
fn malformed_config_leaves_no_lock_or_temporary_file() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[hooks\n").unwrap();
    assert!(update(&path, "plugin", true).is_err());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "[hooks\n");
}

#[test]
fn inline_hook_table_keeps_other_values() {
    let output = edit_config(
        "hooks = { session_start = 'user', turn_end = 'notify' }\n",
        "plugin",
        true,
    )
    .unwrap();
    let doc = output.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(doc["hooks"]["turn_end"].as_str(), Some("notify"));
    assert_eq!(doc["hooks"]["session_start"].as_array().unwrap().len(), 2);
}
