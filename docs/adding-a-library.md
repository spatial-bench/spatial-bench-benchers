# Add a library

An entry needs a `subjects/<name>/subject.toml` manifest and a driver that measures
the declared operations. Complete the [local setup](../CONTRIBUTING.md), then use
the kdtree entry as a Rust starting point or the C++/Python examples below.

## Define the cases

Choose the API operation and the coordinate types, dimensions, metrics and result
counts you will support. Specify whether each case uses individual or batched API
calls and how it uses threads. Begin with a small selection that you can compare
against a reference implementation.

The engine checks identifiers against a closed vocabulary. Find your library's
`impl` name in
[core vocab.rs](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/vocab.rs).
If it is absent, propose that identifier in a linked core PR. For new operations
or input distributions, use the core
[query](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-query-types.md)
or [dataset](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-datasets.md)
guide. Library-specific options can be declared in the subject's own `[vocab]`.

## Create a Rust entry

Copy [subjects/kdtree](../subjects/kdtree) into your subject directory. Rename the
manifest's `name` and `tags.impl` to your registered identifier. Give the driver
crate a distinct package name, update the manifest's `driver.crate`, and add its
path to the root [Cargo workspace](../Cargo.toml). Keep the core and measurement
helper dependencies; the workspace patches them to the adjacent engine checkout.

### Pin the source and select a driver

Set `[source]` to `kind = "cargo-git"`, the upstream repository and release tag,
plus the full commit in `sha`. Resolve an annotated tag to its commit. For example,
the existing kdtree source is:

```toml
[source]
kind = "cargo-git"
repo = "https://github.com/mrhooray/kdtree-rs"
pinned_ref = "v0.8.1"
sha = "c175108ba77175b614d0a35362ff1a67b53a9515"
```

The engine injects this dependency into the generated benchmark crate, where your
driver macro calls the library. The driver crate itself depends on the core and
measurement helpers.

Adapt the copied `[[driver]]` block's package path, macro and supported-version
range. kdtree uses `adapter = "rust-codegen"`, `macro = "bench_case"` and
`compile_time = ["axis"]`; its driver supports `[0.8.0, 0.9.0)`. The
[contract](driver-contract.md#build-settings) explains these fields.

### Describe the workload

Use the copied [kdtree case](../subjects/kdtree/subject.toml) to define:

| Manifest section | kdtree example | Your entry |
| --- | --- | --- |
| `tags` | `query = "exact_nn"`, `dims = 3`, `metric = "squared_euclidean"` | Fixed workload and API semantics |
| `matrix` | `axis = ["f32", "f64"]`, `k = [1, 5, 20, 50]` | Supported combinations to enumerate |
| `params` | Tree-size range `2^16..2^24`, query-count default `1000` | Values a selector may choose at runtime |
| `defaults` | `isa = "native"` | The ordinary configuration against which tuning is classified |

Remove unsupported combinations. Give each case an explicit `driver` name if the
manifest contains more than one driver. `exact_nn` uses k for the number of
neighbours, including nearest-one at k=1.

For a library parameter, declare its type or allowed values in `[vocab]` and refer
to it using its namespaced key, such as `mytree.bucket`, in cases and defaults.
The engine labels a case `default` when every declared default matches, and `tuned`
otherwise. Declare the settings needed to distinguish your tuning sweep; the
engine cannot infer the library's defaults. It rejects a manually supplied
`defaults_or_tuned` case tag.

### Adapt the measured operation

Read [kdtree's bench_case! macro](../subjects/kdtree/driver/src/lib.rs). For each
coordinate type it obtains points from the shared generator, builds a tree and
passes a closure to `measure`. The closure queries every probe and combines result
item IDs into a checksum. Replace the tree construction and query calls with your
API while preserving that flow.

The `measure` call receives the number of probes processed by one closure call.
It uses that count to normalize the batch timing to nanoseconds per query. Keep
input generation and construction outside a query benchmark's timed closure;
include per-call API work and result consumption inside it. See the
[measurement contract](driver-contract.md#measurement) for allocations and batching.

Retain `harness_main` and `--list` support. Each registration must report the keys
listed in `driver.compile_time`, plus compile-time extension keys. Other case
values are read at runtime.

## C++ and Python entries

For C++, copy the [nanoflann subject](../subjects/nanoflann), including its JSON and
other supporting headers. Update the Git source pin and the `cxx-shim` recipe's
compiler flags and include paths. Paths in `build.include` are relative to the
fetched library. `compile_time_dims` controls the generated C++ dimensionality
dispatch; the current shim advertises one registration and dispatches cases at
runtime. Adapt its query loop to your library.

For Python, copy [pykdtree](../subjects/pykdtree), change the PyPI package/version
and adapt `driver.py`. The `python-driver` recipe builds a virtual environment and
installs the selected distribution. The engine records its SHA-256; setting
`source.sha` requires that exact wheel or source archive. Wheel digests depend on
the artifact and platform. Python array conversions and binding calls inside the
query loop contribute to its measured cost. Represent a native batch API as a
separate `query_batching = "batch_query"` case.

Both use `adapter = "exec"` and the shared input/output protocol described in the
[driver contract](driver-contract.md). Preserve the generator invocation and
measurement procedure when replacing the library-specific calls.

## Validate and propose the entry

Follow the [PR checks](../CONTRIBUTING.md#prepare-a-pull-request), substituting your
subject and driver package. Inspect a small selection with `list`, run it and
compare its output tags with the API path exercised. For correctness evidence,
include inputs with ties or duplicate coordinates when these affect your query.

Check [corpus.toml](../corpus.toml) for an existing selection covering your cases.
Add a selection if the intended workload is missing. Each library runs the subset
its manifest supports. Link any required core change in the bencher PR so reviewers
can load and validate the complete entry.
