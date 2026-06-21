use clap::{Parser, Subcommand, ValueEnum};
use dialoguer::Confirm;
use md5::{Digest, Md5};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const UTM_APP: &str = "/Applications/UTM.app";
const UTMCTL: &str = "/usr/local/bin/utmctl";
const TEMPLATE_LINUX: &str = "[t]-linux";
const TEMPLATE_MACOS: &str = "[t]-macos";

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

        /// Custom VM name (auto-generated if omitted)
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

fn ensure_utmctl() -> Result<(), Box<dyn std::error::Error>> {
    if which::which(UTMCTL).is_ok()
        || Command::new("command")
            .args(["-v", "utmctl"])
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    {
        return Ok(());
    }

    eprintln!("utmctl not found. Creating symlink...");
    let proceed = Confirm::new()
        .with_prompt("Create symlink now? (requires sudo)")
        .default(true)
        .interact()?;

    if !proceed {
        eprintln!("Aborted.");
        std::process::exit(1);
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
        eprintln!("[ERROR] Failed to create symlink via sudo.");
        std::process::exit(1);
    }
    println!("[OK] Symlink created.");
    Ok(())
}

async fn handle_clone(
    os_type: OsType,
    custom_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let template = os_type.template();

    let new_name = match custom_name {
        Some(name) => name,
        None => {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let mut hasher = Md5::new();
            hasher.update(nanos.to_string().as_bytes());
            let hash_str = format!("{:x}", hasher.finalize());
            format!("{:?}-{:.4}", os_type, hash_str).to_lowercase()
        }
    };

    let list_output = Command::new("utmctl").arg("list").output()?;
    let list_str = String::from_utf8_lossy(&list_output.stdout);
    if !list_str.contains(template) {
        eprintln!(
            "Error: Template '{}' not found in utmctl registry.",
            template
        );
        std::process::exit(1);
    }

    println!("Cloning '{}' to '{}'...", template, new_name);

    let clone_status = Command::new("utmctl")
        .args(["clone", template, "--name", &new_name])
        .status()?;

    if !clone_status.success() {
        eprintln!("Error: Failed to clone VM.");
        std::process::exit(1);
    }
    println!("[OK] Clone completed successfully");

    println!("Randomizing network MAC address...");
    let mac_script = format!(
        "tell application \"UTM\" to set address of item 1 of network interfaces of virtual machine named \"{}\" to \"\"",
        new_name
    );
    let _ = Command::new("osascript")
        .args(["-e", &mac_script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    println!("Launching and starting '{}'...", new_name);
    let boot_script = format!(
        "tell application \"UTM\"\nactivate\nstart virtual machine named \"{}\"\nend tell",
        new_name
    );
    let _ = Command::new("osascript")
        .args(["-e", &boot_script])
        .status();

    println!(
        "[OK] '{}' is booting directly into the GUI layout.",
        new_name
    );
    println!("\nUseful commands:\n   utmctl list\n   utmd delete-all");

    Ok(())
}

async fn handle_delete_all() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[WARN] This will stop and permanently delete all generated sandbox VMs.");
    let confirm = Confirm::new()
        .with_prompt("Are you absolutely sure?")
        .default(false)
        .interact()?;

    if !confirm {
        println!("Aborted.");
        return Ok(());
    }

    println!("Fetching generated VM list...");
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

            if !matches!(
                vm_name.as_str(),
                "[t]-linux" | "[t]-macos" | "template-linux" | "template-macos"
            ) {
                guest_vms.push(vm_name);
            }
        }
    }

    guest_vms.sort();
    guest_vms.dedup();

    if guest_vms.is_empty() {
        println!("[OK] No generated VMs found. Workspace is already clean.");
        return Ok(());
    }

    println!(
        "Found {} unique sandbox VM(s) to clear out.",
        guest_vms.len()
    );
    for vm in guest_vms {
        println!("Processing '{}'", vm);
        let _ = Command::new("utmctl")
            .args(["stop", &vm])
            .stderr(Stdio::null())
            .status();

        let delete_status = Command::new("utmctl").args(["delete", &vm]).status()?;
        if delete_status.success() {
            println!("[OK] Deleted successfully.");
        } else {
            eprintln!("[ERROR] Failed to delete '{}'.", vm);
        }
    }

    println!("[OK] Workspace sweep finished.");
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
