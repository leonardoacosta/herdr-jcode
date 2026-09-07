#!/usr/bin/env python3
"""Real installed Herdr/Jcode acceptance. No user profiles, model requests, or mock server."""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
import traceback

REPO = Path(__file__).resolve().parents[1]
PLUGIN = REPO / "target/release/herdr-jcode"
ID = "leonardoacosta.herdr-jcode"


def wait_for(check, seconds=15):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if check():
            return
        time.sleep(0.1)
    raise AssertionError("acceptance condition timed out")


def inside():
    root = Path(os.environ["E2E_ROOT"])
    assert os.environ.get("HERDR_ENV") == "1"
    assert Path(os.environ["HERDR_SOCKET_PATH"]).parent == root
    pane = os.environ["HERDR_PANE_ID"]
    secondary = os.environ.get("E2E_SECONDARY") == "1"
    records = []
    def call(*args):
        result = subprocess.run([os.environ["HERDR_BIN_PATH"], *args], capture_output=True, text=True, timeout=8)
        records.append({"args": args, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
        assert result.returncode == 0, records[-1]
        return json.loads(result.stdout)
    child = None
    try:
        if not secondary:
            wait_for(lambda: subprocess.run([os.environ["HERDR_BIN_PATH"], "pane", "get", pane], capture_output=True).returncode == 0)
            linked = call("plugin", "link", str(REPO))
            listed = call("plugin", "list", "--plugin", ID, "--json")
            assert listed["result"]["plugins"][0]["enabled"] is True
            actions = call("plugin", "action", "list", "--plugin", ID)
            call("plugin", "action", "invoke", "setup", "--plugin", ID)
            config = root / "home/.jcode/config.toml"
            wait_for(lambda: "herdr-jcode" in config.read_text())
            installed = config.read_text()
            call("plugin", "action", "invoke", "setup", "--plugin", ID)
            time.sleep(0.5)
            assert config.read_text() == installed
            call("plugin", "action", "invoke", "doctor", "--plugin", ID)
            diagnostic = subprocess.run([str(PLUGIN), "doctor"], capture_output=True, text=True, timeout=10)
            assert diagnostic.returncode == 2
            assert json.loads(diagnostic.stdout)["status"] == "unsupported_host"
            records.append({"doctor": json.loads(diagnostic.stdout)})
            call("pane", "split", pane, "--direction", "right", "--env", "E2E_SECONDARY=1")
        launch_env = dict(os.environ, HERDR_BIN_PATH=str(root / "trace-herdr"), OPENAI_API_KEY="local-test-no-model-request", OPENAI_BASE_URL="http://127.0.0.1:9")
        child = subprocess.Popen([os.environ["E2E_JCODE"], "--provider", "openai-api", "--model", "gpt-4.1", "--socket", str(root / "jcode.sock"), "--no-update", "--no-selfdev", "--cwd", str(root / "home")], env=launch_env)
        def witnessed():
            path = root / "witness.jsonl"
            return path.exists() and any(json.loads(line).get("pane") == pane for line in path.read_text().splitlines())
        wait_for(witnessed, 20)
        if secondary:
            (root / "secondary-ready").write_text(pane)
            wait_for(lambda: (root / "finish-secondary").exists(), 25)
        else:
            wait_for(lambda: (root / "secondary-ready").exists(), 20)
            time.sleep(1)
            evidence = [json.loads(line) for line in (root / "witness.jsonl").read_text().splitlines()]
            assert len({x["pane"] for x in evidence}) == 2, evidence
            assert len({x["session"] for x in evidence}) == 2, evidence
            traffic = [json.loads(line) for line in (root / "traffic.jsonl").read_text().splitlines()]
            assert sum(x["args"][:2] == ["pane", "get"] for x in traffic) >= 2, traffic
            assert not any("report-agent-session" in x["args"] for x in traffic), traffic
            records.append({"real_hook_identity": evidence, "forwarded_real_cli_calls": len(traffic)})
            call("plugin", "disable", ID)
            report_env = dict(os.environ, JCODE_HOOK_EVENT="session_start", JCODE_HOOK_SOURCE="create", JCODE_HOOK_SESSION_ID="disabled-probe")
            report = subprocess.run([str(PLUGIN), "report"], env=report_env, capture_output=True, text=True, timeout=10)
            assert json.loads(report.stdout) == {"status": "skipped", "reason": "disabled_or_unlinked"}
            records.append({"disabled_report": json.loads(report.stdout)})
            call("plugin", "enable", ID)
            call("plugin", "action", "invoke", "remove", "--plugin", ID)
            wait_for(lambda: "herdr-jcode" not in config.read_text())
            assert config.read_text() == (root / "original-config").read_text()
            call("plugin", "unlink", ID)
            assert call("plugin", "list", "--plugin", ID, "--json")["result"]["plugins"] == []
            (root / "finish-secondary").touch()
        result = {"status": "passed", "pane": pane, "records": records}
    except Exception:
        result = {"status": "failed", "pane": pane, "records": records, "error": traceback.format_exc()}
        (root / "finish-secondary").touch()
    finally:
        if child and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
    (root / ("secondary.json" if secondary else "result.json")).write_text(json.dumps(result, indent=2))


def main():
    if "--inside" in sys.argv:
        inside()
        return
    herdr = str(Path(sys.argv[1]).resolve())
    jcode = str(Path(sys.argv[2]).resolve())
    scratch = Path(os.environ["JCODE_SCRATCH_DIR"]).resolve()
    root = Path(tempfile.mkdtemp(prefix="herdr-plugin-live-", dir=scratch))
    for name in ["home/.jcode", "config/herdr", "data", "state", "cache", "run"]:
        (root / name).mkdir(parents=True)
    shell = root / "probe-shell"
    shell.write_text(f'#!/bin/sh\nexec /usr/bin/python3 "{Path(__file__).resolve()}" --inside\n')
    shell.chmod(0o700)
    witness = root / "witness"
    witness.write_text('#!/usr/bin/python3\nimport os,json\nwith open(' + repr(str(root / "witness.jsonl")) + ',"a") as f: f.write(json.dumps({"pane":os.getenv("HERDR_PANE_ID"),"session":os.getenv("JCODE_HOOK_SESSION_ID"),"event":os.getenv("JCODE_HOOK_EVENT")})+"\\n")\n')
    witness.chmod(0o700)
    config = '[hooks]\nsession_start = [' + json.dumps(str(witness)) + '] # preserve user observer\n'
    (root / "home/.jcode/config.toml").write_text(config)
    (root / "original-config").write_text(config)
    trace = root / "trace-herdr"
    trace.write_text('#!/usr/bin/python3\nimport os,sys,json,subprocess\nr=subprocess.run([' + repr(herdr) + ']+sys.argv[1:],capture_output=True)\nwith open(' + repr(str(root / "traffic.jsonl")) + ',"a") as f: f.write(json.dumps({"args":sys.argv[1:],"exit":r.returncode})+"\\n")\nsys.stdout.buffer.write(r.stdout)\nsys.stderr.buffer.write(r.stderr)\nsys.exit(r.returncode)\n')
    trace.chmod(0o700)
    (root / "config/herdr/config.toml").write_text(f'onboarding=false\ndefault_shell={json.dumps(str(shell))}\nshell_mode="non_login"\n[update]\nversion_check=false\nmanifest_check=false\n')
    env = {"PATH": "/usr/bin:/bin", "HOME": str(root / "home"), "JCODE_HOME": str(root / "home/.jcode"), "SHELL": str(shell), "TERM": "xterm-256color", "LANG": "C.UTF-8", "HERDR_SOCKET_PATH": str(root / "api.sock"), "E2E_ROOT": str(root), "E2E_JCODE": jcode}
    for key, value in [("CONFIG", "config"), ("DATA", "data"), ("STATE", "state"), ("CACHE", "cache"), ("RUNTIME", "run")]:
        env[f"XDG_{key}_HOME" if key != "RUNTIME" else "XDG_RUNTIME_DIR"] = str(root / value)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
    process = subprocess.Popen([herdr, "--no-session"], env=env, cwd=root / "home", stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
    os.close(slave)
    try:
        end = time.monotonic() + 80
        with (root / "terminal.log").open("wb") as log:
            while time.monotonic() < end and not (root / "result.json").exists():
                if select.select([master], [], [], 0.2)[0]:
                    try:
                        log.write(os.read(master, 65536))
                    except OSError:
                        break
    finally:
        subprocess.run([jcode, "--socket", str(root / "jcode.sock"), "server", "stop"], env=env, capture_output=True, timeout=10)
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        os.close(master)
    print(root)
    if not (root / "result.json").exists():
        raise SystemExit("live acceptance timed out; inspect terminal.log")
    result = json.loads((root / "result.json").read_text())
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result["status"] == "passed" else 1)


if __name__ == "__main__":
    main()
