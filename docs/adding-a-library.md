# Add a library

Choose the smallest existing adapter that matches your language and API. The
catalog belongs here; engine vocabulary or protocol changes need a coordinated
core PR. Read [setup](../CONTRIBUTING.md) first.

## Start with one operation

For Rust, use [kdtree's manifest](../subjects/kdtree/subject.toml) and
[driver](../subjects/kdtree/driver/src/lib.rs). It exposes 3D exact
nearest-neighbour queries with squared Euclidean distance, `f32`/`f64`
coordinates, `u32` payloads and `k` values 1, 5, 20 and 50. It is a smaller
starting point than Kiddo's multiple API generations and specialized kernels.

Create `subjects/NAME/subject.toml` and a sibling `driver/` crate. Copy the
structure, then replace identity, source, crate names, API calls, capabilities
and compatibility bounds. Add the Rust driver to the root workspace members.
The driver depends on the engine core and measurement crates; the generated
harness supplies the pinned library dependency used by the exported macro. Do
not accidentally benchmark a differently pinned library by adding an independent
upstream dependency to the adapter.

The engine validates `impl` against its [vocabulary](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/vocab.rs).
An absent identity needs a core PR before the catalog can load. The same applies
to new shared queries, datasets and tag values.

## Describe the source and compatible driver

Kdtree uses this shape for its Git-backed Rust dependency:

```toml
[source]
kind = "cargo-git"
repo = "https://github.com/mrhooray/kdtree-rs"
pinned_ref = "v0.8.1"
sha = "c175108ba77175b614d0a35362ff1a67b53a9515"

[[driver]]
name = "v0"
min_supported_semver = "0.8.0"
max_supported_semver = "0.9.0"
adapter = "rust-codegen"
crate = "spatial-bench-kdtree"
path = "driver"
version = "0.1.0"
macro = "bench_case"
compile_time = ["axis"]
```

The supported range is lower-inclusive and upper-exclusive. It describes library
API versions, not the adapter crate's version. Choose bounds from compatibility
evidence. With several drivers, cases name their `driver`; see
[Kiddo](../subjects/kiddo/subject.toml).

Source kinds include `cargo`, `cargo-git`, `git` and `pypi`; their fields and build
paths differ. Resolve Git tags to actual commits and record immutable SHAs,
rather than relying on a moving branch or tag. PyPI uses an exact package version
and distribution hash as described below.

## Map capabilities to cases

`case.tags` fixes a value. `case.matrix` enumerates alternatives; `case.params`
describes runtime values and defaults. Kdtree declares an `axis`/`k` matrix,
a `tree_size` range and an integer `query_count`. Unconstrained runtime parameters
resolve to defaults: a broad selector does not test every possible value. Use
`list` to inspect actual expansion before running.

`driver.compile_time` names axes requiring generated Rust instantiations. Kdtree
needs separate macros for `f32` and `f64`; `k`, tree size and query count come
from `CaseSpec` at runtime. Keep an axis at runtime when the API permits it.
Library-specific extensions belong in the manifest's vocabulary declarations;
Kiddo demonstrates extensions participating in compile-time expansion.

Declare only operations implemented by the adapter. A driver fixed at `dims=3`
should not advertise other dimensions merely because upstream supports them.
`exact_nn` with varying `k` does not imply radius queries. Match metric,
ordering, batching and concurrency labels to the actual call. Single-threaded
dispatch does not prove the underlying library starts no workers.

`[defaults]` classifies `defaults_or_tuned` by comparing declared values. It does
not inspect upstream constructors. With no defaults the comparison is vacuous,
so all configurations can be labelled default. Review constructor settings and
fixed flags too; explain why they represent upstream defaults or deliberate tuning.

## Adapt the measured call

Kdtree obtains shared inputs from `case.dataset_generator`, builds the tree
before timing and calls `measure(case, budget, probes.len() as u64, closure)`.
The closure queries every probe and consumes a returned item in a checksum.
Query cost therefore includes per-query result allocation, lookup and checksum,
but excludes generation and construction. Preserve this boundary or explicitly
define a different operation with reviewers.

Before collecting timing, compare answers on a small deterministic dataset with
known answers or brute force. Exercise ties, distance units, radius boundaries
and ordering where applicable. Keep this check outside the timer. See the
[driver contract](driver-contract.md) for transport and normalization.

## C++ and Python adapters

For C++, start with [nanoflann](../subjects/nanoflann/subject.toml) and
[shim.cpp](../subjects/nanoflann/shim.cpp). Its `exec` driver names a C++ entry
file; the `cxx-shim` recipe supplies C++20, optimization flags, includes and
compile-time dimensions. Keep adjacent headers with the subject. Record explicit
flags such as `-march=native`: exec compilation uses its recipe, not the Rust ISA
resolver. Use existing build-recipe facilities for more involved builds only
when the library needs them.

For Python, start with [pykdtree](../subjects/pykdtree/subject.toml) and
[driver.py](../subjects/pykdtree/driver.py). The `python-driver` recipe creates a
virtual environment and installs the pinned distribution. Without a manifest SHA,
the builder downloads the selected PyPI distribution, computes its SHA-256 and
records it in subject provenance. This records the downloaded artifact; it does
not compare it with an independently supplied expected digest. An explicit SHA must match the selected artifact, which can
vary by Python/platform wheel. Use an unprefixed hexadecimal SHA-256 in
`source.sha`: the catalog currently rejects `sha256:` even though recorded
provenance uses that prefix.

Both exec examples consume harness v2 and the shared generator output. Their
handwritten timing loops differ statistically from Rust/Criterion. Do not call
them Criterion measurements or substitute an independently seeded language-specific
generator. Python conversions, binding overhead and batch dispatch inside the
timing loop are part of the measurement; explain which calls are included.

Follow the [PR checklist](../CONTRIBUTING.md#prepare-the-pr). Extend
[corpus.toml](../corpus.toml) when cases belong in the shared experiment; onboarding
does not require inventing unsupported operations.
