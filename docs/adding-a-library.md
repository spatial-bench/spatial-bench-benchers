# Add a library

Adding a library means writing two things: a `subjects/<name>/subject.toml`
manifest that declares what will be benchmarked, and a driver that actually
measures those operations. Complete the [local setup](../CONTRIBUTING.md) first,
then use the kdtree entry as a Rust starting point, or one of the C++ and Python
examples further down.

## Define the cases

Decide up front which API operation you are measuring and which coordinate types,
dimensions, metrics and result counts you intend to support. It also matters
whether each case makes individual or batched API calls and how it uses threads,
because both change what the timed region contains. Start with a small selection
that you can check against a reference implementation rather than trying to cover
the whole API.

The engine validates identifiers against a closed vocabulary, so look for your
library's `impl` name in the core
[vocab.rs](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/vocab.rs).
If it is not there, propose the identifier in a linked core PR. New operations or
input distributions are larger changes and follow the core
[query](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-query-types.md)
and [dataset](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-datasets.md)
guides. Options specific to your library do not need core at all; declare them in
the subject's own `[vocab]`.

## Create a Rust entry

Copy [subjects/kdtree](../subjects/kdtree) into a new subject directory, then
rename the manifest's `name` and `tags.impl` to your registered identifier. Give
the driver crate a distinct package name, point the manifest's `driver.crate` at
it, and add its path to the root [Cargo workspace](../Cargo.toml). Leave the core
and measurement helper dependencies in place, since the workspace patches them to
the adjacent engine checkout.

### Pin the source and select a driver

A `[source]` of `kind = "cargo-git"` needs the upstream repository, the release tag
and the full commit in `sha`, so resolve an annotated tag to its commit rather than
using the tag name. The existing kdtree source shows the shape:

```toml
[source]
kind = "cargo-git"
repo = "https://github.com/mrhooray/kdtree-rs"
pinned_ref = "v0.8.1"
sha = "c175108ba77175b614d0a35362ff1a67b53a9515"
```

The engine injects this dependency into the generated benchmark crate, which is
where your driver macro calls into the library. The driver crate itself continues
to depend on the core and measurement helpers.

Next, adapt the copied `[[driver]]` block: its package path, the macro it exports
and the range of library versions it supports. kdtree uses
`adapter = "rust-codegen"`, `macro = "bench_case"` and `compile_time = ["axis"]`,
and its driver covers `[0.8.0, 0.9.0)`. The
[contract](driver-contract.md#build-settings) explains what each of those fields
means.

### Describe the workload

The copied kdtree case is a reasonable template for the manifest sections you need:

| Manifest section | kdtree example | Your entry |
| --- | --- | --- |
| `tags` | `query = "exact_nn"`, `dims = 3`, `metric = "squared_euclidean"` | Fixed workload and API semantics |
| `matrix` | `axis = ["f32", "f64"]`, `k = [1, 5, 20, 50]` | Supported combinations to enumerate |
| `params` | Tree-size range `2^16..2^24`, query-count default `1000` | Values a selector may choose at runtime |
| `defaults` | `isa = "native"` | The ordinary configuration against which tuning is classified |

Remove any combination your library does not support. If the manifest ends up with
more than one driver, each case needs an explicit `driver` name. The `exact_nn`
tag counts `k` neighbours, with `k=1` meaning the single nearest point.

Library parameters are declared in `[vocab]` with their type or allowed values, and
then referred to by a namespaced key such as `mytree.bucket` in both cases and
defaults. The engine labels a case `default` when every declared default matches and
`tuned` otherwise, which means it cannot infer your library's defaults, so you have
to declare the settings that distinguish a tuning sweep. Supplying `defaults_or_tuned`
as a case tag by hand is rejected.

### Adapt the measured operation

Read [kdtree's `bench_case!` macro](../subjects/kdtree/driver/src/lib.rs) before
editing anything. For each coordinate type it obtains points from the shared
generator, builds a tree, and hands a closure to `measure`; that closure queries
every probe and folds the returned item IDs into a checksum. Your job is to replace
the tree construction and the query calls with your library's API while leaving
that overall flow intact.

`measure` is told how many probes one closure call processed, and uses that count
to normalize the batch timing into nanoseconds per query. So keep input generation
and construction outside a query benchmark's timed closure, and keep the per-call
API work and result consumption inside it. The
[measurement contract](driver-contract.md#measurement) covers allocations and
batching in more detail.

Retain `harness_main` and the `--list` support. Every registration must report the
keys listed in `driver.compile_time` plus any compile-time extension keys; the
remaining case values are read at runtime.

## C++ and Python entries

For C++, copy the [nanoflann subject](../subjects/nanoflann), including its JSON
and supporting headers, then update the Git source pin and the `cxx-shim` recipe's
compiler flags and include paths. Paths in `build.include` are relative to the
fetched library. `compile_time_dims` controls the generated C++ dimensionality
dispatch; the current shim advertises a single registration and dispatches cases at
runtime. Adapt its query loop to your library.

For Python, copy [pykdtree](../subjects/pykdtree), change the PyPI package and
version, and adapt `driver.py`. The `python-driver` recipe builds a virtual
environment and installs the selected distribution, whose SHA-256 the engine
records; setting `source.sha` requires exactly that wheel or source archive, and
wheel digests depend on both the artifact and the platform. Bear in mind that the
array conversions and binding calls inside the query loop contribute to the
measured cost. If your library offers a native batch API, expose it as a separate
`query_batching = "batch_query"` case.

Both languages use `adapter = "exec"` and the shared input/output protocol described
in the [driver contract](driver-contract.md). When you replace the library-specific
calls, preserve the generator invocation and the measurement procedure.

## Validate and propose the entry

Work through the [PR checks](../CONTRIBUTING.md#prepare-a-pull-request) with your
own subject and driver package substituted. Inspect a small selection with `list`,
run it, and compare the output tags against the API path actually exercised. For
correctness evidence, include inputs with tied or duplicate coordinates whenever
those affect your query.

Check [corpus.toml](../corpus.toml) for a selection that already covers your cases
and add one if the intended workload is missing; each library runs the subset its
manifest supports. If a core change was needed, link it in the bencher PR so
reviewers can load and validate the complete entry.