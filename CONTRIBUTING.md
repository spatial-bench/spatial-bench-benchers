# Contributing benchmark adapters

Use the kdtree example below to build a driver and record a measurement before
changing an entry. It selects a small workload and takes about eight seconds to
measure, after compilation.

## Set up the checkouts

Use Linux for benchmark runs, with Git and rustup installed. The Rust workspace
expects the core checkout beside this repository under the directory name
`spatial-bench`:

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

Build both binaries: the driver invokes `spatial-bench-dataset` to obtain input
points. The engine selects a common Rust toolchain from the chosen manifests;
`rustup toolchain install 1.89.0` supplies the one used by the kdtree example.
For engine build prerequisites, see the core
[development guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/development.md).
C++ adapters also need a C++20 compiler, and some build recipes use CMake.
Python adapters need Python 3 with venv and pip support.

## Run an existing adapter

From the bencher repository root:

```sh
spatial-bench list --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench run --dry-run \
  --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
spatial-bench run --allow-unfingerprinted \
  --select 'impl=kdtree,axis=f64,k=1,tree_size=2^16,query_count=100'
```

The list reports `1 cases, 1 points`. The plan selects one build combination. The
run builds the pinned kdtree release and measures individual nearest-neighbour
queries over 65,536 three-dimensional points with f64 coordinates. The flag
`--allow-unfingerprinted` permits this local smoke test without provisioning a
benchmark machine fingerprint.

The engine prints the run document's path under `$XDG_DATA_HOME/spatial-bench/runs`,
or `~/.local/share/spatial-bench/runs` when XDG_DATA_HOME is unset. Inspect
`run.subjects` for the source pin and `points` for workload tags and measured
values. For publishable measurements, follow the core
[running guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/running-benchmarks.md)
and the [results contribution guide](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md).

Use `list` to inspect other adapters before building them. An executable adapter's
`--dry-run` can prepare its C++ build or Python environment while resolving the plan.

## Prepare a pull request

For a Rust driver, run its package checks. For every adapter, check its registrations
and run a small case. The kdtree commands are:

```sh
cargo fmt -p spatial-bench-kdtree -- --check
cargo test -p spatial-bench-kdtree
spatial-bench conform --subject kdtree
```

`conform` compares the driver's `--list` output with the manifest's compile-time
combinations. Supply a separate known-answer or reference comparison for query
correctness; registration coverage does not check returned neighbours.

Include the affected release and cases, the run command, test output and any linked
core PR. Describe the API call and work inside the timed loop, including allocations
and result handling. State how you chose default settings. This gives maintainers
the evidence needed to review the adapter independently of its author.

Update the relevant guide when changing the driver contract, case semantics or
measurement interpretation. The [add](docs/adding-a-library.md) and
[update](docs/updating-a-library.md) guides identify the files involved.
