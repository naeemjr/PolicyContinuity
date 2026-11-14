//! Command-line entry point.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};

use policycontinuity::adapters::{self, Adapter};
use policycontinuity::error::Error;
use policycontinuity::experiment;
use policycontinuity::metrics::METRICS;
use policycontinuity::model::{Family, RunOutcome, Split};
use policycontinuity::protocol;
use policycontinuity::runner::{RunRow, Session};
use policycontinuity::scenario::{self, Scenario};
use policycontinuity::sys;

/// Controlled Linux benchmark: does a filesystem-containment policy survive
/// state transitions, retries, and compatibility fallbacks?
///
/// Everything runs in temporary directories created under --temp-parent.
/// "Outside the root" always means a synthetic sentinel in that workspace.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Temp {
    /// Directory under which temporary workspaces are created. It is
    /// created if missing. Keep it inside the project.
    #[arg(long, default_value = "target/pc-tmp")]
    temp_parent: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Run the development seeds and write tables to --out.
    Dev {
        #[arg(long, default_value = "results/dev")]
        out: PathBuf,
        #[command(flatten)]
        temp: Temp,
    },
    /// Run the frozen held-out evaluation and write every table and figure.
    Final {
        #[arg(long, default_value = "results/final")]
        out: PathBuf,
        #[command(flatten)]
        temp: Temp,
    },
    /// Run one development seed and print each run.
    Run(RunArgs),
    /// Show one run in full, including the live handle identity.
    Inspect {
        /// A development seed.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Scenario id, as printed by `list scenarios`.
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        adapter: String,
        #[command(flatten)]
        temp: Temp,
    },
    /// List adapters, families, metrics, or one seed's scenarios.
    List {
        #[arg(value_enum)]
        what: Listing,
        /// Seed for `list scenarios` (development seeds only).
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Print the frozen protocol constants.
    Protocol,
    /// Report the kernel release and whether openat2 is usable here.
    Probe {
        #[command(flatten)]
        temp: Temp,
    },
}

#[derive(Args)]
struct RunArgs {
    /// A development seed.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Only these adapters (repeatable).
    #[arg(long = "adapter")]
    adapters: Vec<String>,
    /// Only these scenario families (repeatable).
    #[arg(long = "family")]
    families: Vec<Family>,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    #[command(flatten)]
    temp: Temp,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Listing {
    Adapters,
    Families,
    Metrics,
    Outcomes,
    Scenarios,
}

type CliResult = Result<(), Error>;

/// Held-out seeds are reserved for `final`: looking at them one at a time
/// would defeat the split.
fn development_seed(seed: u64) -> Result<u64, Error> {
    if protocol::held_out_seeds().contains(&seed) {
        return Err(Error::Usage(format!(
            "seed {seed} is a held-out seed; it is only run by `final`"
        )));
    }
    Ok(seed)
}

fn pick_adapters(names: &[String]) -> Result<Vec<Box<dyn Adapter>>, Error> {
    if names.is_empty() {
        return Ok(adapters::all());
    }
    names
        .iter()
        .map(|name| adapters::by_name(name).map_err(Error::Usage))
        .collect()
}

fn write_outputs(eval: &experiment::Evaluation, out: &Path) -> CliResult {
    let files = experiment::artifacts(eval);
    experiment::write(out, &files)?;
    std::fs::write(
        out.join("environment.json"),
        experiment::environment_json(eval),
    )
    .map_err(|source| Error::Io {
        context: "writing environment.json".to_string(),
        source,
    })?;
    print!("{}", experiment::console_summary(eval));
    println!("\nwrote {} files to {}", files.len() + 1, out.display());
    Ok(())
}

fn text_line(row: &RunRow) -> String {
    format!(
        "{:<34} {:<24} {:<9} {:<7} {:<14} {:<22} {}",
        row.record.scenario_id,
        row.record.adapter,
        row.judgement.condition.as_str(),
        row.record.reported.as_str(),
        row.record.path_label.as_str(),
        row.judgement.outcome.as_str(),
        row.record.schedule
    )
}

fn run(args: &RunArgs) -> CliResult {
    let seed = development_seed(args.seed)?;
    let adapters = pick_adapters(&args.adapters)?;
    let scenarios: Vec<Scenario> = scenario::generate(seed, Split::Development)
        .into_iter()
        .filter(|s| args.families.is_empty() || args.families.contains(&s.family))
        .collect();
    let mut session = Session::open(&args.temp.temp_parent)?;
    let mut rows = Vec::new();
    for scenario in &scenarios {
        for adapter in &adapters {
            rows.push(session.run(scenario, adapter.as_ref())?);
        }
    }
    session.close()?;
    match args.format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&rows).expect("serialize the runs")
        ),
        Format::Text => {
            println!(
                "{:<34} {:<24} {:<9} {:<7} {:<14} {:<22} schedule",
                "scenario", "adapter", "condition", "report", "path", "outcome"
            );
            for row in &rows {
                println!("{}", text_line(row));
            }
        }
    }
    Ok(())
}

fn inspect(seed: u64, scenario_id: &str, adapter: &str, temp: &Temp) -> CliResult {
    let seed = development_seed(seed)?;
    let adapter = adapters::by_name(adapter).map_err(Error::Usage)?;
    let scenario = scenario::generate(seed, Split::Development)
        .into_iter()
        .find(|s| s.id == scenario_id)
        .ok_or_else(|| Error::Usage(format!("seed {seed} has no scenario `{scenario_id}`")))?;
    let mut session = Session::open(&temp.temp_parent)?;
    let row = session.run(&scenario, adapter.as_ref())?;
    session.close()?;

    println!(
        "scenario   {} ({} / {})",
        scenario.id, scenario.family, scenario.variant
    );
    println!("request    {}", scenario.request.display());
    println!("operation  {}", scenario.op);
    println!("schedule   {}", scenario.schedule_label());
    println!("fixture steps:");
    for step in &scenario.setup {
        println!("  {step:?}");
    }
    for mutation in &scenario.schedule {
        println!("at {}: {}", mutation.at.label(), mutation.label);
        for step in &mutation.steps {
            println!("  {step:?}");
        }
    }
    println!(
        "\n{}",
        serde_json::to_string_pretty(&row).expect("serialize the run")
    );
    match row.record.handle_identity {
        Some((device, inode)) => println!(
            "\nlive handle identity (not written to result files): device {device}, inode {inode}"
        ),
        None => println!("\nno handle was returned"),
    }
    Ok(())
}

fn list(what: Listing, seed: u64) -> CliResult {
    match what {
        Listing::Adapters => {
            for adapter in adapters::all() {
                println!(
                    "{:<24} v{}  fallback: {:<20} {}",
                    adapter.name(),
                    adapter.contract_version(),
                    adapter.fallback().as_str(),
                    adapter.strategy()
                );
            }
        }
        Listing::Families => {
            for family in Family::ALL {
                println!(
                    "{:<26} {}",
                    family.as_str(),
                    scenario::variants(*family).join(", ")
                );
            }
        }
        Listing::Outcomes => {
            for outcome in RunOutcome::ALL {
                println!("{outcome}");
            }
        }
        Listing::Metrics => {
            for m in METRICS {
                println!(
                    "{}{}\n    = {} / {}\n    {}\n",
                    m.label,
                    if m.primary { "  [primary]" } else { "" },
                    m.numerator,
                    m.denominator,
                    m.interpretation
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        }
        Listing::Scenarios => {
            let seed = development_seed(seed)?;
            for s in scenario::generate(seed, Split::Development) {
                println!(
                    "{:<34} {:<24} {:<5} {:<40} {}",
                    s.id,
                    s.variant,
                    s.op.as_str(),
                    s.request.display(),
                    s.schedule_label()
                );
            }
        }
    }
    Ok(())
}

fn print_protocol() {
    let range = |seeds: Vec<u64>| {
        format!(
            "{}..={} ({})",
            seeds[0],
            seeds[seeds.len() - 1],
            seeds.len()
        )
    };
    println!(
        "protocol version                {}",
        protocol::PROTOCOL_VERSION
    );
    println!(
        "development seeds               {}",
        range(protocol::development_seeds())
    );
    println!(
        "held-out seeds                  {}",
        range(protocol::held_out_seeds())
    );
    println!("scenario families               {}", Family::ALL.len());
    println!(
        "instances per family            {}",
        protocol::INSTANCES_PER_FAMILY
    );
    println!(
        "held-out depth instances/family {}",
        protocol::HELD_OUT_DEPTH_INSTANCES_PER_FAMILY
    );
    println!(
        "depth, in-distribution          {:?}",
        protocol::DEPTH_IN_DISTRIBUTION
    );
    println!(
        "depth, held-out region          {:?}",
        protocol::DEPTH_HELD_OUT
    );
    println!("retry budget                    {}", protocol::RETRY_BUDGET);
    println!(
        "max transient failures          {}",
        protocol::MAX_TRANSIENT_FAILURES
    );
    println!(
        "path-string adapters            {}",
        protocol::PATH_STRING_ADAPTERS.join(", ")
    );
    println!(
        "descriptor-anchored adapters    {}",
        protocol::DESCRIPTOR_ANCHORED_ADAPTERS.join(", ")
    );
    println!("\nhypothesis: {}", protocol::HYPOTHESIS);
    println!("\nnull: {}", protocol::NULL_HYPOTHESIS);
    println!("\nexact predictions:");
    for (id, statement) in protocol::PREDICTIONS {
        println!("  {id}: {statement}");
    }
}

fn dispatch(cli: Cli) -> CliResult {
    match cli.command {
        Command::Dev { out, temp } => {
            let eval = experiment::evaluate(
                Split::Development,
                &protocol::development_seeds(),
                &temp.temp_parent,
            )?;
            write_outputs(&eval, &out)
        }
        Command::Final { out, temp } => {
            let eval = experiment::evaluate(
                Split::HeldOut,
                &protocol::held_out_seeds(),
                &temp.temp_parent,
            )?;
            write_outputs(&eval, &out)
        }
        Command::Run(args) => run(&args),
        Command::Inspect {
            seed,
            scenario,
            adapter,
            temp,
        } => inspect(seed, &scenario, &adapter, &temp),
        Command::List { what, seed } => list(what, seed),
        Command::Protocol => {
            print_protocol();
            Ok(())
        }
        Command::Probe { temp } => {
            let session = Session::open(&temp.temp_parent)?;
            let supported = session.openat2_supported();
            session.close()?;
            println!(
                "kernel release                    {}",
                sys::kernel_release()
            );
            println!("openat2(RESOLVE_BENEATH) usable   {supported}");
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match dispatch(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
