//! Synthetic receiver tests. Real public-host coverage is in live_herdr.py.
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn fixture(mode: &str, event: &str, source: &str, sequence: Option<&str>) -> (Value, Vec<Value>) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let script = dir.path().join("herdr fixture");
    fs::write(dir.path().join("mode"), mode).unwrap();
    fs::write(&script, r#"#!/usr/bin/python3
import json,sys,pathlib,os
root=pathlib.Path(__file__).parent
mode=(root/'mode').read_text()
a=sys.argv[1:]
assert 'JCODE_HOOK_PAYLOAD' not in os.environ
assert 'ANTHROPIC_API_KEY' not in os.environ
with (root/'calls').open('a') as f: f.write(json.dumps(a)+'\n')
if a[:2]==['plugin','list']:
 print(json.dumps({'result':{'plugins':[{'plugin_id':'leonardoacosta.herdr-jcode','enabled':mode!='disabled'}]}}))
elif a[:2]==['pane','get']:
 assert a==['pane','get','w1:p1'], a
 pane={'pane_id':'w1:p1','agent_status':'unknown'}
 if mode=='foreign': pane.update(agent='codex',agent_status='working')
 if mode=='working': pane.update(agent='jcode',agent_status='working')
 if (root/'state').exists() and mode!='discarded': pane.update(json.loads((root/'state').read_text()))
 if mode=='conflict': pane['agent_session']={'kind':'id','value':'other-session','source':'herdr:jcode','agent':'jcode'}
 if (root/'identity').exists(): pane['agent_session']=json.loads((root/'identity').read_text())
 print(json.dumps({'result':{'pane':pane}}))
elif a[:2]==['agent','explain']:
 print(json.dumps({'agent':'jcode','manifest_version':'1' if mode=='native' else None,'manifest_source':'bundled' if mode=='native' else None,'fallback_reason':None if mode=='native' else 'unknown_agent'}))
elif a[:2]==['pane','report-agent']:
 assert a[a.index('--source')+1]=='custom:leonardoacosta.herdr-jcode',a
 assert a[a.index('--agent')+1]=='jcode',a
 assert a[a.index('--agent-session-id')+1]=="opaque id ' \\",a
 (root/'state').write_text(json.dumps({'agent':'jcode','agent_status':a[a.index('--state')+1]}))
 print(json.dumps({'result':{'type':'ok'}}))
elif a[:2]==['pane','release-agent']:
 assert a[a.index('--source')+1]=='custom:leonardoacosta.herdr-jcode',a
 (root/'state').write_text(json.dumps({'agent':None,'agent_status':'unknown'}))
 print(json.dumps({'result':{'type':'ok'}}))
elif a[:2]==['pane','report-agent-session']:
 (root/'identity').write_text(json.dumps({'kind':'id','value':a[a.index('--agent-session-id')+1],'source':'herdr:jcode','agent':'jcode'}))
 print(json.dumps({'result':{'type':'ok'}}))
else: raise Exception('unexpected command '+repr(a))
"#).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_herdr-jcode"));
    cmd.arg("report")
        .env_clear()
        .env("HOME", dir.path())
        .env("PATH", "/usr/bin:/bin")
        .env("HERDR_BIN_PATH", &script)
        .env("HERDR_ENV", "1")
        .env("HERDR_SOCKET_PATH", dir.path().join("test.sock"))
        .env("HERDR_PANE_ID", "w1:p1")
        .env("JCODE_HOOK_EVENT", event)
        .env("JCODE_HOOK_SOURCE", source)
        .env("JCODE_HOOK_SESSION_ID", "opaque id ' \\")
        .env("JCODE_HOOK_PAYLOAD", "secret-must-not-appear")
        .env("ANTHROPIC_API_KEY", "test-only-secret");
    if let Some(seq) = sequence {
        cmd.env("JCODE_HOOK_SEQUENCE", seq);
    }
    let result = cmd.output().unwrap();
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
fn stock_host_receives_working_without_native_detection() {
    let (result, calls) = fixture("stock", "turn_start", "user", Some("42"));
    assert_eq!(result["status"], "reported");
    assert_eq!(result["state"], "working");
    assert_eq!(result["ordering"], "producer_sequence");
    assert!(calls.iter().any(|v| v[1] == "report-agent"));
    assert!(
        !calls
            .iter()
            .any(|v| v[0] == "agent" || v[1] == "report-agent-session")
    );
    let report = calls
        .iter()
        .find(|v| v[1] == "report-agent")
        .unwrap()
        .as_array()
        .unwrap();
    let pos = report.iter().position(|v| v == "--seq").unwrap();
    assert_eq!(report[pos + 1], "42");
}

#[test]
fn all_lifecycle_events_work_on_stock_host() {
    for (event, source, state) in [
        ("session_start", "create", "idle"),
        ("turn_end", "", "idle"),
        ("session_end", "close", "unknown"),
    ] {
        let (result, calls) = fixture("stock", event, source, Some("43"));
        assert_eq!(result["state"], state, "{result}");
        assert_eq!(result["observed_state"], state, "{result}");
        assert_eq!(
            result["status"],
            if event == "session_end" {
                "release_requested"
            } else {
                "reported"
            }
        );
        assert!(calls.iter().any(|v| v[1]
            == if event == "session_end" {
                "release-agent"
            } else {
                "report-agent"
            }));
        if event == "session_start" {
            assert_eq!(result["native_identity"]["status"], "unsupported_host");
        }
    }
}
#[test]
fn attach_and_resume_do_not_turn_a_working_pane_idle() {
    for source in ["attach", "resume"] {
        let (result, _) = fixture("working", "session_start", source, Some("44"));
        assert_eq!(result["state"], "working");
    }
}
#[test]
fn native_identity_is_optional_and_separate_from_state() {
    let (result, calls) = fixture("native", "session_start", "resume", Some("45"));
    assert_eq!(result["status"], "reported");
    assert_eq!(result["native_identity"]["status"], "reported");
    let report = calls
        .iter()
        .find(|v| v[1] == "report-agent-session")
        .unwrap();
    assert!(report.as_array().unwrap().contains(&json!("resume")));
}
#[test]
fn a_discarded_state_report_is_not_confirmed() {
    assert_eq!(
        fixture("discarded", "turn_start", "user", None).0["status"],
        "not_confirmed"
    );
}
#[test]
fn missing_sequence_is_explicitly_best_effort() {
    let (result, calls) = fixture("stock", "turn_start", "user", None);
    assert_eq!(result["ordering"], "best_effort");
    assert!(
        !calls
            .iter()
            .any(|v| v.as_array().unwrap().contains(&json!("--seq")))
    );
}
#[test]
fn invalid_sequence_fails_open_without_api_calls() {
    for sequence in ["", "-1", "+2", " 3", "1.5", "18446744073709551616"] {
        let (result, calls) = fixture("stock", "turn_start", "user", Some(sequence));
        assert_eq!(result["reason"], "invalid_sequence");
        assert!(calls.is_empty());
    }
}
#[test]
fn unknown_source_or_event_does_not_contact_herdr() {
    for (event, source) in [("session_start", "unknown"), ("pre_tool", "user")] {
        let (result, calls) = fixture("stock", event, source, None);
        assert_eq!(result["status"], "skipped");
        assert!(calls.is_empty());
    }
}
#[test]
fn disabled_plugin_does_not_mutate_state() {
    let (result, calls) = fixture("disabled", "turn_start", "user", Some("42"));
    assert_eq!(result["reason"], "disabled_or_unlinked");
    assert_eq!(calls.len(), 1);
}
#[test]
fn another_agent_is_not_overwritten() {
    let (result, calls) = fixture("foreign", "turn_start", "user", Some("42"));
    assert_eq!(result["reason"], "another_agent");
    assert_eq!(calls.len(), 2);
}
