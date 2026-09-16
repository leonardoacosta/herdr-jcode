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
pub const STATE_SOURCE: &str = "custom:leonardoacosta.herdr-jcode";
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

fn lifecycle(
    bin: &str,
    pane: &str,
    session: &str,
    event: &str,
    source: &str,
    sequence: Option<&str>,
) -> Result<Value, ()> {
    let plugins = request(bin, &["plugin", "list", "--plugin", PLUGIN_ID, "--json"])?;
    if !enabled(&plugins).ok_or(())? {
        return Ok(json!({"status":"skipped","reason":"disabled_or_unlinked"}));
    }
    let current = request(bin, &["pane", "get", pane])?;
    let current = &current["result"]["pane"];
    if !current.is_object() {
        return Err(());
    }
    if event != "session_end"
        && current["agent"]
            .as_str()
            .is_some_and(|agent| agent != "jcode")
    {
        return Ok(json!({"status":"skipped","reason":"another_agent"}));
    }
    let state = match event {
        "turn_start" => "working",
        "session_start" if source != "create" && current["agent"] == "jcode" => {
            match current["agent_status"].as_str() {
                Some("working") => "working",
                Some("blocked") => "blocked",
                _ => "idle",
            }
        }
        _ => "idle",
    };
    let release = event == "session_end";
    let mut args = vec![
        "pane",
        if release {
            "release-agent"
        } else {
            "report-agent"
        },
        pane,
        "--source",
        STATE_SOURCE,
        "--agent",
        "jcode",
    ];
    if !release {
        args.extend(["--state", state, "--agent-session-id", session]);
    }
    if let Some(seq) = sequence {
        args.extend(["--seq", seq]);
    }
    request(bin, &args)?;
    let after = request(bin, &["pane", "get", pane])?;
    let after = &after["result"]["pane"];
    if !after.is_object() {
        return Err(());
    }
    let mut result = json!({
        "status": if release {"release_requested"} else if after["agent"] == "jcode" && after["agent_status"] == state {"reported"} else {"not_confirmed"},
        "event":event,
        "state":if release {"unknown"} else {state},
        "observed_state":after["agent_status"],
        "ordering":if sequence.is_some() {"producer_sequence"} else {"best_effort"}
    });
    if event == "session_start" {
        let native_source = if source == "resume" {
            "resume"
        } else {
            "startup"
        };
        result["native_identity"] = report_with(bin, pane, session, native_source)
            .unwrap_or_else(|_| json!({"status":"unavailable"}));
    }
    Ok(result)
}

pub fn report() -> Value {
    if env::var("HERDR_ENV").as_deref() != Ok("1") {
        return json!({"status":"skipped","reason":"outside_herdr"});
    }
    let event = env::var("JCODE_HOOK_EVENT").unwrap_or_default();
    if !matches!(
        event.as_str(),
        "session_start" | "turn_start" | "turn_end" | "session_end"
    ) {
        return json!({"status":"skipped","reason":"unsupported_event"});
    }
    let source = env::var("JCODE_HOOK_SOURCE").unwrap_or_default();
    if event == "session_start" && !matches!(source.as_str(), "create" | "attach" | "resume") {
        return json!({"status":"skipped","reason":"unsupported_source"});
    }
    let sequence = env::var_os("JCODE_HOOK_SEQUENCE");
    let sequence = match sequence.as_ref().map(|s| s.to_str()) {
        None => None,
        Some(Some(s))
            if !s.is_empty()
                && s.bytes().all(|c| c.is_ascii_digit())
                && s.parse::<u64>().is_ok() =>
        {
            Some(s)
        }
        _ => return json!({"status":"skipped","reason":"invalid_sequence"}),
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
    lifecycle(&bin, &pane, &session, &event, &source, sequence)
        .unwrap_or_else(|_| json!({"status":"unavailable"}))
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

const MODEL_SOURCE: &str = "herdr:jcode:model";

/// Derive a short model name from JCODE_HOOK_MODEL.
/// Strips version/date suffixes. Falls back to the raw model string.
fn model_short_name(model: &str) -> String {
    // Common patterns: "claude-4-opus-20250219" -> "claude-4-opus"
    // "deepseek-chat", "gpt-5.1" stay unchanged.
    let trimmed = model.trim();
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() >= 4 && parts.last().map_or(false, |s| s.len() >= 6 && s.bytes().all(|b| b.is_ascii_digit())) {
        parts[..parts.len() - 1].join("-")
    } else {
        trimmed.to_string()
    }
}

pub fn report_model() -> Value {
    if env::var("HERDR_ENV").as_deref() != Ok("1") {
        return json!({"status":"skipped","reason":"outside_herdr"});
    }
    let model = env::var("JCODE_HOOK_MODEL").unwrap_or_default();
    if model.is_empty() {
        return json!({"status":"skipped","reason":"no_model_info"});
    }
    let pane = env::var("HERDR_PANE_ID")
        .ok()
        .filter(|s| !s.is_empty() && s.len() <= 4096 && !s.contains('\0'));
    let Some(pane) = pane else {
        return json!({"status":"skipped","reason":"missing_pane_id"});
    };
    let Some(bin) = herdr_bin() else {
        return json!({"status":"unavailable"});
    };

    let plugins = request(&bin, &["plugin", "list", "--plugin", PLUGIN_ID, "--json"]);
    if !plugins.ok().and_then(|v| enabled(&v)).unwrap_or(false) {
        return json!({"status":"skipped","reason":"disabled_or_unlinked"});
    }

    let short = model_short_name(&model);
    let provider = crate::config::provider_for_model(&model);

    // Publish display-only metadata with a dedicated source.
    // report-metadata returns no stdout on success; use command() which checks exit code.
    let mut args: Vec<String> = vec![
        "pane".into(),
        "report-metadata".into(),
        pane.clone(),
        "--source".into(),
        MODEL_SOURCE.into(),
        "--token".into(),
        format!("model={}", short),
    ];
    if let Some(ref provider_id) = provider {
        args.push("--token".into());
        args.push(format!("provider={}", provider_id));
    }
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let result = command(&bin, &args_refs);
    match result {
        Ok(_) => json!({"status":"reported","model":short,"provider":provider}),
        Err(_) => json!({"status":"unavailable"}),
    }
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
