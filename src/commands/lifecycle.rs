use crate::cli::{DeleteArgs, ListArgs, NameArgs};
use crate::config::EffectiveConfig;
use crate::errors::ExitCode;
use crate::models::{CommandResponse, OperationResult};
use crate::output::Reporter;
use crate::state;
use crate::utm;
use color_eyre::Result;
use dialoguer::Confirm;

pub fn list(args: ListArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    let mut vms = utm::list_vms(cfg)?;
    let state = state::load(&cfg.state_path)?;

    let prefix = args.prefix.unwrap_or_else(|| cfg.default_prefix.clone());
    if !prefix.is_empty() {
        vms.retain(|vm| vm.name.starts_with(&prefix));
    }
    if let Some(os) = args.os {
        let os_value = os.as_str();
        vms.retain(|vm| {
            state
                .vms
                .get(&vm.name)
                .and_then(|m| m.os.as_deref())
                .is_some_and(|value| value == os_value)
        });
    }

    for vm in &mut vms {
        if let Some(meta) = state.vms.get(&vm.name) {
            vm.os = meta.os.clone();
            vm.created_at = Some(meta.created_at);
        }
    }

    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success("list", vms))?;
    } else {
        for vm in vms {
            reporter.print_stdout(&vm.name);
        }
    }

    Ok(ExitCode::Success)
}

pub fn status(args: NameArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    let mut vms = utm::list_vms(cfg)?;
    let state = state::load(&cfg.state_path)?;
    for vm in &mut vms {
        if let Some(meta) = state.vms.get(&vm.name) {
            vm.os = meta.os.clone();
            vm.created_at = Some(meta.created_at);
        }
    }

    let found = vms.into_iter().find(|vm| vm.name == args.name);

    if let Some(vm) = found {
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success("status", vm))?;
        } else {
            reporter.print_stdout(&vm.name);
        }
        return Ok(ExitCode::Success);
    }

    let msg = format!("vm '{}' not found", args.name);
    if reporter.is_json() {
        reporter.print_json(&CommandResponse::<OperationResult>::failure("status", msg))?;
    } else {
        reporter.error(&msg);
    }
    Ok(ExitCode::NotFound)
}

pub fn start(args: NameArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    mutate_vm("start", &args.name, cfg, reporter, utm::start_vm)
}

pub fn stop(args: NameArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    mutate_vm("stop", &args.name, cfg, reporter, utm::stop_vm)
}

pub fn open(args: NameArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    if cfg.dry_run {
        let result = OperationResult {
            ok: true,
            action: "open".to_string(),
            target: Some(args.name.clone()),
            message: format!("dry-run: would open '{}'", args.name),
            warnings: Vec::new(),
        };
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success("open", result))?;
        } else {
            reporter.info(&result.message);
        }
        return Ok(ExitCode::Success);
    }

    if let Err(err) = utm::open_vm(&args.name) {
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                "open",
                format!("{}", err),
            ))?;
        } else {
            reporter.error(&format!("{}", err));
        }
        return Ok(ExitCode::ExternalCommandFailed);
    }
    let result = OperationResult {
        ok: true,
        action: "open".to_string(),
        target: Some(args.name),
        message: "open succeeded".to_string(),
        warnings: Vec::new(),
    };
    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success("open", result))?;
    } else {
        reporter.info("open succeeded");
    }

    Ok(ExitCode::Success)
}

pub fn delete(args: DeleteArgs, cfg: &EffectiveConfig, reporter: &Reporter) -> Result<ExitCode> {
    if !cfg.yes && !args.force && !cfg.dry_run {
        let confirmed = Confirm::new()
            .with_prompt(format!("delete vm '{}' now?", args.name))
            .default(false)
            .interact()?;
        if !confirmed {
            reporter.info("aborted");
            return Ok(ExitCode::Success);
        }
    }

    let code = mutate_vm("delete", &args.name, cfg, reporter, utm::delete_vm)?;
    if matches!(code, ExitCode::Success) && !cfg.dry_run {
        state::remove_vm(&cfg.state_path, &args.name)?;
    }

    Ok(code)
}

fn mutate_vm<F>(
    action: &str,
    name: &str,
    cfg: &EffectiveConfig,
    reporter: &Reporter,
    f: F,
) -> Result<ExitCode>
where
    F: Fn(&EffectiveConfig, &str) -> Result<()>,
{
    if cfg.dry_run {
        let result = OperationResult {
            ok: true,
            action: action.to_string(),
            target: Some(name.to_string()),
            message: format!("dry-run: would {} '{}'", action, name),
            warnings: Vec::new(),
        };
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::success(action, result))?;
        } else {
            reporter.info(&result.message);
        }
        return Ok(ExitCode::Success);
    }

    if let Err(err) = f(cfg, name) {
        if reporter.is_json() {
            reporter.print_json(&CommandResponse::<OperationResult>::failure(
                action,
                format!("{}", err),
            ))?;
        } else {
            reporter.error(&format!("{}", err));
        }
        return Ok(ExitCode::ExternalCommandFailed);
    }

    let result = OperationResult {
        ok: true,
        action: action.to_string(),
        target: Some(name.to_string()),
        message: format!("{} succeeded for '{}'", action, name),
        warnings: Vec::new(),
    };
    if reporter.is_json() {
        reporter.print_json(&CommandResponse::success(action, result))?;
    } else {
        reporter.info(&result.message);
    }

    Ok(ExitCode::Success)
}
