use crate::cli::InitArgs;
use crate::config::{EffectiveConfig, boilerplate_config};
use crate::errors::ExitCode;
use crate::models::{CommandResponse, OperationResult};
use crate::output::Reporter;
use color_eyre::Result;
use std::fs;

pub fn run(args: InitArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    let path = &cfg.config_path;

    if cfg.dry_run {
        let result = OperationResult {
            ok: true,
            action: "init".to_string(),
            target: Some(path.display().to_string()),
            message: if path.exists() {
                format!("dry-run: would overwrite config at '{}'", path.display())
            } else {
                format!("dry-run: would write config to '{}'", path.display())
            },
            warnings: Vec::new(),
        };
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success("init", result))?;
        } else {
            reporter.info(&result.message);
        }
        return Ok(ExitCode::Success);
    }

    if path.exists() && !args.force {
        let msg = format!(
            "config file '{}' already exists, rerun with --force to overwrite",
            path.display()
        );
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure("init", msg))?;
        } else {
            reporter.error(&msg);
        }
        return Ok(ExitCode::Conflict);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, boilerplate_config(cfg))?;

    let result = OperationResult {
        ok: true,
        action: "init".to_string(),
        target: Some(path.display().to_string()),
        message: format!("wrote config to '{}'", path.display()),
        warnings: Vec::new(),
    };

    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success("init", result))?;
    } else {
        reporter.info(&result.message);
        reporter.info("set templates.linux and templates.macos to existing UTM VM template names");
    }

    Ok(ExitCode::Success)
}
