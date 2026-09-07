use herdr_jcode::config;
use serde_json::json;
use std::{env, path::PathBuf, process::ExitCode};

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args == ["--help"] || args == ["help"] {
        println!(
            "herdr-jcode: setup | remove | doctor | report\nsetup/remove accept --config PATH. Otherwise use $JCODE_HOME/config.toml or ~/.jcode/config.toml.\nSession identity only. Native Jcode support in Herdr is required for restore."
        );
        return Ok(());
    }
    let action = &args[0];
    if args == ["report"] {
        println!("{}", herdr_jcode::runtime::report());
        return Ok(());
    }
    if args == ["doctor"] {
        let result = herdr_jcode::runtime::doctor();
        let ready = result["status"] == "ready";
        println!("{result}");
        return if ready {
            Ok(())
        } else {
            Err("integration is not ready; see diagnostic JSON".into())
        };
    }
    if action != "setup" && action != "remove" {
        return Err("unknown action; use --help".into());
    }
    let path = match args.as_slice() {
        [_] => {
            let home = env::var_os("JCODE_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".jcode")))
                .ok_or("set JCODE_HOME or HOME, or pass --config PATH")?;
            home.join("config.toml")
        }
        [_, option, path] if option == "--config" => PathBuf::from(path),
        _ => return Err("expected setup/remove [--config PATH]".into()),
    };
    if action == "setup" && env::var_os("JCODE_HOOK_SESSION_START").is_some() {
        return Err("JCODE_HOOK_SESSION_START overrides config. Unset it before setup and in Jcode's launch environment.".into());
    }
    let command = config::hook_command(&env::current_exe().map_err(|e| e.to_string())?)?;
    let changed = config::update(&path, &command, action == "setup")?;
    println!(
        "{}",
        json!({"status": if action == "setup" {"configured"} else {"removed"}, "changed":changed, "config":path, "native_restore_verified":false})
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", json!({"status":"error", "message":error}));
            ExitCode::from(2)
        }
    }
}
