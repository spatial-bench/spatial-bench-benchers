# Contributing benchmark adapters

Before changing any entry, work through the kdtree example below to build a driver
and record a measurement. It selects a small workload and, once compilation is
done, takes about eight seconds to measure.

## Set up the checkouts

Benchmark runs are supported on Linux, so you will need Git, rustup and the usual
native toolchain. The Rust workspace expects the core checkout beside this
repository under the directory name `spatial-bench`:

```sh
git clone https://github.com/spatial-bench/spatial-bench-core.git spatial-bench
git clone https://github.com/spatial-bench/spatial-bench-benchers.git
cd spatial-bench-benchers
cargo build --release --manifest-path ../spatial-bench/Cargo.toml \
  -p spatial-bench -p spatial-bench-dataset
export PATH="$(realpath ../spatial-bench/target/release):$PATH"
export SPATIAL_BENCH_ENGINE_SRC="$(realpath ../spatial-bench)"
export SPATIAL_BENCH_SUBJECTS="$PWD/subjects"
```

Both binaries matter: the driver calls `spatial-bench-dataset` to obtain its input
points. The engine picks a common Rust toolchain from the manifests it is about to
build, and `rustup toolchain install 1.89.0` provides the one the kdtree example
needs. Engine build prerequisites are listed in the core
[development guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/development.md).
If you are adding a C++ adapter, you will also need a C++20 compiler and, for some
build recipes, CMake; Python adapters need Python 3 with venv and pip support.

## Run an existing adapter

From the bencher repository root:

```sh
spatial-bench list --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench run --dry-run \
  --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench run --allow-unfingerprinted \
  --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
```

The `list` command should report `1 cases, 1 points`, and the plan one build
combination. The run itself builds the pinned kdtree release and measures
individual nearest-neighbour queries over 65,536 three-dimensional points with
f64 coordinates. Adding `--allow-unfingerprinted` lets this local smoke test run
without provisioning a benchmark machine fingerprint.

The engine prints the path of the run document under
`$XDG_DATA_HOME/spatial-bench/runs`, or `~/.local/share/spatial-bench/runs` when
`XDG_DATA_HOME` is unset. Read `run.subjects` for the source pin and `points` for
the workload tags and measured values. For measurements you intend to publish,
follow the core
[running guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/running-benchmarks.md)
and the [results contribution guide](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md).

Use `list` to inspect other adapters before building them. Note that running an
executable adapter with `--dry-run` can prepare its C++ build or Python environment
while it resolves the plan, so it is not always as cheap as inspecting the catalog.

## Prepare a pull request

Rust drivers should pass their package checks; every adapter, regardless of
language, should check its registrations and run a small case. For kdtree that
means:

```sh
cargo fmt -p spatial-bench-kdtree -- --check
cargo test -p spatial-bench-kdtree
spatial-bench conform --subject kdtree
```

`conform` compares the driver's `--list` output against the manifest's compile-time
combinations, which is a check on declared coverage and nothing more. It does not
look at the neighbours returned. Supply a separate known-answer or reference
comparison for query correctness.

A reviewable PR states the affected release and cases, the run command and its
output, and any linked core PR. It also describes the API call being timed and what
work sits inside the timed loop, including allocations and result handling, and
explains how the default settings were chosen. Together these give a maintainer
enough to assess the adapter independently of its author.

Update the relevant guide whenever you change the driver contract, the semantics of
a case, or the interpretation of a measurement. The [add](docs/adding-a-library.md)
and [update](docs/updating-a-library.md) guides list the files involved.