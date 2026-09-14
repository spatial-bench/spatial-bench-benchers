# Contributing library coverage

Contributions can add a library, adapt a new release, expose another query or
correct an existing measurement. Explain the API semantics and experiment your
change implements. Library expertise helps us choose representative calls;
independent reviewers check their mapping to the workload and measurement contract.

- [Add a library](docs/adding-a-library.md): start from a working adapter.
- [Update an entry](docs/updating-a-library.md): pins, API changes and new cases.
- [Driver contract](docs/driver-contract.md): input, output, timing and checks.

## Set up the engine and catalog

Use Linux for the validated benchmarking path. Install Git, a C/C++ build
toolchain and Rust through rustup. Platform packages are listed in the
[core setup guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/development.md).
Python drivers need Python 3 with `venv` and `pip`; nanoflann needs a C++20
compiler with the library facilities used by its shim, including `std::format`.

From a parent directory, clone this layout (or use existing matching checkouts):

```sh
git clone https://github.com/spatial-bench/spatial-bench-core.git spatial-bench
git clone https://github.com/spatial-bench/spatial-bench-benchers.git
cd spatial-bench
cargo build --release -p spatial-bench -p spatial-bench-dataset
export PATH="$PWD/target/release:$PATH"
export SPATIAL_BENCH_BENCHERS="$(cd ../spatial-bench-benchers && pwd)"
export SPATIAL_BENCH_SUBJECTS="$SPATIAL_BENCH_BENCHERS/subjects"
export SPATIAL_BENCH_ENGINE_SRC="$PWD"
cd ../spatial-bench-benchers
```

The workspace patches also expect the engine at `../spatial-bench`. With another
layout, override the Cargo patches as well as pointing the CLI at the catalog.
For example, from the bencher root with `SPATIAL_BENCH_ENGINE_SRC` set to an
absolute core path:

```sh
cargo --config "patch.crates-io.spatial-bench-core.path=\"$SPATIAL_BENCH_ENGINE_SRC/crates/spatial-bench-core\"" \
  --config "patch.crates-io.spatial-bench-measure.path=\"$SPATIAL_BENCH_ENGINE_SRC/crates/spatial-bench-measure\"" \
  test -p spatial-bench-kdtree
```

`SPATIAL_BENCH_SUBJECTS` names the `subjects` directory; the core tests' separate
`SPATIAL_BENCH_BENCHERS` variable names the bencher repository root.

The engine chooses a Rust version from the selected subjects' declared minimum
versions. Install the resolved toolchain if missing; the kdtree example uses
`rustup toolchain install 1.89.0`. Alternatively pass an installed numeric version
such as `--rustc 1.90.0` to `run`, respecting each selected subject's floor.

## Validate a small selection

Run from this repository after the setup above:

```sh
spatial-bench subjects
spatial-bench list --select 'impl=kdtree,query=exact_nn,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench run --dry-run \
  --select 'impl=kdtree,query=exact_nn,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench conform --subject kdtree
spatial-bench run --random-seed 42 --allow-unfingerprinted \
  --select 'impl=kdtree,query=exact_nn,axis=f64,k=1,tree_size=2^16,query_count=100'
cargo fmt --all -- --check
cargo test -p spatial-bench-kdtree
```

Expect one resolved point. The kdtree crate currently has no unit tests: its
`cargo test` checks compilation but does not instantiate the exported API macro.
The generated benchmark and conformance builds exercise those instantiations.
Conformance compares the manifest's compile-time coverage with the binary's registrations; it does not execute a known-answer
query test. Supply that evidence separately. A dry run can prepare/build an exec
driver's environment, so it is not necessarily free of builds or downloads.

The benchmark command prints its run-document path. Inspect the tags, positive
latency, sample count, source provenance and machine. Retain the command and
seed: the current run schema does not persist the seed. An unfingerprinted smoke
run establishes adapter execution; use the [results contribution guide](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md)
before proposing published measurements. Do not submit smoke runs as performance
evidence for the public corpus.

For local upstream development, `spatial-bench --subject-path NAME=/absolute/path
run ...` uses the checkout instead of the pin and marks checkout provenance.
Restore and validate a resolved immutable source before proposing publishable
results. See [running benchmarks](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/running-benchmarks.md)
for machine setup, perf and output inspection.

## Prepare the PR

- Explain supported operations, distance units, ordering, batching and actual
  threading; omit cases the adapter cannot implement faithfully.
- Supply immutable source/distribution provenance, driver compatibility bounds
  and minimum toolchain. Explain changed build flags.
- Run relevant driver tests, conformance and a small reproducible run. Include a
  known-answer or brute-force comparison for new query semantics; a checksum and
  positive timing are not correctness tests.
- Review setup versus timed work, result consumption, query normalization and
  truthful default/tuned labels. State library-specific work inside the timer.
- Link coordinated core changes for vocabulary, datasets or contracts. Identify
  corpus and public-methodology changes the contribution requires.
- Include commands, revisions and limitations. Target the default branch,
  currently `master`; see [automation limitations](docs/updating-a-library.md#maintainer-automation).
