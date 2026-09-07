//! spatial-bench driver for kiddo v6.
//!
//! The engine owns the harness, so what is measured lives here rather than in
//! kiddo: point generation, seeds, what sits inside the timed region. kiddo
//! contributes its public API and nothing else, and never depends on this crate.
//!
//! kiddo is generic enough that instantiating every combination of scalar,
//! dimensionality, index type, bucket size, stem strategy and leaf strategy is
//! not compilable in reasonable time. So this crate exposes [`bench_case!`], and
//! the engine generates a `main.rs` invoking it once per combination a selection
//! actually needs.

use spatial_bench_core::harness::{Budget, CaseSpec};
use spatial_bench_core::schema::Point;

/// Re-exported for [`bench_case!`]. A macro expands in the generated crate,
/// which depends on this crate and the subject and nothing else, so anything it
/// names must be reachable through `$crate`.
pub use spatial_bench_core::tag::{TagKey, TagValue};
use std::process::ExitCode;

/// Spawn the dataset generator and read construction + query points from
/// its stdout (the dataset generator streams binary points).
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
            eprintln!("spatial-bench-kiddo-v6: {err:?}");
            return ExitCode::from(4);
        }
        return ExitCode::SUCCESS;
    }

    let spec = match spatial_bench_core::harness::read_spec(std::io::stdin().lock()) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("spatial-bench-kiddo-v6: bad spec: {err:?}");
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
                "spatial-bench-kiddo-v6: no monomorphisation for case {}; \
                 the generated driver does not cover it",
                case.id
            );
            return ExitCode::from(3);
        };
        match (reg.run)(case, &spec.budget) {
            Ok(point) => {
                if let Err(err) = spatial_bench_core::harness::write_point(&mut out, &point) {
                    eprintln!("spatial-bench-kiddo-v6: {err:?}");
                    return ExitCode::from(4);
                }
            }
            Err(err) => {
                eprintln!("spatial-bench-kiddo-v6: case {} failed: {err}", case.id);
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

/// Map a tag value to the kiddo type it names.
///
/// This mapping lives here, not in the engine or the manifest: it is knowledge
/// about kiddo's API, and the engine stays generic by emitting tag values
/// verbatim and letting each driver interpret them.
#[macro_export]
#[doc(hidden)]
macro_rules! __kiddo_stem {
    // Eytzinger has no block height; the axis exists for the Donnelly family
    // and is matched and discarded here rather than needing a separate shape.
    (eytzinger, $bh:literal, $axis:ident) => { ::kiddo::Eytzinger };
    (eytzinger_nopf, $bh:literal, $axis:ident) => { ::kiddo::EytzingerNoPf };
    (donnelly, $bh:literal, $axis:ident) => { ::kiddo::Donnelly<$bh> };
    (donnelly_nopf, $bh:literal, $axis:ident) => { ::kiddo::DonnellyNoPf<$bh> };
    (donnelly_unrolled, $bh:literal, $axis:ident) => { ::kiddo::DonnellyUnrolled<$bh> };
    (donnelly_unrolled_block_dim, $bh:literal, $axis:ident) => {
        ::kiddo::DonnellyUnrolledBlockDim<$bh>
    };
    (donnelly_simd_descent, $bh:literal, $axis:ident) => { ::kiddo::DonnellySimdDescent<$bh> };
    (donnelly_simd_full, $bh:literal, $axis:ident) => { ::kiddo::DonnellySimdFull<$bh> };

    // The cyclic SIMD strategies assert BH==3 for f64 and BH==4 for f32,
    // independent of dimensionality. That is a property of the strategy, so it
    // is decided here rather than being something a manifest has to know or a
    // selection can get wrong. The declared block height is ignored, and the
    // emitted point records what was actually compiled -- see __kiddo_bh!.
    (donnelly_cyclic_simd_descent, $bh:literal, f64) => {
        ::kiddo::DonnellyCyclicSimdDescent<3>
    };
    (donnelly_cyclic_simd_descent, $bh:literal, f32) => {
        ::kiddo::DonnellyCyclicSimdDescent<4>
    };
    (donnelly_cyclic_simd_full, $bh:literal, f64) => { ::kiddo::DonnellyCyclicSimdFull<3> };
    (donnelly_cyclic_simd_full, $bh:literal, f32) => { ::kiddo::DonnellyCyclicSimdFull<4> };
}

/// The block height actually compiled, which is not always the one declared.
///
/// A tag that disagreed with the binary would put a wrong number in the dataset
/// with nothing to catch it, so the point records this rather than the manifest
/// value.
#[macro_export]
#[doc(hidden)]
macro_rules! __kiddo_bh {
    (donnelly_cyclic_simd_descent, $bh:literal, f64) => {
        3
    };
    (donnelly_cyclic_simd_descent, $bh:literal, f32) => {
        4
    };
    (donnelly_cyclic_simd_full, $bh:literal, f64) => {
        3
    };
    (donnelly_cyclic_simd_full, $bh:literal, f32) => {
        4
    };
    ($stem:ident, $bh:literal, $axis:ident) => {
        $bh
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __kiddo_leaf {
    (flatvec, $axis:ty, $idx:ty, $dims:literal, $bucket:literal) => {
        ::kiddo::leaf_strategy::FlatVec<$axis, $idx, $dims, $bucket>
    };
    (vec_of_arenas, $axis:ty, $idx:ty, $dims:literal, $bucket:literal) => {
        ::kiddo::VecOfArenas<$axis, $idx, $dims, $bucket>
    };
}

/// Instantiate one monomorphisation of the kiddo harness.
///
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

/// Arguments arrive in the order the engine generates them: alphabetical by tag
/// key, with the subject namespace stripped.
///
/// ```ignore
/// bench_case!(axis = f64, block_height = 3, bucket = 32, dims = 3,
///             idx = u32, leaf = flatvec, stem = eytzinger)
/// ```
#[macro_export]
macro_rules! bench_case {
    (
        axis = $axis:ident, block_height = $bh:literal, bucket = $bucket:literal,
        dims = $dims:literal, idx = $idx:ident, leaf = $leaf:ident, stem = $stem:ident
    ) => {
        $crate::Registration {
            compile_time: &[
                ("axis", stringify!($axis)),
                ("kiddo.block_height", stringify!($bh)),
                ("kiddo.bucket", stringify!($bucket)),
                ("dims", stringify!($dims)),
                ("idx", stringify!($idx)),
                ("kiddo.leaf", stringify!($leaf)),
                ("kiddo.stem", stringify!($stem)),
            ],
            run: |case, budget| {
                type Axis = $axis;
                type Idx = $idx;
                type Leaf = $crate::__kiddo_leaf!($leaf, Axis, Idx, $dims, $bucket);
                type Tree = ::kiddo::kd_tree::KdTree<
                    Axis,
                    Idx,
                    $crate::__kiddo_stem!($stem, $bh, $axis),
                    Leaf,
                    $dims,
                    $bucket,
                >;

                let points =
                    case.int("tree_size")
                        .ok_or("tree_size is missing or not an integer")? as usize;
                let queries = case
                    .int("query_count")
                    .ok_or("query_count is missing or not an integer")?
                    as usize;
                // within_radius cases carry no k (a radius query has no result count);
                // everything that reads k declares it.
                let k = case.int("k").unwrap_or(1) as usize;
                // kiddo takes a NonZero here, so a k of 0 is rejected up front
                // rather than panicking inside the timed region.
                let k_nz = ::std::num::NonZeroUsize::new(k).ok_or("k must be greater than zero")?;
                let query_kind = case.word("query").ok_or("query is missing")?;

                // Day-1.5 dataset work: points come from the generator
                // binary, not from in-driver generation. The spec carries the
                // binary path and the dataset kind.
                // Day-1.5 dataset work: spawn the dataset generator binary
                // and read the binary points from its stdout.
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
                // Parse the header to find the data offsets.
                // Header: SBDS + u32 version + u32 dims + u8 dtype +
                // u64 tree_count (13..21) + u64 query_count (21..29).
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

                let tree: Tree = ::kiddo::kd_tree::KdTree::new_from_slice(&data)
                    .map_err(|e| format!("building the tree failed: {e:?}"))?;

                // The measured operation's shape and execution are identity
                // tags: single-query runs walk the probes one at a time
                // through the query builder; batch-query runs feed the bulk
                // API in query_batch_size chunks under the tagged executor.
                let batching = case
                    .word("query_batching")
                    .unwrap_or_else(|| "single_query".to_owned());
                let parallelism = case
                    .word("parallelism")
                    .unwrap_or_else(|| "single_threaded".to_owned());
                let batch_size = case
                    .int("query_batch_size")
                    .map(|v| v as usize)
                    .unwrap_or(queries)
                    .max(1);
                let executor = match parallelism.as_str() {
                    "single_threaded" => ::kiddo::batch::Executor::serial(),
                    "multi_threaded" => ::kiddo::batch::Executor::parallel(),
                    other => panic!("unknown parallelism `{other}`"),
                };

                let body = || -> u64 {
                    let mut checksum = 0u64;
                    match batching.as_str() {
                        "single_query" => {
                            for probe in &probes {
                                match query_kind.as_str() {
                                    "exact_nn" if k == 1 => {
                                        let hit = tree
                                            .query(::std::hint::black_box(probe))
                                            .nearest_one::<::kiddo::SquaredEuclidean<Axis>>()
                                            .execute();
                                        checksum = checksum.wrapping_add(hit.item as u64);
                                    }
                                    "exact_nn" => {
                                        let hits = tree
                                            .query(::std::hint::black_box(probe))
                                            .nearest_n::<::kiddo::SquaredEuclidean<Axis>>(k_nz)
                                            .execute();
                                        for hit in hits {
                                            checksum = checksum.wrapping_add(hit.item as u64);
                                        }
                                    }
                                    "within_radius" => {
                                        let radius =
                                            $crate::target_results_radius(points as f64) as Axis;
                                        let hits = tree
                                            .query(::std::hint::black_box(probe))
                                            .within::<::kiddo::SquaredEuclidean<Axis>>(radius)
                                            .execute();
                                        for hit in hits {
                                            checksum = checksum.wrapping_add(hit.item as u64);
                                        }
                                    }

                                    "best_n_within" => {
                                        let radius =
                                            $crate::target_results_radius(points as f64) as Axis;
                                        let hits = tree
                                            .query(::std::hint::black_box(probe))
                                            .best_n_within::<::kiddo::SquaredEuclidean<Axis>>(
                                                radius, k_nz,
                                            )
                                            .execute();
                                        for hit in hits {
                                            checksum = checksum.wrapping_add(hit.item as u64);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "batch_query" => {
                            for chunk in probes.chunks(batch_size) {
                                match query_kind.as_str() {
                                    "exact_nn" if k == 1 => {
                                        let hits = tree
                                            .query_batch(chunk)
                                            .with_executor(&executor)
                                            .nearest_one::<::kiddo::SquaredEuclidean<Axis>>()
                                            .execute();
                                        for hit in &hits {
                                            checksum = checksum.wrapping_add(hit.item as u64);
                                        }
                                    }
                                    "exact_nn" => {
                                        let hits = tree
                                            .query_batch(chunk)
                                            .with_executor(&executor)
                                            .nearest_n::<::kiddo::SquaredEuclidean<Axis>>(k_nz)
                                            .execute();
                                        for per_query in &hits {
                                            for hit in per_query {
                                                checksum = checksum.wrapping_add(hit.item as u64);
                                            }
                                        }
                                    }
                                    "within_radius" => {
                                        let radius =
                                            $crate::target_results_radius(points as f64) as Axis;
                                        let hits = tree
                                            .query_batch(chunk)
                                            .with_executor(&executor)
                                            .within::<::kiddo::SquaredEuclidean<Axis>>(radius)
                                            .execute();
                                        for per_query in &hits {
                                            for hit in per_query {
                                                checksum = checksum.wrapping_add(hit.item as u64);
                                            }
                                        }
                                    }
                                    "best_n_within" => {
                                        let radius =
                                            $crate::target_results_radius(points as f64) as Axis;
                                        let hits = tree
                                            .query_batch(chunk)
                                            .with_executor(&executor)
                                            .best_n_within::<::kiddo::SquaredEuclidean<Axis>>(
                                                radius, k_nz,
                                            )
                                            .execute();
                                        for per_query in &hits {
                                            for hit in per_query {
                                                checksum = checksum.wrapping_add(hit.item as u64);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        other => panic!("unknown query_batching `{other}`"),
                    }
                    checksum
                };
                let mut point = $crate::measure(case, budget, queries as u64, body)?;
                // Record the block height the binary was built with, which the
                // strategy may have overridden.
                point.tags.insert(
                    $crate::TagKey::Borrowed("kiddo.block_height"),
                    $crate::TagValue::Int($crate::__kiddo_bh!($stem, $bh, $axis) as i64),
                );
                Ok(point)
            },
        }
    };
}

#[cfg(test)]
mod tests {
    /// The cyclic SIMD strategies assert BH==3 for f64 and BH==4 for f32 at run
    /// time. The driver decides that rather than the manifest, so a selection
    /// cannot produce a combination that panics inside the timed region.
    ///
    /// `__kiddo_bh!` is what the emitted point records, so this also guards
    /// against the tag disagreeing with the binary — a wrong block height in the
    /// dataset would be invisible otherwise.
    #[test]
    fn cyclic_strategies_override_the_declared_block_height() {
        assert_eq!(__kiddo_bh!(donnelly_cyclic_simd_descent, 3, f64), 3);
        assert_eq!(__kiddo_bh!(donnelly_cyclic_simd_descent, 3, f32), 4);
        assert_eq!(__kiddo_bh!(donnelly_cyclic_simd_full, 3, f64), 3);
        assert_eq!(__kiddo_bh!(donnelly_cyclic_simd_full, 3, f32), 4);

        // Even a declaration that would panic is corrected rather than obeyed.
        assert_eq!(__kiddo_bh!(donnelly_cyclic_simd_descent, 7, f32), 4);
    }

    /// Everything else uses what it was given.
    #[test]
    fn other_strategies_keep_the_declared_block_height() {
        assert_eq!(__kiddo_bh!(donnelly_unrolled, 3, f64), 3);
        assert_eq!(__kiddo_bh!(donnelly_unrolled, 5, f64), 5);
        assert_eq!(__kiddo_bh!(eytzinger, 3, f32), 3);
    }
}
