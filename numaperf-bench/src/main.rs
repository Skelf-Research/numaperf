//! numaperf CLI tool for benchmarking and system information.
//!
//! Subcommands:
//! - `bench`: Run NUMA locality and performance benchmarks
//! - `info`: Display system topology and capabilities

use clap::{Args, Parser, Subcommand, ValueEnum};
use numaperf::{Capabilities, Topology};
use std::sync::Arc;

mod info;
mod output;
mod runner;

use output::{JsonOutput, SystemInfo};
use runner::{
    run_affinity_benchmarks, run_memory_benchmarks, run_scheduler_benchmarks,
    run_sharded_benchmarks,
};

#[derive(Parser)]
#[command(name = "numaperf-bench")]
#[command(about = "NUMA-first runtime tools: benchmarking and system information")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run NUMA locality and performance benchmarks
    Bench(BenchArgs),
    /// Display system topology, capabilities, and current state
    Info(InfoArgs),
}

// =============================================================================
// Bench Command
// =============================================================================

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BenchCategory {
    All,
    Sharded,
    Memory,
    Scheduler,
    Affinity,
}

#[derive(Args, Debug)]
struct BenchArgs {
    /// Benchmark category to run
    #[arg(short, long, value_enum, default_value = "all")]
    category: BenchCategory,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Number of iterations for quick benchmarks
    #[arg(short, long, default_value = "1000")]
    iterations: u64,

    /// Number of threads for multi-threaded benchmarks
    #[arg(short, long)]
    threads: Option<usize>,
}

// =============================================================================
// Info Command
// =============================================================================

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InfoSection {
    /// Show all information
    All,
    /// NUMA topology (nodes, CPUs, memory)
    Topology,
    /// System capabilities for hard mode
    Capabilities,
    /// Current thread CPU affinity
    Affinity,
    /// NUMA node distance matrix
    Distances,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Section to display
    #[arg(value_enum, default_value = "all")]
    pub section: InfoSection,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Verbose output with recommendations
    #[arg(short, long)]
    pub verbose: bool,
}

// =============================================================================
// Shared Types
// =============================================================================

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Bench(args)) => run_bench(args),
        Some(Commands::Info(args)) => info::run(args),
        None => {
            // Default to showing info when no subcommand is given
            info::run(InfoArgs {
                section: InfoSection::All,
                format: OutputFormat::Text,
                verbose: false,
            })
        }
    }
}

fn run_bench(args: BenchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let topo = Arc::new(Topology::discover()?);
    let caps = Capabilities::detect();

    let threads = args.threads.unwrap_or_else(|| {
        topo.numa_nodes()
            .iter()
            .map(|n| n.cpu_count())
            .sum::<usize>()
            .max(1)
    });

    match args.format {
        OutputFormat::Text => {
            println!("=== numaperf Benchmark Suite ===\n");
            println!("System: {} NUMA nodes, {} CPUs", caps.numa_node_count, threads);
            println!("Hard mode supported: {}", caps.supports_hard_mode());
            println!();
        }
        OutputFormat::Json => {}
    }

    let mut results = Vec::new();

    match args.category {
        BenchCategory::All => {
            results.extend(run_sharded_benchmarks(&topo, args.iterations, threads)?);
            results.extend(run_memory_benchmarks(&topo, args.iterations)?);
            results.extend(run_scheduler_benchmarks(&topo, args.iterations, threads)?);
            results.extend(run_affinity_benchmarks(&topo, args.iterations)?);
        }
        BenchCategory::Sharded => {
            results.extend(run_sharded_benchmarks(&topo, args.iterations, threads)?);
        }
        BenchCategory::Memory => {
            results.extend(run_memory_benchmarks(&topo, args.iterations)?);
        }
        BenchCategory::Scheduler => {
            results.extend(run_scheduler_benchmarks(&topo, args.iterations, threads)?);
        }
        BenchCategory::Affinity => {
            results.extend(run_affinity_benchmarks(&topo, args.iterations)?);
        }
    }

    // Calculate overall locality
    let locality_ratios: Vec<f64> = results.iter().filter_map(|r| r.locality_ratio).collect();

    let overall_locality = if locality_ratios.is_empty() {
        None
    } else {
        Some(locality_ratios.iter().sum::<f64>() / locality_ratios.len() as f64)
    };

    match args.format {
        OutputFormat::Text => {
            output::print_text_results(&results, overall_locality);
        }
        OutputFormat::Json => {
            let json_output = JsonOutput {
                system: SystemInfo {
                    numa_nodes: caps.numa_node_count,
                    cpus: threads,
                    hard_mode_supported: caps.supports_hard_mode(),
                },
                benchmarks: results
                    .iter()
                    .map(|r| output::BenchmarkJson {
                        name: r.name.clone(),
                        ops_per_sec: r.ops_per_sec(),
                        duration_ns: r.duration.as_nanos() as u64,
                        operations: r.operations,
                        locality_ratio: r.locality_ratio,
                    })
                    .collect(),
                locality_health: overall_locality.map(|r| output::LocalityHealthJson {
                    ratio: r,
                    status: numaperf_bench::LocalityHealth::from_ratio(r)
                        .description()
                        .to_lowercase(),
                }),
            };
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
    }

    Ok(())
}
