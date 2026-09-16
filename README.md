# spatial-bench-benchers

Library manifests and benchmark drivers for
[spatial-bench](https://spatial-bench.org). Each manifest selects a library release
and describes its benchmark cases; its driver calls the library's API. Entries
cover Kiddo and kdtree in Rust, nanoflann in C++, and pykdtree in Python.

To contribute, [set up the engine and run an adapter](CONTRIBUTING.md), then follow
[add a library](docs/adding-a-library.md) or
[update an entry](docs/updating-a-library.md). The
[driver contract](docs/driver-contract.md) describes the interface between the
engine and an adapter.

## Repository contents

- `subjects/<library>/subject.toml`: source pin, build settings and supported cases.
- `subjects/<library>/driver*`: Rust driver crates; C++ and Python entries keep
  their executable drivers beside the manifest.
- [corpus.toml](corpus.toml): workload selections for regular benchmark runs.
- [Cargo.toml](Cargo.toml): the Rust driver workspace.

The [core repository](https://github.com/spatial-bench/spatial-bench-core) owns the
engine and input generator. The
[results repository](https://github.com/spatial-bench/spatial-bench-results) receives
run documents and publishes the database used by the
[explorer](https://spatial-bench.org/explore). Read the
[methodology](https://spatial-bench.org/methodology) for measurement procedures and
interpretation.
