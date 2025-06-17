mod bench;
mod utils;

use bench::Count;
use std::sync::Arc;
use clap::Parser;
use quanta::Clock;
use crate::bench::run_bench;
use std::fs::OpenOptions;
use std::io::Write;

const DEFAULT_NUM_SAMPLES: Count = 300;
const DEFAULT_NUM_ITERATIONS_PER_SAMPLE: Count = 1000;

#[derive(Clone)]
#[derive(clap::Parser)]
pub struct CliArgs {
    /// The number of iterations per sample
    #[clap(default_value_t = DEFAULT_NUM_ITERATIONS_PER_SAMPLE, value_parser)]
    num_iterations: Count,

    /// The number of samples
    #[clap(default_value_t = DEFAULT_NUM_SAMPLES, value_parser)]
    num_samples: Count,

    /// Outputs the mean latencies in CSV format on stdout
    #[clap(long, value_parser)]
    csv: bool,

    /// Select which benchmark to run, in a comma delimited list, e.g., '1,3' {n}
    /// 1: CAS latency on a single shared cache line. {n}
    /// 2: Single-writer single-reader latency on two shared cache lines. {n}
    /// 3: One writer and one reader on many cache line, using the clock. {n}
    #[clap(short, long, default_value="1", require_delimiter=true, value_delimiter=',', value_parser)]
    bench: Vec<usize>,

    /// Specify the cores by id that should be used, comma delimited. By default all cores are used.
    #[clap(short, long, require_delimiter=true, value_delimiter=',', value_parser)]
    cores: Vec<usize>,
}

fn main() {
    let args = CliArgs::parse();

    let cores = core_affinity::get_core_ids().expect("get_core_ids() failed");

    let cores = if !args.cores.is_empty() {
        args.cores.iter().copied()
            .map(|cid| *cores.iter().find(|c| c.id == cid)
                .unwrap_or_else(||panic!("Core {} not found. Available: {:?}", cid, &cores)))
            .collect()
    } else {
        cores
    };

    utils::show_cpuid_info();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("output.txt")
        .expect("Cannot open file");

    writeln!(file, "Num cores: {}", cores.len()).unwrap();
    writeln!(file, "Num iterations per samples: {}", args.num_iterations).unwrap();
    writeln!(file, "Num samples: {}", args.num_samples).unwrap();
    #[cfg(target_os = "macos")]
    writeln!(file, "WARN macOS may ignore thread-CPU affinity (we can't select a CPU to run on). Results may be inaccurate").unwrap();

    let clock = Arc::new(Clock::new());

    for b in &args.bench {
        match b {
            1 => {
                writeln!(file).unwrap();
                writeln!(file, "1) CAS latency on a single shared cache line").unwrap();
                writeln!(file).unwrap();
                run_bench(&cores, &clock, &args, bench::cas::Bench::new(), &mut file);
            }
            2 => {
                writeln!(file).unwrap();
                writeln!(file, "2) Single-writer single-reader latency on two shared cache lines").unwrap();
                writeln!(file).unwrap();
                run_bench(&cores, &clock, &args, bench::read_write::Bench::new(), &mut file);
            }
            3 => {
                utils::assert_rdtsc_usable(&clock);
                writeln!(file).unwrap();
                writeln!(file, "3) Message passing. One writer and one reader on many cache line").unwrap();
                writeln!(file).unwrap();
                run_bench(&cores, &clock, &args, bench::msg_passing::Bench::new(args.num_iterations), &mut file);
            }
            _ => panic!("--bench should be 1, 2 or 3"),
        }
    }
}