//! Harness driver for kd-tree's immutable ordered-float tree.

use spatial_bench_core::harness::{Budget, CaseSpec};
use spatial_bench_core::schema::Point;
use std::process::ExitCode;

pub struct Registration {
    pub run: fn(&CaseSpec, &Budget) -> Result<Point, String>,
}

pub fn harness_main(registrations: &[Registration]) -> ExitCode {
    if std::env::args().any(|arg| arg == "--list") {
        return match spatial_bench_core::harness::write_registrations(
            std::io::stdout(),
            registrations.iter().map(|_| Vec::new()),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("spatial-bench-kd-tree: {error:?}");
                ExitCode::from(4)
            }
        };
    }
    let spec = match spatial_bench_core::harness::read_spec(std::io::stdin()) {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("spatial-bench-kd-tree: bad spec: {error:?}");
            return ExitCode::from(2);
        }
    };
    let mut out = std::io::stdout().lock();
    for case in &spec.cases {
        if registrations.len() != 1 {
            eprintln!("spatial-bench-kd-tree: expected one registration");
            return ExitCode::from(3);
        }
        match (registrations[0].run)(case, &spec.budget).and_then(|point| {
            spatial_bench_core::harness::write_point(&mut out, &point).map_err(|e| format!("{e:?}"))
        }) {
            Ok(()) => {}
            Err(error) => {
                eprintln!("spatial-bench-kd-tree: case {} failed: {error}", case.id);
                return ExitCode::from(5);
            }
        }
    }
    ExitCode::SUCCESS
}

pub use spatial_bench_measure::measure;

type Points = (Vec<[f64; 3]>, Vec<[f64; 3]>);

pub fn points(case: &CaseSpec) -> Result<Points, String> {
    let tree_size = case.int("tree_size").ok_or("tree_size is missing")?;
    let query_count = case.int("query_count").ok_or("query_count is missing")?;
    let output = std::process::Command::new(&case.dataset_generator)
        .args([
            "--kind",
            &case.dataset,
            "--dims",
            "3",
            "--dtype",
            "f64",
            "--tree-count",
            &tree_size.to_string(),
            "--query-count",
            &query_count.to_string(),
            "--seed",
            &case.random_seed.to_string(),
        ])
        .output()
        .map_err(|error| format!("dataset generator: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let raw = output.stdout;
    if raw.len() < 29 || &raw[..4] != b"SBDS" {
        return Err("invalid dataset header".to_owned());
    }
    let points =
        u64::from_ne_bytes(raw[13..21].try_into().map_err(|_| "invalid tree count")?) as usize;
    let queries =
        u64::from_ne_bytes(raw[21..29].try_into().map_err(|_| "invalid query count")?) as usize;
    let (words, remainder) = raw[29..].as_chunks::<8>();
    if !remainder.is_empty() || words.len() != (points + queries) * 3 {
        return Err("dataset body disagrees with header".to_owned());
    }
    let values: Vec<f64> = words.iter().map(|word| f64::from_ne_bytes(*word)).collect();
    let tree = values[..points * 3].as_chunks::<3>().0.to_vec();
    let probes = values[points * 3..].as_chunks::<3>().0.to_vec();
    Ok((tree, probes))
}

#[macro_export]
macro_rules! bench_case {
    () => {
        $crate::Registration {
            run: |case, budget| {
                let (points, probes) = $crate::points(case)?;
                let k = case.int("k").ok_or("k is missing")? as usize;
                let tree = ::kd_tree::KdTree::build_by_ordered_float(points);
                $crate::measure(case, budget, probes.len() as u64, || {
                    probes.iter().fold(0_u64, |sum, probe| {
                        let hits = tree.nearests(probe, k);
                        sum.wrapping_add(hits.last().expect("positive k").item[0].to_bits())
                    })
                })
            },
        }
    };
}
