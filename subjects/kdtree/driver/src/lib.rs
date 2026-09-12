//! Harness driver for the `kdtree` crate's mutable bucket point-region tree.

use spatial_bench_core::harness::{Budget, CaseSpec};
use spatial_bench_core::schema::Point;
use std::process::ExitCode;

pub struct Registration {
    pub compile_time: &'static [(&'static str, &'static str)],
    pub run: fn(&CaseSpec, &Budget) -> Result<Point, String>,
}

pub fn harness_main(registrations: &[Registration]) -> ExitCode {
    if std::env::args().any(|arg| arg == "--list") {
        let entries = registrations.iter().map(|registration| {
            registration
                .compile_time
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect()
        });
        return match spatial_bench_core::harness::write_registrations(std::io::stdout(), entries) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("spatial-bench-kdtree: {error:?}");
                ExitCode::from(4)
            }
        };
    }

    let spec = match spatial_bench_core::harness::read_spec(std::io::stdin()) {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("spatial-bench-kdtree: bad spec: {error:?}");
            return ExitCode::from(2);
        }
    };
    let mut stdout = std::io::stdout().lock();
    for case in &spec.cases {
        let Some(registration) = registrations.iter().find(|registration| {
            registration.compile_time.iter().all(|(key, value)| {
                case.tags.get(*key).map(ToString::to_string).as_deref() == Some(*value)
            })
        }) else {
            eprintln!("spatial-bench-kdtree: no registration for {}", case.id);
            return ExitCode::from(3);
        };
        match (registration.run)(case, &spec.budget).and_then(|point| {
            spatial_bench_core::harness::write_point(&mut stdout, &point)
                .map_err(|e| format!("{e:?}"))
        }) {
            Ok(()) => {}
            Err(error) => {
                eprintln!("spatial-bench-kdtree: case {} failed: {error}", case.id);
                return ExitCode::from(5);
            }
        }
    }
    ExitCode::SUCCESS
}

pub use spatial_bench_measure::measure;

type F32Points = (Vec<[f32; 3]>, Vec<[f32; 3]>);
type F64Points = (Vec<[f64; 3]>, Vec<[f64; 3]>);

fn dataset(case: &CaseSpec, axis: &str) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new(&case.dataset_generator)
        .args([
            "--kind",
            &case.dataset,
            "--dims",
            "3",
            "--dtype",
            axis,
            "--tree-count",
            &case
                .int("tree_size")
                .ok_or("tree_size is missing")?
                .to_string(),
            "--query-count",
            &case
                .int("query_count")
                .ok_or("query_count is missing")?
                .to_string(),
            "--seed",
            &case.random_seed.to_string(),
        ])
        .output()
        .map_err(|error| format!("dataset generator: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "dataset generator failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn counts(raw: &[u8]) -> Result<(usize, usize), String> {
    if raw.len() < 29 || &raw[..4] != b"SBDS" {
        return Err("invalid dataset header".to_owned());
    }
    let trees =
        u64::from_ne_bytes(raw[13..21].try_into().map_err(|_| "invalid tree count")?) as usize;
    let queries =
        u64::from_ne_bytes(raw[21..29].try_into().map_err(|_| "invalid query count")?) as usize;
    Ok((trees, queries))
}

pub fn points_f32(case: &CaseSpec) -> Result<F32Points, String> {
    let raw = dataset(case, "f32")?;
    let (trees, queries) = counts(&raw)?;
    let (chunks, remainder) = raw[29..].as_chunks::<4>();
    if !remainder.is_empty() {
        return Err("dataset body ends partway through an f32".to_owned());
    }
    let values: Result<Vec<f32>, String> = chunks
        .iter()
        .map(|bytes| Ok(f32::from_ne_bytes(*bytes)))
        .collect();
    let values = values?;
    if values.len() != (trees + queries) * 3 {
        return Err("dataset body length disagrees with header".to_owned());
    }
    let points = values[..trees * 3]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    let probes = values[trees * 3..]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    Ok((points, probes))
}

pub fn points_f64(case: &CaseSpec) -> Result<F64Points, String> {
    let raw = dataset(case, "f64")?;
    let (trees, queries) = counts(&raw)?;
    let (chunks, remainder) = raw[29..].as_chunks::<8>();
    if !remainder.is_empty() {
        return Err("dataset body ends partway through an f64".to_owned());
    }
    let values: Result<Vec<f64>, String> = chunks
        .iter()
        .map(|bytes| Ok(f64::from_ne_bytes(*bytes)))
        .collect();
    let values = values?;
    if values.len() != (trees + queries) * 3 {
        return Err("dataset body length disagrees with header".to_owned());
    }
    let points = values[..trees * 3]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    let probes = values[trees * 3..]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    Ok((points, probes))
}

#[macro_export]
macro_rules! bench_case {
    (axis = f32) => {
        $crate::Registration {
            compile_time: &[("axis", "f32")],
            run: |case, budget| {
                let (points, probes) = $crate::points_f32(case)?;
                let k = case.int("k").ok_or("k is missing")? as usize;
                let mut tree = ::kdtree::KdTree::with_capacity(3, 32);
                for (index, point) in points.into_iter().enumerate() {
                    tree.add(point, index as u32)
                        .map_err(|e| format!("adding point: {e:?}"))?;
                }
                $crate::measure(case, budget, probes.len() as u64, || {
                    probes.iter().fold(0_u64, |sum, probe| {
                        let hits = tree
                            .nearest(probe, k, &::kdtree::distance::squared_euclidean)
                            .expect("query");
                        sum.wrapping_add(*hits.last().expect("k is positive").1 as u64)
                    })
                })
            },
        }
    };
    (axis = f64) => {
        $crate::Registration {
            compile_time: &[("axis", "f64")],
            run: |case, budget| {
                let (points, probes) = $crate::points_f64(case)?;
                let k = case.int("k").ok_or("k is missing")? as usize;
                let mut tree = ::kdtree::KdTree::with_capacity(3, 32);
                for (index, point) in points.into_iter().enumerate() {
                    tree.add(point, index as u32)
                        .map_err(|e| format!("adding point: {e:?}"))?;
                }
                $crate::measure(case, budget, probes.len() as u64, || {
                    probes.iter().fold(0_u64, |sum, probe| {
                        let hits = tree
                            .nearest(probe, k, &::kdtree::distance::squared_euclidean)
                            .expect("query");
                        sum.wrapping_add(*hits.last().expect("k is positive").1 as u64)
                    })
                })
            },
        }
    };
}
