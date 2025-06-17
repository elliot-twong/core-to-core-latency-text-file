pub mod cas;
pub mod read_write;
pub mod msg_passing;

use core_affinity::CoreId;
use quanta::Clock;
use std::io::{self, Write};
use ndarray::{s, Axis};
use ordered_float::NotNan;
use crate::CliArgs;

pub type Count = u32;

pub trait Bench {
    fn run(&self, cores: (CoreId, CoreId), clock: &Clock, num_iterations: Count, num_samples: Count) -> Vec<f64>;
    /// Whether the bench on (i,j) is the same as the bench on (j,i)
    fn is_symmetric(&self) -> bool { true }
}

pub fn run_bench(cores: &[CoreId], clock: &Clock, args: &CliArgs, bench: impl Bench, file: &mut std::fs::File) {
    let num_samples = args.num_samples;
    let num_iterations = args.num_iterations;

    let n_cores = cores.len();
    assert!(n_cores >= 2);
    let shape = ndarray::Ix3(n_cores, n_cores, num_samples as usize);
    let mut results = ndarray::Array::from_elem(shape, f64::NAN);

    // First print the column header
    write!(file, "    {: >3}", "").unwrap();
    for j in cores {
        write!(file, " {: >4}{: >3}", j.id, "").unwrap();
        //        |||
        //        ||+-- Width
        //        |+--- Align
        //        +---- Fill
    }
    writeln!(file).unwrap();

    // Do the benchmark
    for i in 0..n_cores {
        let core_i = cores[i];
        write!(file, "    {: >3}", core_i.id).unwrap();
        for j in 0..n_cores {
            if bench.is_symmetric() {
                if i <= j {
                   continue;
                }
            } else if i == j {
                write!(file, "{: >8}", "").unwrap();
                continue;
            }

            let core_j = cores[j];
            // We add 1 warmup cycle first
            let durations = bench.run((core_i, core_j), clock, num_iterations, 1+num_samples);
            let durations = &durations[1..];
            let mut values = results.slice_mut(s![i,j,..]);
            for s in 0..num_samples as usize {
                values[s] = durations[s]
            }

            let mean = format!("{: >4.0}", values.mean().unwrap());
            // We apply the central limit theorem to estimate the standard deviation
            let stddev = format!("±{: <2.0}", values.std(1.0).min(99.0) / (num_samples as f64).sqrt());
            write!(file, " {}{}", mean, stddev).unwrap();
            let _ = io::stdout().lock().flush();
        }
        writeln!(file).unwrap();
    }

    writeln!(file).unwrap();

    // Print min/max latency
    {
        let mean = results.mean_axis(Axis(2)).unwrap();
        let stddev = results.std_axis(Axis(2), 1.0) / (num_samples as f64).sqrt();

        let ((min_i, min_j), _) = mean.indexed_iter()
            .filter_map(|(i, v)| NotNan::new(*v).ok().map(|v| (i, v)))
            .min_by_key(|(_, v)| *v)
            .unwrap();
        let min_mean = format!("{:.1}", mean[(min_i, min_j)]);
        let min_stddev = format!("±{:.1}", stddev[(min_i, min_j)]);
        let (min_core_id_i, min_core_id_j) = (cores[min_i].id, cores[min_j].id);

        let ((max_i, max_j), _) = mean.indexed_iter()
            .filter_map(|(i, v)| NotNan::new(*v).ok().map(|v| (i, v)))
            .max_by_key(|(_, v)| *v)
            .unwrap();
        let max_mean = format!("{:.1}", mean[(max_i, max_j)]);
        let max_stddev = format!("±{:.1}", stddev[(max_i, max_j)]);
        let (max_core_id_i, max_core_id_j) = (cores[max_i].id, cores[max_j].id);

        writeln!(file, "    Min  latency: {}ns {} cores: ({},{})", min_mean, min_stddev, min_core_id_i, min_core_id_j).unwrap();
        writeln!(file, "    Max  latency: {}ns {} cores: ({},{})", max_mean, max_stddev, max_core_id_i, max_core_id_j).unwrap();
    }

    // Print mean latency
    {
        let values = results.iter().copied().filter(|v| !v.is_nan()).collect::<Vec<_>>();
        let values = ndarray::arr1(&values);
        let mean = format!("{:.1}", values.mean().unwrap());
        // no stddev, it's hard to put a value that is meaningful without a lengthy explanation
        writeln!(file, "    Mean latency: {}ns", mean).unwrap();
    }

    if args.csv {
        let results = results.mean_axis(Axis(2)).unwrap();
        for row in results.rows() {
            let row = row.iter()
                .map(|v| if v.is_nan() { "".to_string() } else { v.to_string() })
                .collect::<Vec<_>>().join(",");
            writeln!(file, "{}", row).unwrap();
        }
    }

    writeln!(file).unwrap();
    writeln!(file).unwrap();
    writeln!(file).unwrap();
    writeln!(file).unwrap();
}