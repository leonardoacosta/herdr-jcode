//! Synthetic protocol coverage supplements tests/live_herdr.py; it is not native-host acceptance.
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn fixture(mode: &str, source: &str) -> (Value, Vec<Value>) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let script = dir.path().join("herdr fixture");
    fs::write(dir.path().join("mode"), mode).unwrap();
    fs::write(&script, r#"#!/usr/bin/python3
import json,sys,pathlib
root=pathlib.Path(__file__).parent
mode=(root/'mode').read_text()
a=sys.argv[1:]
with (root/'calls').open('a') as f: f.write(json.dumps(a)+'\n')
if a[:2]==['plugin','list']:
 print(json.dumps({'result':{'plugins':[{'plugin_id':'leonardoacosta.herdr-jcode','enabled':mode!='disabled'}]}}))
elif a[:2]==['pane','get']:
 assert a==['pane','get','w1:p1'], a
 pane={'pane_id':'w1:p1'}
 if mode=='conflict': pane['agent_session']={'kind':'id','value':'other-session','source':'herdr:jcode','agent':'jcode'}
 if (root/'reported').exists() and mode!='not_confirmed': pane['agent_session']=json.loads((root/'reported').read_text())
 print(json.dumps({'result':{'pane':pane}}))
elif a[:2]==['agent','explain']:
 print(json.dumps({'agent':'jcode','manifest_version':None if mode=='unsupported' else '1','manifest_source':None if mode=='unsupported' else 'bundled','fallback_reason':'unknown_agent' if mode=='unsupported' else None}))
elif a[:2]==['pane','report-agent-session']:
 assert a[2]=='w1:p1' and '--seq' not in a, a
 assert a[a.index('--source')+1]=='herdr:jcode', a
 assert a[a.index('--agent')+1]=='jcode', a
 (root/'reported').write_text(json.dumps({'kind':'id','value':a[a.index('--agent-session-id')+1],'source':'herdr:jcode','agent':'jcode'}))
 print(json.dumps({'result':{'type':'ok'}}))
else: raise Exception('unexpected command '+repr(a))
"#).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"))
        .arg("report")
        .env_clear()
        .env("HOME", dir.path())
        .env("PATH", "/usr/bin:/bin")
        .env("HERDR_BIN_PATH", &script)
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", dir.path().join("test.sock"))
        .env("HERDR_PANE_ID", "w1:p1")
        .env("JCODE_HOOK_EVENT", "session_start")
        .env("JCODE_HOOK_SOURCE", source)
        .env(
            "JCODE_HOOK_SESSION_ID",
            "opaque id with spaces ' and quotes\"",
        )
        .env("JCODE_HOOK_PAYLOAD", "secret-must-not-appear")
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("secret-must-not-appear"));
    let calls = fs::read_to_string(dir.path().join("calls"))
        .unwrap_or_default()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    (serde_json::from_slice(&result.stdout).unwrap(), calls)
}

#[test]
fn reports_only_after_readback_matches_and_maps_sources() {
    for (source, expected) in [
        ("create", "startup"),
        ("attach", "startup"),
        ("resume", "resume"),
    ] {
        let (result, calls) = fixture("supported", source);
        assert_eq!(result["status"], "reported");
        let report = calls
            .iter()
            .find(|v| v[1] == "report-agent-session")
            .unwrap();
        assert_eq!(report.as_array().unwrap().last().unwrap(), expected);
    }
}
#[test]
fn a_successful_report_without_readback_is_not_confirmed() {
    assert_eq!(
        fixture("not_confirmed", "create").0["status"],
        "not_confirmed"
    );
}
#[test]
fn unsupported_host_does_not_receive_native_mutation() {
    let (result, calls) = fixture("unsupported", "create");
    assert_eq!(result["status"], "unsupported_host");
    assert!(!calls.iter().any(|v| v[1] == "report-agent-session"));
}
#[test]
fn different_identity_is_not_overwritten() {
    let (result, calls) = fixture("conflict", "create");
    assert_eq!(result["status"], "identity_conflict");
    assert_eq!(calls.len(), 2);
}
#[test]
fn disabled_plugin_never_reads_or_mutates_a_pane() {
    let (result, calls) = fixture("disabled", "create");
    assert_eq!(
        result,
        json!({"status":"skipped","reason":"disabled_or_unlinked"})
    );
    assert_eq!(calls.len(), 1);
}
#[test]
fn unknown_source_does_not_contact_herdr() {
    let (result, calls) = fixture("supported", "unknown");
    assert_eq!(result["status"], "skipped");
    assert!(calls.is_empty());
}
