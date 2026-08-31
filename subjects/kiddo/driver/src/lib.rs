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
}

/// Instantiate one monomorphisation of the kiddo harness.
///
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
                let k = case.int("k").ok_or("k is missing or not an integer")? as usize;
                // kiddo takes a NonZero here, so a k of 0 is rejected up front
                // rather than panicking inside the timed region.
                let k_nz = ::std::num::NonZeroUsize::new(k).ok_or("k must be greater than zero")?;
                let query_kind = case.word("query").ok_or("query is missing")?;

                let data: Vec<[Axis; $dims]> = $crate::generate(points, case.point_seed);
                let probes: Vec<[Axis; $dims]> = $crate::generate(queries, case.query_seed);

                let tree: Tree = ::kiddo::kd_tree::KdTree::new_from_slice(&data)
                    .map_err(|e| format!("building the tree failed: {e:?}"))?;

                let body = || -> u64 {
                    let mut checksum = 0u64;
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
                            _ => {}
                        }
                    }
                    checksum
                };

                if !matches!(query_kind.as_str(), "exact_nn") {
                    return Err(format!("query kind `{query_kind}` is not implemented yet"));
                }
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

/// Deterministic uniform points. Seeds come from the spec, not from here, so two
/// subjects in one run see byte-identical data.
pub fn generate<const D: usize, A>(count: usize, seed: u64) -> Vec<[A; D]>
where
    rand::distr::StandardUniform: rand::distr::Distribution<[A; D]>,
{
    use rand::{RngExt, SeedableRng};
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    (0..count).map(|_| rng.random::<[A; D]>()).collect()
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
