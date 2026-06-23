use clap::Parser;
use color_eyre::Result;
use utmd::cli::{Cli, Commands};
use utmd::commands::{clone, delete_all, init, lifecycle, spawn};
use utmd::config::load_effective;
use utmd::errors::ExitCode;
use utmd::output::Reporter;
use utmd::utm;

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let cfg = load_effective(&cli)?;
    let reporter = Reporter::new(cfg.json, cfg.quiet);

    if let Commands::Init(args) = cli.command {
        return init::run(args, &cfg, &reporter);
    }

    if let Err(err) = utm::ensure_utmctl(&cfg, &reporter) {
        reporter.error(&format!("{}", err));
        return Ok(ExitCode::DependencyMissing);
    }

    let code = match cli.command {
        Commands::Create(args) => clone::run(args, &cfg, &reporter)?,
        Commands::Run(args) => spawn::run(args, &cfg, &reporter)?,
        Commands::Init(_) => unreachable!("init handled before dependency checks"),
        Commands::Ls(args) => lifecycle::list(args, &cfg, &reporter)?,
        Commands::Inspect(args) => lifecycle::status(args, &cfg, &reporter)?,
        Commands::Start(args) => lifecycle::start(args, &cfg, &reporter)?,
        Commands::Stop(args) => lifecycle::stop(args, &cfg, &reporter)?,
        Commands::Show(args) => lifecycle::open(args, &cfg, &reporter)?,
        Commands::Rm(args) => lifecycle::delete(args, &cfg, &reporter)?,
        Commands::Prune(args) => delete_all::run(args, &cfg, &reporter)?,
    };

    Ok(code)
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let code = match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {}", err);
            ExitCode::InvalidUsage
        }
    };

    std::process::exit(code.as_i32());
}
