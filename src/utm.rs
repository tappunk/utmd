use crate::config::EffectiveConfig;
use crate::models::VmInfo;
use crate::output::Reporter;
use color_eyre::{Result, eyre::bail};
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT_CLONE: u64 = 120;
const TIMEOUT_MUTATION: u64 = 30;
const TIMEOUT_QUERY: u64 = 10;

fn stderr_msg(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.trim().lines().next().unwrap_or("unknown error").to_string()
}

fn run_with_timeout(mut cmd: Command, label: &'static str, timeout_secs: u64) -> Result<Output> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => bail!("spawn failed: {}", e),
    };

    let stdout_handle = std::thread::spawn({
        let mut pipe = child.stdout.take().expect("stdout should be piped");
        move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        }
    });

    let stderr_handle = std::thread::spawn({
        let mut pipe = child.stderr.take().expect("stderr should be piped");
        move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        }
    });

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_handle.join().expect("stdout thread panicked");
                let stderr = stderr_handle.join().expect("stderr thread panicked");
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();
                    return Err(crate::errors::TimedOut {
                        label,
                        timeout_secs,
                    }
                    .into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => bail!("wait error: {}", e),
        }
    }
}

pub fn ensure_utmctl(cfg: &EffectiveConfig, reporter: &Reporter) -> Result<()> {
    if which::which(&cfg.utmctl_path).is_ok() || which::which("utmctl").is_ok() {
        return Ok(());
    }

    reporter.error("utmctl not found in configured path or path");
    reporter.info("you can set UTMD_UTMCTL_PATH or install a symlink to utmctl");
    bail!("utmctl dependency missing")
}

pub fn list_vms(cfg: &EffectiveConfig) -> Result<Vec<VmInfo>> {
    let output = run_with_timeout(
        {
            let mut cmd = utmctl(cfg);
            cmd.arg("list");
            cmd
        },
        "utmctl list",
        TIMEOUT_QUERY,
    )?;
    if !output.status.success() {
        bail!("failed to run utmctl list: {}", stderr_msg(&output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut vms = Vec::new();

    for line in stdout.lines() {
        if let Some((state, name)) = parse_list_line(line) {
            vms.push(VmInfo {
                name,
                state,
                os: None,
                created_at: None,
            });
        }
    }

    Ok(vms)
}

pub fn clone_vm(cfg: &EffectiveConfig, template: &str, name: &str) -> Result<()> {
    eprintln!("info: running utmctl clone...");
    let output = run_with_timeout(
        {
            let mut cmd = utmctl(cfg);
            cmd.args(["clone", template, "--name", name]);
            cmd
        },
        "utmctl clone",
        TIMEOUT_CLONE,
    )?;
    if !output.status.success() {
        bail!("failed to clone vm: {}", stderr_msg(&output));
    }

    Ok(())
}

pub fn start_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    eprintln!("info: running utmctl start...");
    let output = run_with_timeout(
        {
            let mut cmd = utmctl(cfg);
            cmd.args(["start", name]);
            cmd
        },
        "utmctl start",
        TIMEOUT_MUTATION,
    )?;
    if !output.status.success() {
        bail!("failed to start vm '{}': {}", name, stderr_msg(&output));
    }

    Ok(())
}

pub fn stop_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    eprintln!("info: running utmctl stop...");
    let output = run_with_timeout(
        {
            let mut cmd = utmctl(cfg);
            cmd.args(["stop", name]);
            cmd
        },
        "utmctl stop",
        TIMEOUT_MUTATION,
    )?;
    if !output.status.success() {
        bail!("failed to stop vm '{}': {}", name, stderr_msg(&output));
    }

    Ok(())
}

pub fn delete_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    eprintln!("info: running utmctl delete...");
    let output = run_with_timeout(
        {
            let mut cmd = utmctl(cfg);
            cmd.args(["delete", name]);
            cmd
        },
        "utmctl delete",
        TIMEOUT_MUTATION,
    )?;
    if !output.status.success() {
        bail!("failed to delete vm '{}': {}", name, stderr_msg(&output));
    }

    Ok(())
}

pub fn open_vm(name: &str) -> Result<()> {
    let escaped_name = escape_applescript_string(name);
    let script = format!(
        "tell application \"UTM\"\nactivate\nset vmref to virtual machine named \"{}\"\nset vm_status to status of vmref\nif vm_status is stopped or vm_status is paused then\nstart vmref\nend if\nend tell",
        escaped_name
    );
    let output = run_with_timeout(
        {
            let mut cmd = Command::new("osascript");
            cmd.args(["-e", &script]);
            cmd
        },
        "osascript",
        TIMEOUT_QUERY,
    )?;
    if !output.status.success() {
        bail!("failed to open vm '{}' in UTM: {}", name, stderr_msg(&output));
    }

    Ok(())
}

fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn utmctl(cfg: &EffectiveConfig) -> Command {
    if cfg.utmctl_path.trim().is_empty() {
        return Command::new("utmctl");
    }

    Command::new(&cfg.utmctl_path)
}

fn is_header_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("name") && lower.contains("status")
}

fn parse_list_line(line: &str) -> Option<(Option<String>, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_header_line(trimmed) {
        return None;
    }

    let columns = parse_columns(trimmed);
    if columns.is_empty() {
        return None;
    }

    if columns.len() == 1 {
        return None;
    }

    if columns.len() == 2 {
        return Some((parse_state(columns[0]), columns[1].to_string()));
    }

    let state = parse_state(columns[1]);
    let name = columns[2].to_string();
    Some((state, name))
}

fn parse_columns(line: &str) -> Vec<&str> {
    let mut cols = Vec::new();
    let mut idx = 0;
    let bytes = line.as_bytes();

    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() {
        return cols;
    }

    for _ in 0..2 {
        if idx >= bytes.len() {
            break;
        }
        let start = idx;
        while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if start < idx {
            cols.push(&line[start..idx]);
        }
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
    }

    if idx < bytes.len() {
        cols.push(line[idx..].trim());
    }

    cols
}

fn parse_state(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    if lower == "running" || lower == "stopped" || lower == "paused" {
        return Some(lower);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{escape_applescript_string, parse_list_line};

    #[test]
    fn parse_with_multi_word_name() {
        let line = "A1B2-C3D4 running Ubuntu Sandbox Development";
        let parsed = parse_list_line(line).expect("line should parse");
        assert_eq!(parsed.0.as_deref(), Some("running"));
        assert_eq!(parsed.1, "Ubuntu Sandbox Development");
    }

    #[test]
    fn parse_with_extra_whitespace() {
        let line = "A1B2-C3D4    stopped    my vm";
        let parsed = parse_list_line(line).expect("line should parse");
        assert_eq!(parsed.0.as_deref(), Some("stopped"));
        assert_eq!(parsed.1, "my vm");
    }

    #[test]
    fn ignore_header_line() {
        let line = "UUID STATUS NAME";
        assert!(parse_list_line(line).is_none());
    }

    #[test]
    fn ignore_single_column_line() {
        let line = "unauthorized";
        assert!(parse_list_line(line).is_none());
    }

    #[test]
    fn escape_quotes_for_applescript() {
        let escaped = escape_applescript_string("foo\"bar");
        assert_eq!(escaped, "foo\\\"bar");
    }
}
