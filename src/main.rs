use clap::Parser;
use color_eyre::Result;
use utmd::cli::{Cli, Commands};
use utmd::commands::{clone, delete_all, lifecycle};
use utmd::config::load_effective;
use utmd::errors::ExitCode;
use utmd::output::Reporter;
use utmd::utm;

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let cfg = load_effective(&cli)?;
    let reporter = Reporter::new(cfg.json, cfg.quiet);

    if let Err(err) = utm::ensure_utmctl(&cfg, &reporter) {
        reporter.error(&format!("{}", err));
        return Ok(ExitCode::DependencyMissing);
    }

    let code = match cli.command {
        Commands::Clone(args) => clone::run(args, &cfg, &reporter)?,
        Commands::List(args) => lifecycle::list(args, &cfg, &reporter)?,
        Commands::Status(args) => lifecycle::status(args, &cfg, &reporter)?,
        Commands::Start(args) => lifecycle::start(args, &cfg, &reporter)?,
        Commands::Stop(args) => lifecycle::stop(args, &cfg, &reporter)?,
        Commands::Open(args) => lifecycle::open(args, &cfg, &reporter)?,
        Commands::Delete(args) => lifecycle::delete(args, &cfg, &reporter)?,
        Commands::DeleteAll(args) => delete_all::run(args, &cfg, &reporter)?,
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
