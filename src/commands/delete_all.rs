use crate::cli::DeleteAllArgs;
use crate::config::EffectiveConfig;
use crate::errors::ExitCode;
use crate::models::{CommandResponse, DeleteSummary, OperationResult};
use crate::output::Reporter;
use crate::state;
use crate::utm;
use chrono::{Duration, Utc};
use color_eyre::Result;
use dialoguer::Confirm;

pub fn run(args: DeleteAllArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    let prefix = args.prefix.unwrap_or_else(|| cfg.default_prefix.clone());
    let os_filter = args.os.map(|v| v.as_str().to_string());
    let older_than = match args.older_than.as_deref().map(parse_older_than).transpose() {
        Ok(v) => v,
        Err(err) => {
            if reporter.is_json() {
                reporter.print_json(&CommandResponse::<DeleteSummary>::failure(
                    "prune",
                    format!("{}", err),
                ))?;
            } else {
                reporter.error(&format!("{}", err));
            }
            return Ok(ExitCode::InvalidUsage);
        }
    };

    let mut candidates = utm::list_vms(cfg)?;
    let mut state = state::load(&cfg.state_path)?;

    for vm in &mut candidates {
        if let Some(meta) = state.vms.get(&vm.name) {
            vm.os = meta.os.clone();
            vm.created_at = Some(meta.created_at);
        }
    }

    candidates.retain(|vm| vm.name.starts_with(&prefix));

    if let Some(os) = &os_filter {
        candidates.retain(|vm| vm.os.as_deref().is_some_and(|value| value == os));
    }

    if let Some(age) = older_than {
        let cutoff = Utc::now() - age;
        candidates.retain(|vm| vm.created_at.is_some_and(|created| created <= cutoff));
    }

    if !cfg.yes && !cfg.dry_run {
        let prompt = format!(
            "remove {} vm(s) matching prefix '{}' now?",
            candidates.len(),
            prefix
        );
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !confirmed {
            reporter.info("aborted");
            return Ok(ExitCode::Success);
        }
    }

    let mut summary = DeleteSummary {
        matched: candidates.len(),
        skipped: 0,
        deleted: 0,
        failed: 0,
        results: Vec::new(),
    };

    for vm in candidates {
        if cfg.dry_run {
            summary.results.push(OperationResult {
                ok: true,
                action: "rm".to_string(),
                target: Some(vm.name.clone()),
                message: format!("dry-run: would remove '{}'", vm.name),
                warnings: Vec::new(),
            });
            summary.skipped += 1;
            continue;
        }

        match utm::stop_vm(cfg, &vm.name).and_then(|_| utm::delete_vm(cfg, &vm.name)) {
            Ok(_) => {
                state::remove_vm_from_state(&mut state, &vm.name);
                summary.deleted += 1;
                summary.results.push(OperationResult {
                    ok: true,
                    action: "rm".to_string(),
                    target: Some(vm.name.clone()),
                    message: "removed successfully".to_string(),
                    warnings: Vec::new(),
                });
            }
            Err(err) => {
                summary.failed += 1;
                summary.results.push(OperationResult {
                    ok: false,
                    action: "rm".to_string(),
                    target: Some(vm.name.clone()),
                    message: format!("failed to remove '{}': {}", vm.name, err),
                    warnings: Vec::new(),
                });
            }
        }
    }

    if !cfg.dry_run {
        state::save(&cfg.state_path, &state)?;
    }

    if reporter.is_json() {
        let response = if summary.failed > 0 {
            CommandResponse {
                command: "prune".to_string(),
                ok: false,
                data: Some(summary.clone()),
                warnings: Vec::new(),
                error: Some("partial failure".to_string()),
            }
        } else {
            CommandResponse::success("prune", summary.clone())
        };
        reporter.print_json(&response)?;
    } else {
        reporter.info(&format!(
            "summary: matched {}, deleted {}, failed {}, skipped {}",
            summary.matched, summary.deleted, summary.failed, summary.skipped
        ));
    }

    if summary.failed > 0 {
        return Ok(ExitCode::PartialFailure);
    }

    Ok(ExitCode::Success)
}

fn parse_older_than(raw: &str) -> Result<Duration> {
    if raw.len() < 2 {
        return Err(color_eyre::eyre::eyre!(
            "invalid older-than '{}': expected values like 24h or 7d",
            raw
        ));
    }

    let (num, unit) = raw.split_at(raw.len() - 1);
    let amount: i64 = num.parse()?;
    let duration = match unit {
        "h" => Duration::hours(amount),
        "d" => Duration::days(amount),
        _ => {
            return Err(color_eyre::eyre::eyre!(
                "invalid older-than unit '{}': use h or d",
                unit
            ));
        }
    };

    if duration <= Duration::zero() {
        return Err(color_eyre::eyre::eyre!(
            "older-than must be greater than zero"
        ));
    }

    Ok(duration)
}
