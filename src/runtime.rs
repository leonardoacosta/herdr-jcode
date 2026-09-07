use serde_json::{Value, json};
use std::{
    env,
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub const PLUGIN_ID: &str = "leonardoacosta.herdr-jcode";
const MAX_OUTPUT: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

fn command(bin: &str, args: &[&str]) -> Result<Vec<u8>, ()> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Herdr does not need Jcode prompts, tool data, or provider credentials.
    cmd.env_clear();
    for (key, value) in env::vars_os() {
        let name = key.to_string_lossy();
        if matches!(name.as_ref(), "PATH" | "HOME" | "LANG" | "TERM")
            || name.starts_with("XDG_")
            || name.starts_with("HERDR_")
        {
            cmd.env(key, value);
        }
    }
    let mut child = cmd.spawn().map_err(|_| ())?;
    let stdout = child.stdout.take().ok_or(())?;
    let (send, receive) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout.take(MAX_OUTPUT as u64 + 1).read_to_end(&mut output);
        let _ = send.send(result.map(|_| output));
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(());
            }
        }
    };
    let output = receive
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| ())?
        .map_err(|_| ())?;
    if !status.success() || output.len() > MAX_OUTPUT {
        return Err(());
    }
    Ok(output)
}

fn herdr_bin() -> Option<String> {
    env::var("HERDR_BIN_PATH").ok().filter(|s| !s.is_empty())
}

fn request(bin: &str, args: &[&str]) -> Result<Value, ()> {
    let value: Value = serde_json::from_slice(&command(bin, args)?).map_err(|_| ())?;
    if value.get("error").is_some() {
        return Err(());
    }
    Ok(value)
}

fn enabled(value: &Value) -> Option<bool> {
    Some(
        value
            .get("result")?
            .get("plugins")?
            .as_array()?
            .iter()
            .any(|plugin| plugin["plugin_id"] == PLUGIN_ID && plugin["enabled"] == true),
    )
}

fn native_detector(value: &Value) -> bool {
    value["agent"] == "jcode"
        && value["manifest_version"].is_string()
        && value["manifest_source"].is_string()
        && value["fallback_reason"] != "unknown_agent"
}

fn detector(bin: &str) -> Result<Value, ()> {
    request(
        bin,
        &[
            "agent",
            "explain",
            "--file",
            "/dev/null",
            "--agent",
            "jcode",
            "--json",
        ],
    )
}

fn session_fields(value: &Value) -> Option<(&str, &str, &str)> {
    let session = value.get("result")?.get("pane")?.get("agent_session")?;
    if session["kind"] != "id" {
        return None;
    }
    Some((
        session["value"].as_str()?,
        session["source"].as_str()?,
        session["agent"].as_str()?,
    ))
}

fn report_with(bin: &str, pane: &str, session: &str, source: &str) -> Result<Value, ()> {
    let plugins = request(bin, &["plugin", "list", "--plugin", PLUGIN_ID, "--json"])?;
    if !enabled(&plugins).ok_or(())? {
        return Ok(json!({"status":"skipped","reason":"disabled_or_unlinked"}));
    }
    let current = request(bin, &["pane", "get", pane])?;
    if !current["result"]["pane"].is_object() {
        return Err(());
    }
    if !current["result"]["pane"]["agent_session"].is_null()
        && session_fields(&current) != Some((session, "herdr:jcode", "jcode"))
    {
        return Ok(json!({"status":"identity_conflict"}));
    }
    if !native_detector(&detector(bin)?) {
        return Ok(json!({"status":"unsupported_host"}));
    }
    request(
        bin,
        &[
            "pane",
            "report-agent-session",
            pane,
            "--source",
            "herdr:jcode",
            "--agent",
            "jcode",
            "--agent-session-id",
            session,
            "--session-start-source",
            source,
        ],
    )?;
    let current = request(bin, &["pane", "get", pane])?;
    Ok(
        json!({"status":if session_fields(&current) == Some((session, "herdr:jcode", "jcode")) {"reported"} else {"not_confirmed"}}),
    )
}

pub fn report() -> Value {
    if env::var("HERDR_ENV").as_deref() != Ok("1") {
        return json!({"status":"skipped","reason":"outside_herdr"});
    }
    if env::var("JCODE_HOOK_EVENT").as_deref() != Ok("session_start") {
        return json!({"status":"skipped","reason":"unsupported_event"});
    }
    let source = match env::var("JCODE_HOOK_SOURCE").as_deref() {
        Ok("create" | "attach") => "startup",
        Ok("resume") => "resume",
        _ => return json!({"status":"skipped","reason":"unsupported_source"}),
    };
    let fields = [
        "HERDR_PANE_ID",
        "JCODE_HOOK_SESSION_ID",
        "HERDR_SOCKET_PATH",
    ]
    .map(|key| {
        env::var(key)
            .ok()
            .filter(|s| !s.is_empty() && s.len() <= 4096 && !s.contains('\0'))
    });
    let [Some(pane), Some(session), Some(_socket)] = fields else {
        return json!({"status":"skipped","reason":"missing_env"});
    };
    let Some(bin) = herdr_bin() else {
        return json!({"status":"unavailable"});
    };
    report_with(&bin, &pane, &session, source).unwrap_or_else(|_| json!({"status":"unavailable"}))
}

pub fn doctor() -> Value {
    let Some(bin) = herdr_bin() else {
        return json!({"status":"unavailable","reason":"missing_HERDR_BIN_PATH"});
    };
    let Ok(version) = command(&bin, &["--version"]) else {
        return json!({"status":"unavailable"});
    };
    let Ok(detection) = detector(&bin) else {
        return json!({"status":"unavailable"});
    };
    let supported = native_detector(&detection);
    let registered = request(&bin, &["plugin", "list", "--plugin", PLUGIN_ID, "--json"])
        .ok()
        .and_then(|value| enabled(&value));
    let overridden = env::var_os("JCODE_HOOK_SESSION_START").is_some();
    json!({"status":if !supported {"unsupported_host"} else if registered == Some(true) && !overridden {"ready"} else {"unavailable"},
        "herdr_version":String::from_utf8_lossy(&version).trim(), "native_detector":supported,
        "plugin_enabled":registered, "hook_env_override":overridden,
        "native_restore_verified":false, "ordering":"best_effort_first_identity; replacements require explicit resolution"})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_pane_session_uses_kind_and_value() {
        let response = json!({"result":{"pane":{"agent_session":{"kind":"id","value":"session-1","source":"herdr:jcode","agent":"jcode"}}}});
        assert_eq!(
            session_fields(&response),
            Some(("session-1", "herdr:jcode", "jcode"))
        );
    }
    #[test]
    fn unknown_agent_json_is_not_native_support() {
        assert!(!native_detector(
            &json!({"agent":"jcode","fallback_reason":"unknown_agent","manifest_version":null,"manifest_source":null})
        ));
        assert!(native_detector(
            &json!({"agent":"jcode","manifest_version":"1","manifest_source":"bundled"})
        ));
    }
    #[test]
    fn registration_requires_exact_id_and_enabled() {
        assert_eq!(
            enabled(&json!({"result":{"plugins":[{"plugin_id":PLUGIN_ID,"enabled":true}]}})),
            Some(true)
        );
        assert_eq!(
            enabled(&json!({"result":{"plugins":[{"plugin_id":PLUGIN_ID,"enabled":false}]}})),
            Some(false)
        );
        assert_eq!(enabled(&json!({"result":{"plugins":[]}})), Some(false));
        assert_eq!(enabled(&json!({"error":{}})), None);
    }
    #[test]
    fn timeout_also_bounds_inherited_output_pipe() {
        let start = Instant::now();
        assert!(command("/bin/sh", &["-c", "sleep 3 & exit 0"]).is_err());
        assert!(start.elapsed() < Duration::from_millis(2500));
    }
    #[test]
    fn output_is_bounded() {
        assert!(command("/usr/bin/head", &["-c", "70000", "/dev/zero"]).is_err());
    }
}
