use crate::config::EffectiveConfig;
use crate::models::VmInfo;
use crate::output::Reporter;
use color_eyre::{Result, eyre::WrapErr, eyre::bail, eyre::eyre};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT_CLONE: u64 = 120;
const TIMEOUT_MUTATION: u64 = 30;
const TIMEOUT_QUERY: u64 = 10;

fn stderr_msg(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .trim()
        .lines()
        .next()
        .unwrap_or("unknown error")
        .to_string()
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
    let path = resolve_utmctl(cfg)
        .map_err(|err| {
            reporter.error(&format!("{}", err));
            reporter.info(
                "utmctl ships inside UTM.app and requires a GUI login session; it fails over SSH or before login",
            );
            eyre!("utmctl dependency missing")
        })?;

    reporter.info(&format!("using utmctl at {}", path.display()));
    Ok(())
}

pub fn resolve_utmctl(cfg: &EffectiveConfig) -> Result<PathBuf> {
    if let Some(explicit) = cfg.utmctl_path.as_deref() {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            return validated_utmctl_path(Path::new(trimmed));
        }
    }

    if let Ok(found) = which::which("utmctl") {
        return validated_utmctl_path(&found);
    }

    for fallback in fallback_utmctl_paths() {
        if fallback.exists() {
            return validated_utmctl_path(&fallback);
        }
    }

    bail!(
        "utmctl not found: no explicit utmctl_path, not on PATH, and no UTM.app install found in standard locations. Set utmctl_path or UTMD_UTMCTL_PATH to the path of a utmctl binary inside UTM.app"
    )
}

fn fallback_utmctl_paths() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("/Applications/UTM.app/Contents/MacOS/utmctl"),
        PathBuf::from("/opt/homebrew/bin/utmctl"),
        PathBuf::from("/usr/local/bin/utmctl"),
    ];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Applications/UTM.app/Contents/MacOS/utmctl"));
    }
    paths
}

fn validated_utmctl_path(path: &Path) -> Result<PathBuf> {
    let resolved = std::fs::canonicalize(path).wrap_err(format!(
        "failed to resolve utmctl path '{}'",
        path.display()
    ))?;
    if !is_utmctl_path(&resolved) {
        bail!(
            "resolved utmctl path '{}' does not end with UTM.app/Contents/MacOS/utmctl",
            resolved.display()
        );
    }
    Ok(resolved)
}

pub fn is_utmctl_path(path: &Path) -> bool {
    let mut components = path.components().rev();
    matches!(
        (
            components.next(),
            components.next(),
            components.next(),
            components.next()
        ),
        (
            Some(Component::Normal(a)),
            Some(Component::Normal(b)),
            Some(Component::Normal(c)),
            Some(Component::Normal(d))
        ) if a == "utmctl" && b == "MacOS" && c == "Contents" && d == "UTM.app"
    )
}

pub fn list_vms(cfg: &EffectiveConfig) -> Result<Vec<VmInfo>> {
    let output = run_with_timeout(
        {
            let mut cmd = utmctl_cmd(cfg)?;
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
            let mut cmd = utmctl_cmd(cfg)?;
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
            let mut cmd = utmctl_cmd(cfg)?;
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
            let mut cmd = utmctl_cmd(cfg)?;
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
            let mut cmd = utmctl_cmd(cfg)?;
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
        bail!(
            "failed to open vm '{}' in UTM: {}",
            name,
            stderr_msg(&output)
        );
    }

    Ok(())
}

fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn utmctl_cmd(cfg: &EffectiveConfig) -> Result<Command> {
    let path = resolve_utmctl(cfg)?;
    Ok(Command::new(path))
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
    use super::{escape_applescript_string, is_utmctl_path, parse_list_line};
    use std::path::Path;

    #[test]
    fn accepts_embedded_app_path() {
        let path = Path::new("/Applications/UTM.app/Contents/MacOS/utmctl");
        assert!(is_utmctl_path(path));
    }

    #[test]
    fn accepts_home_app_path() {
        let path = Path::new("/Users/tappunk/Applications/UTM.app/Contents/MacOS/utmctl");
        assert!(is_utmctl_path(path));
    }

    #[test]
    fn rejects_path_not_inside_app_bundle() {
        let path = Path::new("/opt/homebrew/bin/utmctl-copy");
        assert!(!is_utmctl_path(path));
    }

    #[test]
    fn rejects_similar_suffix_only() {
        let path = Path::new("/Applications/UTM.app/Contents/MacOS/utmctl/extra");
        assert!(!is_utmctl_path(path));
    }

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
