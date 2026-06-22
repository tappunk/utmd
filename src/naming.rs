use crate::cli::OsType;
use crate::config::EffectiveConfig;
use chrono::Local;
use color_eyre::{Result, eyre::bail};
use rand::{Rng, distr::Alphanumeric};
use std::collections::HashSet;

pub fn generate_name(
    cfg: &EffectiveConfig,
    os: OsType,
    custom_name: Option<&str>,
    name_exact: bool,
    name_template: Option<&str>,
    existing: &HashSet<String>,
) -> Result<String> {
    if let Some(name) = custom_name {
        let final_name = if name_exact || name.starts_with(&cfg.default_prefix) {
            name.to_string()
        } else {
            format!("{}{}", cfg.default_prefix, name)
        };

        if existing.contains(&final_name) {
            bail!("vm name '{}' already exists", final_name);
        }
        return Ok(final_name);
    }

    let template = name_template.unwrap_or(&cfg.naming_template);
    for _ in 0..cfg.naming_max_retries {
        let candidate = render_template(template, &cfg.default_prefix, os, cfg.naming_rand_len);
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }

    bail!(
        "failed to generate unique vm name after {} retries",
        cfg.naming_max_retries
    )
}

fn render_template(template: &str, prefix: &str, os: OsType, rand_len: usize) -> String {
    let now = Local::now();
    let rand: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(rand_len)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();

    template
        .replace("{prefix}", prefix)
        .replace("{os}", os.as_str())
        .replace("{date}", &now.format("%Y%m%d").to_string())
        .replace("{time}", &now.format("%H%M%S").to_string())
        .replace("{rand}", &rand)
}
