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
        about = "clone a new sandbox VM from a template",
        after_help = "examples:\n  utmd clone linux\n  utmd clone linux --name sandbox1\n  utmd clone macos --name-template \"{prefix}{os}-{date}-{rand}\""
    )]
    Clone(CloneArgs),
    #[command(
        about = "list managed or filtered VMs",
        after_help = "examples:\n  utmd list\n  utmd list --prefix \"\"\n  utmd list --prefix utmd- --os linux"
    )]
    List(ListArgs),
    #[command(
        about = "show details for a VM",
        after_help = "examples:\n  utmd status utmd-linux-abc123"
    )]
    Status(NameArgs),
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
        about = "open a VM in the UTM app",
        after_help = "examples:\n  utmd open utmd-linux-abc123"
    )]
    Open(NameArgs),
    #[command(
        about = "delete a single VM",
        after_help = "examples:\n  utmd delete utmd-linux-abc123\n  utmd --yes delete utmd-linux-abc123"
    )]
    Delete(DeleteArgs),
    #[command(
        about = "delete multiple VMs by filters",
        after_help = "examples:\n  utmd delete-all\n  utmd delete-all --prefix utmd- --os linux --older-than 24h --dry-run\n  utmd --yes delete-all --prefix utmd-"
    )]
    DeleteAll(DeleteAllArgs),
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
