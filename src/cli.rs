use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "utmd",
    about = "Disposable VM sandbox manager for UTM",
    long_about = "A developer tool to clone, boot, and clean disposable UTM sandbox environments."
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub quiet: bool,
    #[arg(long, global = true)]
    pub yes: bool,
    #[arg(long, global = true)]
    pub dry_run: bool,
    #[arg(long, global = true)]
    pub config: Option<String>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        about = "create a new sandbox VM from a template",
        after_help = "examples:\n  utmd create linux\n  utmd create linux --name sandbox1\n  utmd create macos --name-template \"{prefix}{os}-{rand}\""
    )]
    Create(CloneArgs),
    #[command(
        about = "create, start, and show a sandbox VM",
        after_help = "examples:\n  utmd run linux\n  utmd run macos\n  utmd run linux --name dev --name-template \"{prefix}{os}-{rand}\""
    )]
    Run(CloneArgs),
    #[command(
        about = "create a boilerplate config file",
        after_help = "examples:\n  utmd init\n  utmd --config /tmp/utmd.toml init\n  utmd init --force"
    )]
    Init(InitArgs),
    #[command(
        about = "list managed or filtered VMs",
        after_help = "examples:\n  utmd ls\n  utmd ls --prefix \"\"\n  utmd ls --prefix utmd- --os linux"
    )]
    Ls(ListArgs),
    #[command(
        about = "inspect details for a VM",
        after_help = "examples:\n  utmd inspect utmd-linux-abc123"
    )]
    Inspect(NameArgs),
    #[command(
        about = "start a VM",
        after_help = "examples:\n  utmd start utmd-linux-abc123\n  utmd --dry-run start utmd-linux-abc123"
    )]
    Start(NameArgs),
    #[command(
        about = "stop a VM",
        after_help = "examples:\n  utmd stop utmd-linux-abc123\n  utmd --dry-run stop utmd-linux-abc123"
    )]
    Stop(NameArgs),
    #[command(
        about = "show a VM in the UTM app",
        after_help = "examples:\n  utmd show utmd-linux-abc123"
    )]
    Show(NameArgs),
    #[command(
        about = "remove a single VM",
        after_help = "examples:\n  utmd rm utmd-linux-abc123\n  utmd --yes rm utmd-linux-abc123"
    )]
    Rm(DeleteArgs),
    #[command(
        about = "remove multiple VMs by filters",
        after_help = "examples:\n  utmd prune\n  utmd prune --prefix utmd- --os linux --older-than 24h --dry-run\n  utmd --yes prune --prefix utmd-"
    )]
    Prune(DeleteAllArgs),
}

#[derive(Args, Debug)]
pub struct CloneArgs {
    #[arg(value_enum, value_name = "OS")]
    pub os_type: OsType,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub name_exact: bool,
    #[arg(long)]
    pub name_template: Option<String>,
    #[arg(long)]
    pub prefix: Option<String>,
    #[arg(long)]
    pub template: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub prefix: Option<String>,
    #[arg(long, value_enum)]
    pub os: Option<OsType>,
}

#[derive(Args, Debug)]
pub struct NameArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    pub name: String,
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct DeleteAllArgs {
    #[arg(long)]
    pub prefix: Option<String>,
    #[arg(long, value_enum)]
    pub os: Option<OsType>,
    #[arg(long)]
    pub older_than: Option<String>,
}

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OsType {
    Linux,
    Macos,
}

impl OsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }
}
