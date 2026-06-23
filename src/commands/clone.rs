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
    if let Some(prefix) = args.prefix {
        effective_cfg.default_prefix = prefix;
    }

    let name = match generate_name(
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
                    "create",
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
            action: "create".to_string(),
            target: Some(name.clone()),
            message: format!("dry-run: would clone '{}' from '{}'", name, template),
            warnings: Vec::new(),
        };
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success("create", result))?;
        } else {
            reporter.info(&result.message);
        }
        return Ok(ExitCode::Success);
    }

    reporter.info(&format!("cloning '{}' to '{}'", template, name));
    if let Err(err) = utm::clone_vm(cfg, template, &name) {
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                "create",
                format!("{}", err),
            ))?;
        } else {
            reporter.error(&format!("{}", err));
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

    let result = OperationResult {
        ok: true,
        action: "create".to_string(),
        target: Some(name),
        message: "create completed successfully".to_string(),
        warnings: Vec::new(),
    };

    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success("create", result))?;
    } else {
        reporter.info(&result.message);
    }

    Ok(ExitCode::Success)
}
