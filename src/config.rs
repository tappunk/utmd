use crate::cli::{Cli, OsType};
use color_eyre::{Result, eyre::bail};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    pub utm_app: Option<String>,
    pub utmctl_path: Option<String>,
    pub state_path: Option<String>,
    pub default_prefix: Option<String>,
    pub templates: Option<TemplateConfig>,
    pub naming: Option<NamingConfig>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateConfig {
    pub linux: Option<String>,
    pub macos: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamingConfig {
    pub default_template: Option<String>,
    pub rand_len: Option<usize>,
    pub max_retries: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    pub default_json: Option<bool>,
    pub default_quiet: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub config_path: PathBuf,
    pub utm_app: String,
    pub utmctl_path: String,
    pub state_path: PathBuf,
    pub default_prefix: String,
    pub template_linux: String,
    pub template_macos: String,
    pub naming_template: String,
    pub naming_rand_len: usize,
    pub naming_max_retries: u8,
    pub json: bool,
    pub quiet: bool,
    pub yes: bool,
    pub dry_run: bool,
}

impl EffectiveConfig {
    pub fn template_for(&self, os: OsType) -> &str {
        match os {
            OsType::Linux => &self.template_linux,
            OsType::Macos => &self.template_macos,
        }
    }
}

pub fn load_effective(cli: &Cli) -> Result<EffectiveConfig> {
    let config_path = if let Some(ref path) = cli.config {
        let path = PathBuf::from(path);
        if !path.exists() {
            bail!("config file '{}' does not exist", path.display());
        }
        path
    } else {
        resolve_config_path(cli).unwrap_or_else(default_config_path)
    };
    let mut cfg = EffectiveConfig {
        config_path: config_path.clone(),
        utm_app: "/Applications/UTM.app".to_string(),
        utmctl_path: "/usr/local/bin/utmctl".to_string(),
        state_path: default_state_path(),
        default_prefix: "utmd-".to_string(),
        template_linux: "[t]-linux".to_string(),
        template_macos: "[t]-macos".to_string(),
        naming_template: "{prefix}{os}-{rand}".to_string(),
        naming_rand_len: 4,
        naming_max_retries: 8,
        json: false,
        quiet: false,
        yes: false,
        dry_run: false,
    };

    merge_file_config(&mut cfg, &config_path)?;

    merge_env_config(&mut cfg);

    cfg.json = cli.json || cfg.json;
    cfg.quiet = cli.quiet || cfg.quiet;
    cfg.yes = cli.yes;
    cfg.dry_run = cli.dry_run;

    validate(&cfg)?;
    Ok(cfg)
}

pub fn boilerplate_config(cfg: &EffectiveConfig) -> String {
    format!(
        "utm_app = \"{}\"\nutmctl_path = \"{}\"\nstate_path = \"{}\"\ndefault_prefix = \"{}\"\n\n[templates]\nlinux = \"{}\"\nmacos = \"{}\"\n\n[naming]\ndefault_template = \"{}\"\nrand_len = {}\nmax_retries = {}\n\n[output]\ndefault_json = {}\ndefault_quiet = {}\n",
        cfg.utm_app,
        cfg.utmctl_path,
        cfg.state_path.display(),
        cfg.default_prefix,
        cfg.template_linux,
        cfg.template_macos,
        cfg.naming_template,
        cfg.naming_rand_len,
        cfg.naming_max_retries,
        cfg.json,
        cfg.quiet,
    )
}

fn resolve_config_path(cli: &Cli) -> Option<PathBuf> {
    if let Some(path) = &cli.config {
        return Some(PathBuf::from(path));
    }

    dirs::config_dir().map(|dir| dir.join("utmd").join("config.toml"))
}

fn default_config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join("utmd").join("config.toml");
    }

    PathBuf::from("/tmp/utmd/config.toml")
}

fn merge_file_config(cfg: &mut EffectiveConfig, path: &PathBuf) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let parsed: FileConfig = toml::from_str(&content)?;

    if let Some(v) = parsed.utm_app {
        cfg.utm_app = v;
    }
    if let Some(v) = parsed.utmctl_path {
        cfg.utmctl_path = v;
    }
    if let Some(v) = parsed.state_path {
        cfg.state_path = PathBuf::from(v);
    }
    if let Some(v) = parsed.default_prefix {
        cfg.default_prefix = v;
    }

    if let Some(templates) = parsed.templates {
        if let Some(v) = templates.linux {
            cfg.template_linux = v;
        }
        if let Some(v) = templates.macos {
            cfg.template_macos = v;
        }
    }

    if let Some(naming) = parsed.naming {
        if let Some(v) = naming.default_template {
            cfg.naming_template = v;
        }
        if let Some(v) = naming.rand_len {
            cfg.naming_rand_len = v;
        }
        if let Some(v) = naming.max_retries {
            cfg.naming_max_retries = v;
        }
    }

    if let Some(output) = parsed.output {
        if let Some(v) = output.default_json {
            cfg.json = v;
        }
        if let Some(v) = output.default_quiet {
            cfg.quiet = v;
        }
    }

    Ok(())
}

fn merge_env_config(cfg: &mut EffectiveConfig) {
    if let Ok(v) = std::env::var("UTMD_UTM_APP") {
        cfg.utm_app = v;
    }
    if let Ok(v) = std::env::var("UTMD_UTMCTL_PATH") {
        cfg.utmctl_path = v;
    }
    if let Ok(v) = std::env::var("UTMD_STATE_PATH") {
        cfg.state_path = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("UTMD_PREFIX") {
        cfg.default_prefix = v;
    }
    if let Ok(v) = std::env::var("UTMD_TEMPLATE_LINUX") {
        cfg.template_linux = v;
    }
    if let Ok(v) = std::env::var("UTMD_TEMPLATE_MACOS") {
        cfg.template_macos = v;
    }
    if let Ok(v) = std::env::var("UTMD_JSON") {
        cfg.json = matches!(v.as_str(), "1" | "true" | "yes");
    }
    if let Ok(v) = std::env::var("UTMD_QUIET") {
        cfg.quiet = matches!(v.as_str(), "1" | "true" | "yes");
    }
}

fn validate(cfg: &EffectiveConfig) -> Result<()> {
    if cfg.default_prefix.trim().is_empty() {
        bail!("default prefix must not be empty");
    }
    if cfg.template_linux.trim().is_empty() || cfg.template_macos.trim().is_empty() {
        bail!("template names must not be empty");
    }
    if cfg.naming_rand_len == 0 {
        bail!("naming rand length must be greater than zero");
    }
    if cfg.naming_max_retries == 0 {
        bail!("naming max retries must be greater than zero");
    }

    Ok(())
}

fn default_state_path() -> PathBuf {
    if let Some(dir) = dirs::state_dir() {
        return dir.join("utmd").join("state.json");
    }

    if let Some(dir) = dirs::config_dir() {
        return dir.join("utmd").join("state.json");
    }

    PathBuf::from("/tmp/utmd-state.json")
}
