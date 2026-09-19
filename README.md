# spatial-bench-benchers

This repository holds the library manifests and benchmark drivers for
[spatial-bench](https://spatial-bench.org). A manifest selects one library release
and describes the benchmark cases for it; the matching driver translates those
cases into calls to the library's API. Entries currently cover Kiddo and kdtree in
Rust, nanoflann in C++, and pykdtree in Python.

To contribute an adapter, first
[set up the engine and run an existing one](CONTRIBUTING.md), then follow
[add a library](docs/adding-a-library.md) for a new entry or
[update an entry](docs/updating-a-library.md) for a version bump, a new
configuration or a corrected measurement. The
[driver contract](docs/driver-contract.md) documents the interface between the
engine and an adapter.

## Repository contents

- `subjects/<library>/subject.toml` — the source pin, build settings and supported
  cases for that library.
- `subjects/<library>/driver*` — Rust driver crates. The C++ and Python entries
  keep their executable drivers beside the manifest instead.
- [corpus.toml](corpus.toml) — the workload selections used by regular benchmark
  runs.
- [Cargo.toml](Cargo.toml) — the Rust driver workspace.

The [core repository](https://github.com/spatial-bench/spatial-bench-core) owns the
engine and the input generator, while the
[results repository](https://github.com/spatial-bench/spatial-bench-results) receives
run documents and publishes the database behind the
[explorer](https://spatial-bench.org/explore). Measurement procedures and how to
read them are described in the
[methodology](https://spatial-bench.org/methodology).