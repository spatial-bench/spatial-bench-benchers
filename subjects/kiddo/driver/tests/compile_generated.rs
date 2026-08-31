//! §13 drift guard 2, as a test: **the source generated for a selection
//! compiles.**
//!
//! `bench_case!` names kiddo's types — every stem strategy, the leaf strategy,
//! the tree's constructors. Nothing else in the engine does. So the failure
//! mode of a subject renaming a type is a generated driver that does not
//! compile, and the sooner that is caught the better: here it is a test result,
//! rather than a cargo error minutes into a real run.
//!
//! This is deliberately the same pipeline a run uses — catalog, selector,
//! `codegen::generate`, `build::materialise` against the manifest's pin — so it
//! cannot drift from what `spatial-bench run` actually builds. The selection
//! covers each *shape* of macro arm, not every stem:
//!
//! - `eytzinger` — the no-block-height arm
//! - `donnelly_simd_full` — the block-height-parameterised arm
//! - `donnelly_cyclic_simd_*` on both scalars — the arms that override the
//!   declared block height (BH 3 for f64, 4 for f32)
//!
//! `cargo clean` in the engine wipes the cache (it lives under `target/`); the
//! first build after that needs the network for kiddo's pinned ref, and every
//! later one reuses cargo's git checkout. Set `SPATIAL_BENCH_SKIP_DRIFT=1` to
//! skip, e.g. when deliberately offline.
//!
//! Not yet done here (§13 check 3, `conform`): building the driver, asking it
//! to list its cases, and asserting set-equality with the manifest. That needs
//! a driver introspection mode the harness contract does not have yet.

use spatial_bench_core::build::{materialise, BuildRequest, DriverSource, SubjectSource};
use spatial_bench_core::codegen::{self, BuildInputs};
use spatial_bench_core::SelectorSet;
use std::path::PathBuf;
use std::process::Command;

/// The arms whose expansion differs. One compile-time axis each; k and
/// tree_size are runtime axes and deliberately absent — the guard is about the
/// macro's type mapping, not the sweep.
const SELECTION: &[&str] = &[
    "impl=kiddo_v6,k=1,axis=f64,kiddo.stem=eytzinger|donnelly_simd_full|donnelly_cyclic_simd_descent",
    "impl=kiddo_v6,k=1,axis=f32,kiddo.stem=donnelly_cyclic_simd_full",
];

#[test]
fn generated_source_for_a_selection_compiles() {
    if std::env::var_os("SPATIAL_BENCH_SKIP_DRIFT").is_some() {
        eprintln!("skipping: SPATIAL_BENCH_SKIP_DRIFT is set");
        return;
    }
    // Day 1.5: the crate lives in the bencher repo beside its manifest.
    // driver -> kiddo -> subjects: the catalog is right there; the engine is
    // the bencher checkout's sibling, or SPATIAL_BENCH_ENGINE_SRC.
    let bencher_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("bencher root");
    let engine_root = match std::env::var_os("SPATIAL_BENCH_ENGINE_SRC") {
        Some(raw) => PathBuf::from(raw)
            .canonicalize()
            .expect("--engine-src path"),
        None => bencher_root
            .parent()
            .expect("bencher root has a parent")
            .join("spatial-bench"),
    };
    assert!(
        engine_root.join("crates/spatial-bench-core").is_dir(),
        "engine checkout not found at {}",
        engine_root.display()
    );
    let catalog = spatial_bench_core::catalog_load::load_dir(&bencher_root.join("subjects"))
        .expect("vendored manifests should load");

    let selection = SelectorSet::parse_all(SELECTION).unwrap();
    let (_, cases) = codegen::by_subject(&catalog, &selection)
        .into_iter()
        .find(|(subject, _)| subject == "kiddo_v6")
        .expect("the selection should reach kiddo_v6");
    assert!(
        cases.len() >= 4,
        "expected several arms, got {}",
        cases.len()
    );

    // A cache lane of its own under the engine's target/: the drift check
    // builds with whatever cargo is default, not the run toolchain, so sharing
    // the real build dirs would only confuse their cache keys.
    let generated = codegen::generate(
        "spatial-bench-kiddo-v6",
        "bench_case",
        "kiddo_v6",
        &cases,
        &BuildInputs {
            toolchain: "drift-check".into(),
            rustflags: catalog.rustflags("kiddo_v6"),
            subject_rev: catalog.pinned_ref("kiddo_v6").unwrap_or_default(),
            driver_rev: format!(
                "path:{};engine:{}",
                env!("CARGO_MANIFEST_DIR"),
                engine_root.display()
            ),
            features: catalog.features("kiddo_v6"),
        },
    );

    let (kind, repo, pin) = catalog
        .source("kiddo_v6")
        .expect("kiddo_v6 declares a source");
    assert_eq!(
        kind, "cargo-git",
        "this guard knows the cargo-git path only"
    );
    let request = BuildRequest {
        subject: "kiddo_v6".into(),
        driver_crate: "spatial-bench-kiddo-v6".into(),
        driver_source: DriverSource::Path(env!("CARGO_MANIFEST_DIR").into()),
        // The generated package patches the driver's version deps on core +
        // measure to this engine checkout (day 1.5's [patch.crates-io]).
        engine_root: Some(engine_root.clone()),
        subject_crate: "kiddo".into(),
        subject_source: SubjectSource::Git {
            repo: repo.expect("cargo-git declares a repo"),
            reference: pin,
        },
        features: catalog.features("kiddo_v6"),
        rustflags: catalog.rustflags("kiddo_v6"),
        generated,
    };
    let dir = materialise(&engine_root.join("target/drift-check"), &request)
        .expect("materialising the generated package");

    let mut cargo = Command::new("cargo");
    cargo.args(["build", "--bin", "driver"]).current_dir(&dir);
    match &request.rustflags {
        Some(flags) => {
            cargo.env("RUSTFLAGS", flags);
        }
        None => {
            cargo.env_remove("RUSTFLAGS");
        }
    }
    let out = cargo.output().expect("could not run cargo");
    assert!(
        out.status.success(),
        "the generated driver failed to compile — a subject renamed something the \
         macro names?\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}
