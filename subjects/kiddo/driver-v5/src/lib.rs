//! spatial-bench driver for kiddo 5.x.
//!
//! kiddo 6 rebuilt its query surface around the builder and batch executors;
//! 5.x is the direct-method API: `nearest_one`, `nearest_n`, `within`,
//! `best_n_within` and `nearest_n_within` are methods on the tree, there is no
//! query builder, no batch API and no executor. This driver speaks that API
//! and only that API — the manifest scopes it to `>=5.0.0, <6.0.0`, and a 6.x
//! pin selects the v6 driver instead.
//!
//! The engine owns the harness, so what is measured lives here rather than in
//! kiddo: point generation, seeds, what sits inside the timed region. kiddo
//! contributes its public API and nothing else, and never depends on this
//! crate.
//!
//! kiddo 5's immutable tree monomorphises on scalar, index type,
//! dimensionality and bucket size. This crate exposes [`bench_case!`], and the
//! engine generates a `main.rs` invoking it once per combination a selection
//! actually needs.

use spatial_bench_core::harness::{Budget, CaseSpec};
use spatial_bench_core::schema::Point;

/// Re-exported for [`bench_case!`]. A macro expands in the generated crate,
/// which depends on this crate and the subject and nothing else, so anything
/// it names must be reachable through `$crate`.
pub use spatial_bench_core::tag::{TagKey, TagValue};
use std::process::ExitCode;

/// One monomorphisation, produced by [`bench_case!`].
pub struct Registration {
    /// The compile-time coordinate this entry serves, as tag key/value pairs.
    /// The harness matches a spec to an entry on these.
    pub compile_time: &'static [(&'static str, &'static str)],
    pub run: fn(&CaseSpec, &Budget) -> Result<Point, String>,
}

/// Entry point for a generated driver.
pub fn harness_main(registrations: &[Registration]) -> ExitCode {
    // §13: conform asks the binary what it contains before measuring anything.
    // The listing mode is argv-gated and reads no stdin, so it works wherever
    // the driver works — including inside `perf stat` or any wrapper that does
    // not feed a spec.
    if std::env::args().any(|arg| arg == "--list") {
        let stdout = std::io::stdout();
        // Canonical order: the registration's pairs arrive in macro-argument
        // order, but the listing's contract is sorted-by-key, so conform can
        // compare content rather than an accident of generation.
        let entries = registrations.iter().map(|reg| {
            let mut pairs: Vec<(String, String)> = reg
                .compile_time
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            pairs.sort();
            pairs
        });
        if let Err(err) = spatial_bench_core::harness::write_registrations(stdout.lock(), entries) {
            eprintln!("spatial-bench-kiddo-v5: {err:?}");
            return ExitCode::from(4);
        }
        return ExitCode::SUCCESS;
    }

    let spec = match spatial_bench_core::harness::read_spec(std::io::stdin().lock()) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("spatial-bench-kiddo-v5: bad spec: {err:?}");
            return ExitCode::from(2);
        }
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for case in &spec.cases {
        let Some(reg) = find(registrations, case) else {
            // Refused rather than skipped: a silently missing point is a hole in
            // a comparison that nothing downstream would report.
            eprintln!(
                "spatial-bench-kiddo-v5: no monomorphisation for case {}; \
                 the generated driver does not cover it",
                case.id
            );
            return ExitCode::from(3);
        };
        match (reg.run)(case, &spec.budget) {
            Ok(point) => {
                if let Err(err) = spatial_bench_core::harness::write_point(&mut out, &point) {
                    eprintln!("spatial-bench-kiddo-v5: {err:?}");
                    return ExitCode::from(4);
                }
            }
            Err(err) => {
                eprintln!("spatial-bench-kiddo-v5: case {} failed: {err}", case.id);
                return ExitCode::from(5);
            }
        }
    }
    ExitCode::SUCCESS
}

fn find<'a>(registrations: &'a [Registration], case: &CaseSpec) -> Option<&'a Registration> {
    registrations.iter().find(|reg| {
        reg.compile_time.iter().all(|(key, value)| {
            case.tags.get(*key).map(ToString::to_string).as_deref() == Some(*value)
        })
    })
}

/// Time `body` to the budget and turn the samples into a point.
///
/// Lives in the measure crate, not here, so every Rust subject's driver
/// measures by the same routine — the same warm-up, the same outlier
/// filtering, the same confidence intervals. This crate contributes the timed
/// region only: what sits inside it and the query set it walks are kiddo
/// knowledge, which is the one thing an engine must not decide for a subject.
pub use spatial_bench_measure::measure;

/// Compute a search radius that yields approximately `target` results for
/// `count` uniformly distributed points in the unit D-cube. The formula for
/// the expected count in a D-ball of radius r is: count × V_D(r) where
/// V_D(r) = π^(D/2) r^D / Γ(D/2 + 1). Solving for r given a target count
/// keeps the result count roughly constant across tree sizes, making the
/// comparison fair.
pub fn target_results_radius(count: f64) -> f64 {
    const TARGET: f64 = 100.0;
    // V_3(r) = (4/3)π r³ for 3D
    ((3.0 * TARGET) / (4.0 * std::f64::consts::PI * count)).cbrt()
}

/// Instantiate one monomorphisation of the kiddo 5.x harness.
///
/// Arguments arrive in the order the engine generates them: alphabetical by
/// tag key, with the subject namespace stripped.
///
/// ```ignore
/// bench_case!(axis = f64, dims = 3, idx = u64)
/// ```
///
/// kiddo 5's immutable tree has no stem, leaf or storage choices: the default
/// alias's structure is the structure. There is no batch API and no executor,
/// so every case is single-query and single-threaded by construction — the
/// manifest does not declare those axes for the v5 driver, and the driver
/// refuses them if a selection ever produces one.
#[macro_export]
macro_rules! bench_case {
    (axis = $axis:ident, dims = $dims:literal, idx = $idx:ident) => {
        $crate::Registration {
            compile_time: &[
                ("axis", stringify!($axis)),
                ("dims", stringify!($dims)),
                ("idx", stringify!($idx)),
            ],
            run: |case, budget| {
                type Axis = $axis;
                type Idx = $idx;

                let points =
                    case.int("tree_size")
                        .ok_or("tree_size is missing or not an integer")? as usize;
                let queries = case
                    .int("query_count")
                    .ok_or("query_count is missing or not an integer")?
                    as usize;
                // within_radius cases carry no k (a radius query has no result
                // count); everything that reads k declares it.
                let k = case.int("k").unwrap_or(1) as usize;
                // kiddo 5 takes a NonZero here, so a k of 0 is rejected up
                // front rather than panicking inside the timed region.
                let k_nz =
                    ::std::num::NonZeroUsize::new(k).ok_or("k must be greater than zero")?;
                let query_kind = case.word("query").ok_or("query is missing")?;
                let batching = case
                    .word("query_batching")
                    .unwrap_or_else(|| "single_query".to_owned());
                if batching != "single_query" {
                    return Err(format!(
                        "kiddo 5.x has no batch API; query_batching must be \\
                         single_query, got `{batching}`"
                    ));
                }

                // Harness v2: points come from the engine's dataset generator
                // binary. Header: SBDS + u32 version + u32 dims + u8 dtype +
                // u64 tree_count (13..21) + u64 query_count (21..29); body
                // from byte 29 is tree points then query points.
                let gen_output = std::process::Command::new(&case.dataset_generator)
                    .args([
                        "--kind",
                        &case.dataset,
                        "--dims",
                        &$dims.to_string(),
                        "--dtype",
                        stringify!($axis),
                        "--tree-count",
                        &points.to_string(),
                        "--query-count",
                        &queries.to_string(),
                        "--seed",
                        &case.random_seed.to_string(),
                    ])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::inherit())
                    .output()
                    .unwrap_or_else(|e| panic!("dataset generator: {e}"));
                if !gen_output.status.success() {
                    panic!(
                        "dataset generator failed:\\n{}",
                        String::from_utf8_lossy(&gen_output.stderr)
                    );
                }
                let hdr = &gen_output.stdout[..29];
                let tree_count = u64::from_ne_bytes(hdr[13..21].try_into().unwrap()) as usize;
                let query_count = u64::from_ne_bytes(hdr[21..29].try_into().unwrap()) as usize;
                let elem_size = std::mem::size_of::<Axis>();
                let data_start = 29;
                let data_end = data_start + tree_count * $dims * elem_size;
                let query_start = data_end;
                let query_end = query_start + queries * $dims * elem_size;
                let data: Vec<[Axis; $dims]> = gen_output.stdout[data_start..data_end]
                    .chunks_exact(elem_size * $dims)
                    .map(|c| {
                        let mut arr = [0 as Axis; $dims];
                        for (d, chunk) in c.chunks_exact(elem_size).enumerate() {
                            arr[d] = unsafe {
                                let aligned = chunk.as_ptr() as *const Axis;
                                std::ptr::read(aligned)
                            };
                        }
                        arr
                    })
                    .collect();
                let probes: Vec<[Axis; $dims]> = gen_output.stdout[query_start..query_end]
                    .chunks_exact(elem_size * $dims)
                    .map(|c| {
                        let mut arr = [0 as Axis; $dims];
                        for (d, chunk) in c.chunks_exact(elem_size).enumerate() {
                            arr[d] = unsafe {
                                let aligned = chunk.as_ptr() as *const Axis;
                                std::ptr::read(aligned)
                            };
                        }
                        arr
                    })
                    .collect();
                let _ = (tree_count, query_count);

                // kiddo 5's immutable tree, at the manifest's declared index
                // type and the default bucket size.
                let tree = ::kiddo::immutable::float::kdtree::ImmutableKdTree::<
                    Axis,
                    Idx,
                    $dims,
                    32,
                >::new_from_slice(&data);

                let body = || -> u64 {
                    let mut checksum = 0u64;
                    for probe in &probes {
                        match query_kind.as_str() {
                            "exact_nn" if k == 1 => {
                                let hit =
                                    tree.nearest_one::<::kiddo::SquaredEuclidean>(probe);
                                checksum = checksum.wrapping_add(hit.item as u64);
                            }
                            "exact_nn" => {
                                let hits =
                                    tree.nearest_n::<::kiddo::SquaredEuclidean>(probe, k_nz);
                                for hit in hits {
                                    checksum = checksum.wrapping_add(hit.item as u64);
                                }
                            }
                            "within_radius" => {
                                let radius =
                                    $crate::target_results_radius(points as f64) as Axis;
                                let hits = tree.within::<::kiddo::SquaredEuclidean>(probe, radius);
                                for hit in hits {
                                    checksum = checksum.wrapping_add(hit.item as u64);
                                }
                            }
                            "best_n_within" => {
                                let radius =
                                    $crate::target_results_radius(points as f64) as Axis;
                                let hits = tree.best_n_within::<::kiddo::SquaredEuclidean>(
                                    probe, radius, k_nz,
                                );
                                for hit in hits {
                                    checksum = checksum.wrapping_add(hit.item as u64);
                                }
                            }
                            // kiddo 5 has this single-point where kiddo 6 only
                            // offers it batched: the one query kind the two
                            // APIs cover from opposite sides.
                            "nearest_n_within" => {
                                let radius =
                                    $crate::target_results_radius(points as f64) as Axis;
                                let hits = tree.nearest_n_within::<::kiddo::SquaredEuclidean>(
                                    probe, radius, k_nz, true,
                                );
                                for hit in hits {
                                    checksum = checksum.wrapping_add(hit.item as u64);
                                }
                            }
                            _ => {}
                        }
                    }
                    checksum
                };
                let mut point = $crate::measure(case, budget, queries as u64, body)?;
                Ok(point)
            },
        }
    };
}
