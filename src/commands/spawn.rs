use crate::cli::CloneArgs;
use crate::config::EffectiveConfig;
use crate::errors::ExitCode;
use crate::models::{CommandResponse, OperationResult};
use crate::naming::generate_name;
use crate::output::Reporter;
use crate::state::{self, VmMetadata};
use crate::utm;
use chrono::Utc;
use color_eyre::Result;
use std::collections::HashSet;

pub fn run(args: CloneArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    let template = args
        .template
        .as_deref()
        .unwrap_or_else(|| cfg.template_for(args.os_type));

    let existing = utm::list_vms(cfg)?
        .into_iter()
        .map(|vm| vm.name)
        .collect::<HashSet<_>>();

    let mut effective_cfg = cfg.clone();
    if let Some(ref prefix) = args.prefix {
        effective_cfg.default_prefix = prefix.clone();
    }

    let mut name = match generate_name(
        &effective_cfg,
        args.os_type,
        args.name.as_deref(),
        args.name_exact,
        args.name_template.as_deref(),
        &existing,
    ) {
        Ok(name) => name,
        Err(err) => {
            if reporter.is_json() {
                reporter.print_json(&CommandResponse::<OperationResult>::failure(
                    "run",
                    format!("{}", err),
                ))?;
            } else {
                reporter.error(&format!("{}", err));
            }
            return Ok(ExitCode::Conflict);
        }
    };

    if effective_cfg.dry_run {
        let result = OperationResult {
            ok: true,
            action: "run".to_string(),
            target: Some(name.clone()),
            message: format!(
                "dry-run: would clone, start, and open '{}' from '{}'",
                name, template
            ),
            warnings: Vec::new(),
        };
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success("run", result))?;
        } else {
            reporter.info(&result.message);
        }
        return Ok(ExitCode::Success);
    }

    reporter.info(&format!("cloning '{}' to '{}'", template, name));
    let mut clone_err: Option<String> = None;
    for attempt in 0..=2 {
        if attempt > 0 {
            let fresh: HashSet<String> = utm::list_vms(cfg)
                .map(|vms| vms.into_iter().map(|vm| vm.name).collect())
                .unwrap_or_default();
            let mut retry_cfg = cfg.clone();
            if let Some(ref prefix) = args.prefix {
                retry_cfg.default_prefix = prefix.clone();
            }
            match generate_name(
                &retry_cfg,
                args.os_type,
                None,
                false,
                None,
                &fresh,
            ) {
                Ok(n) => name = n,
                Err(_) => {
                    return Ok(ExitCode::Conflict);
                }
            }
        }
        match utm::clone_vm(cfg, template, &name) {
            Ok(()) => break,
            Err(e) => {
                let fresh = utm::list_vms(cfg).unwrap_or_default();
                if fresh.iter().any(|vm| vm.name == name) {
                    continue;
                }
                clone_err = Some(format!("{}", e));
                break;
            }
        }
    }
    if let Some(err) = clone_err {
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                "run",
                err,
            ))?;
        } else {
            reporter.error(&err);
        }
        return Ok(ExitCode::ExternalCommandFailed);
    }

    state::upsert_vm(
        &cfg.state_path,
        VmMetadata {
            name: name.clone(),
            os: Some(args.os_type.as_str().to_string()),
            template: Some(template.to_string()),
            created_at: Utc::now(),
        },
    )?;

    reporter.info(&format!("starting '{}'", name));
    if let Err(err) = utm::start_vm(cfg, &name) {
        eprintln!("warning: cleaning up orphaned vm '{}'", name);
        let _ = utm::delete_vm(cfg, &name);
        let _ = state::remove_vm(&cfg.state_path, &name);
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                "run",
                format!("{}", err),
            ))?;
        } else {
            reporter.error(&format!("{}", err));
        }
        return Ok(ExitCode::ExternalCommandFailed);
    }

    reporter.info(&format!("opening '{}' in UTM", name));
    if let Err(err) = utm::open_vm(&name) {
        eprintln!("warning: cleaning up orphaned vm '{}'", name);
        let _ = utm::stop_vm(cfg, &name);
        let _ = utm::delete_vm(cfg, &name);
        let _ = state::remove_vm(&cfg.state_path, &name);
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                "run",
                format!("{}", err),
            ))?;
        } else {
            reporter.error(&format!("{}", err));
        }
        return Ok(ExitCode::ExternalCommandFailed);
    }

    let result = OperationResult {
        ok: true,
        action: "run".to_string(),
        target: Some(name),
        message: "run completed successfully".to_string(),
        warnings: Vec::new(),
    };

    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success("run", result))?;
    } else {
        reporter.info(&result.message);
    }

    Ok(ExitCode::Success)
}
