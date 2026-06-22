use crate::config::EffectiveConfig;
use crate::models::VmInfo;
use crate::output::Reporter;
use color_eyre::{Result, eyre::bail};
use std::process::Command;

pub fn ensure_utmctl(cfg: &EffectiveConfig, reporter: &Reporter) -> Result<()> {
    if which::which(&cfg.utmctl_path).is_ok() || which::which("utmctl").is_ok() {
        return Ok(());
    }

    reporter.error("utmctl not found in configured path or path");
    reporter.info("you can set UTMD_UTMCTL_PATH or install a symlink to utmctl");
    bail!("utmctl dependency missing")
}

pub fn list_vms(cfg: &EffectiveConfig) -> Result<Vec<VmInfo>> {
    let output = utmctl(cfg).arg("list").output()?;
    if !output.status.success() {
        bail!("failed to run utmctl list");
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
    let status = utmctl(cfg)
        .args(["clone", template, "--name", name])
        .status()?;
    if !status.success() {
        bail!("failed to clone vm");
    }

    Ok(())
}

pub fn start_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    let status = utmctl(cfg).args(["start", name]).status()?;
    if !status.success() {
        bail!("failed to start vm '{}'", name);
    }

    Ok(())
}

pub fn stop_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    let status = utmctl(cfg).args(["stop", name]).status()?;
    if !status.success() {
        bail!("failed to stop vm '{}'", name);
    }

    Ok(())
}

pub fn delete_vm(cfg: &EffectiveConfig, name: &str) -> Result<()> {
    let status = utmctl(cfg).args(["delete", name]).status()?;
    if !status.success() {
        bail!("failed to delete vm '{}'", name);
    }

    Ok(())
}

pub fn open_vm(name: &str) -> Result<()> {
    let script = format!(
        "tell application \"UTM\"\nactivate\nshow virtual machine named \"{}\"\nend tell",
        name
    );
    let status = Command::new("osascript").args(["-e", &script]).status()?;
    if !status.success() {
        bail!("failed to open vm '{}' in UTM", name);
    }

    Ok(())
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
        return Some((None, columns[0].to_string()));
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
    use super::parse_list_line;

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
}
