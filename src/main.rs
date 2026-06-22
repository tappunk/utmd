use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::{Result, eyre::bail};
use dialoguer::Confirm;
use md5::{Digest, Md5};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const UTM_APP: &str = "/Applications/UTM.app";
const UTMCTL: &str = "/usr/local/bin/utmctl";
const TEMPLATE_LINUX: &str = "[t]-linux";
const TEMPLATE_MACOS: &str = "[t]-macos";
const PREFIX: &str = "utmd-";

#[derive(Parser)]
#[command(
    name = "utmd",
    about = "Disposable VM sandbox manager for UTM",
    long_about = "A developer tool to instantly clone, boot, and clean up disposable UTM sandbox environments."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a VM from a base template
    Clone {
        /// Operating system template (linux or macos)
        #[arg(value_enum, value_name = "OS")]
        os_type: OsType,

        /// Custom VM name (will be automatically prefixed with "utmd-")
        name: Option<String>,
    },
    /// Delete all generated sandbox VMs
    DeleteAll,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OsType {
    Linux,
    Macos,
}

impl OsType {
    fn template(&self) -> &str {
        match self {
            OsType::Linux => TEMPLATE_LINUX,
            OsType::Macos => TEMPLATE_MACOS,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    ensure_utmctl()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Clone { os_type, name } => {
            handle_clone(os_type, name).await?;
        }
        Commands::DeleteAll => {
            handle_delete_all().await?;
        }
    }

    Ok(())
}

fn ensure_utmctl() -> Result<()> {
    if which::which(UTMCTL).is_ok()
        || Command::new("command")
            .args(["-v", "utmctl"])
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    {
        return Ok(());
    }

    eprintln!("error: utmctl not found. creating symlink...");
    let proceed = Confirm::new()
        .with_prompt("Create symlink now? (requires sudo)")
        .default(true)
        .interact()?;

    if !proceed {
        bail!("aborted by user");
    }

    let status = Command::new("sudo")
        .args([
            "ln",
            "-sf",
            &format!("{}/Contents/MacOS/utmctl", UTM_APP),
            UTMCTL,
        ])
        .status()?;

    if !status.success() {
        bail!("failed to create symlink via sudo");
    }
    eprintln!("info: symlink created");
    Ok(())
}

async fn handle_clone(os_type: OsType, custom_name: Option<String>) -> Result<()> {
    let template = os_type.template();

    let new_name = match custom_name {
        Some(name) => {
            if name.starts_with(PREFIX) {
                name
            } else {
                format!("{}{}", PREFIX, name)
            }
        }
        None => {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let mut hasher = Md5::new();
            hasher.update(nanos.to_string().as_bytes());
            let hash_str = format!("{:x}", hasher.finalize());
            format!(
                "{}{}-{:.4}",
                PREFIX,
                format!("{:?}", os_type).to_lowercase(),
                hash_str
            )
        }
    };

    let list_output = Command::new("utmctl").arg("list").output()?;
    let list_str = String::from_utf8_lossy(&list_output.stdout);
    if !list_str.contains(template) {
        bail!("template '{}' not found in utmctl registry", template);
    }

    eprintln!("info: cloning '{}' to '{}'", template, new_name);

    let clone_status = Command::new("utmctl")
        .args(["clone", template, "--name", &new_name])
        .status()?;

    if !clone_status.success() {
        bail!("failed to clone VM");
    }
    eprintln!("info: clone completed successfully");

    eprintln!("info: randomizing network mac address...");
    let mac_script = format!(
        "tell application \"UTM\" to set address of item 1 of network interfaces of virtual machine named \"{}\" to \"\"",
        new_name
    );
    let _ = Command::new("osascript")
        .args(["-e", &mac_script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    eprintln!("info: launching and starting '{}'", new_name);
    let boot_script = format!(
        "tell application \"UTM\"\nactivate\nstart virtual machine named \"{}\"\nend tell",
        new_name
    );
    let _ = Command::new("osascript")
        .args(["-e", &boot_script])
        .status();

    eprintln!(
        "info: '{}' is booting directly into the gui layout",
        new_name
    );
    eprintln!("\nuseful commands:\n   utmctl list\n   utmd delete-all");

    Ok(())
}

async fn handle_delete_all() -> Result<()> {
    eprintln!(
        "warning: this will stop and permanently delete all sandbox vms prefixed with '{}'",
        PREFIX
    );
    let confirm = Confirm::new()
        .with_prompt("Are you absolutely sure?")
        .default(false)
        .interact()?;

    if !confirm {
        eprintln!("info: aborted");
        return Ok(());
    }

    eprintln!("info: fetching generated vm list...");
    let output = Command::new("utmctl").arg("list").output()?;
    let output_str = String::from_utf8_lossy(&output.stdout);

    let mut guest_vms: Vec<String> = Vec::new();
    let lines = output_str.lines().skip(1);

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3
            && let Some(idx) = line.find(parts[2])
        {
            let vm_name = line[idx..].trim().to_string();

            if vm_name.starts_with(PREFIX) {
                guest_vms.push(vm_name);
            }
        }
    }

    guest_vms.sort();
    guest_vms.dedup();

    if guest_vms.is_empty() {
        eprintln!("info: no generated vms found. workspace is already clean");
        return Ok(());
    }

    eprintln!(
        "info: found {} unique sandbox vm(s) to clear out",
        guest_vms.len()
    );
    for vm in guest_vms {
        eprintln!("info: processing '{}'", vm);
        let _ = Command::new("utmctl")
            .args(["stop", &vm])
            .stderr(Stdio::null())
            .status();

        let delete_status = Command::new("utmctl").args(["delete", &vm]).status()?;
        if delete_status.success() {
            eprintln!("info: deleted successfully");
        } else {
            eprintln!("error: failed to delete '{}'", vm);
        }
    }

    eprintln!("info: workspace sweep finished");
    Ok(())
}

mod which {
    use std::path::Path;
    pub fn which<P: AsRef<Path>>(binary_name: P) -> Result<std::path::PathBuf, &'static str> {
        if binary_name.as_ref().exists() {
            return Ok(binary_name.as_ref().to_path_buf());
        }
        std::env::var_os("PATH")
            .and_then(|paths| {
                std::env::split_paths(&paths)
                    .filter_map(|dir| {
                        let full_path = dir.join(&binary_name);
                        if full_path.is_file() {
                            Some(full_path)
                        } else {
                            None
                        }
                    })
                    .next()
            })
            .ok_or("Binary not found in system path targets")
    }
}
