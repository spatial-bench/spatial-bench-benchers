# Driver contract

The engine resolves cases, builds a driver and sends it a run specification; the
driver obtains the input points, calls the library under test, and returns
measurements. Rust, C++ and Python drivers all share that transport contract, but
their statistical procedures differ, and those differences are described in the
[methodology](https://spatial-bench.org/methodology).

## Cases and registrations

A manifest expands its `tags` and `matrix` values into cases, and selectors resolve
each case's `params` into measured points. The
[onboarding guide](adding-a-library.md#describe-the-workload) covers how to declare
those cases and their default settings.

For a Rust driver, `driver.compile_time` lists the core keys that require separate
instantiations, and library-specific vocabulary entries carry their own
`compile_time` flag. The driver's `--list` output advertises each resulting
combination, one JSON object per line:

```json
{"compile_time":[["axis","f32"]]}
{"compile_time":[["axis","f64"]]}
```

An executable that dispatches all cases at runtime emits `{"compile_time":[]}`
instead. Listing does not read a run specification; these registrations are exactly
what `spatial-bench conform` checks.

## Input and output

Without `--list`, the executable reads one JSON `RunSpec` from stdin. The current
`harness_version` is **2**, and a driver should reject versions it does not
understand. The top-level fields are:

| Field | Meaning |
| --- | --- |
| `harness_version` | Protocol version |
| `budget` | `warm_up_ms`, `measurement_ms`, `sample_size` |
| `cases` | Cases to measure, each containing `id`, resolved `tags`, `dataset_generator`, `dataset`, `random_seed` |

Each completed case produces one JSON `Point` on stdout with its tags preserved.
Diagnostics go to stderr, and a failure should exit nonzero. The
[Point schema](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/schema.rs)
defines `metrics`, the optional `stats` and `provenance`. Latency is recorded as
`latency_ns` in `ns/query`, carrying a mean point estimate and optional lower and
upper bounds; `throughput_qps` gives queries per second. Optional statistics that
are unavailable may be omitted or null, as the schema allows.

### Shared input generator

Invoke `case.dataset_generator` with `--kind`, `--dims`, `--dtype`, `--tree-count`,
`--query-count` and `--seed`, using the resolved case values. The supplied generator
supports the `uniform` and `gaussian` distributions, and its SBDS output contains a
header followed by dimension-contiguous tree points and then query points. Reuse the
existing Rust, C++ or Python decoder from the example driver rather than
reimplementing it. The core
[dataset guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-datasets.md)
documents the format and the extension path.

Whichever generator you use, the query implementation has to match the case's
distance units, ordering and radius boundary. Best-item-within-radius and
nearest-by-distance-within-radius are genuinely different operations, not two ways
of describing one. Query count always counts probes, including every probe inside a
batch API call. If your driver cannot implement a case, reject it.

## Measurement

For a query benchmark, generate the inputs and construct the index before timing
starts. The timed body should include the API call, any per-call conversions and
allocations it needs, and the consumption of its results. Where an API accepts
reusable output storage, prepare that storage outside the loop and describe the
choice in the case's implementation notes. Keep the computation consumed by making
some checksum depend on the returned results.

### Rust

A rust-codegen crate exports the macro named in the manifest along with
`harness_main`. The engine injects the pinned library dependency into a generated
crate and invokes the macro once per selected compile-time combination; the macro
returns a registration whose function accepts `CaseSpec` and `Budget`.

The [kdtree driver](../subjects/kdtree/driver/src/lib.rs) calls
`spatial_bench_measure::measure(case, budget, probes.len() as u64, body)` after
building its tree. `measure` passes the body's return value through `black_box`,
uses Criterion, and divides batch estimates by the probe count it was given. That
count must equal the work done by one complete body invocation, accumulated across
chunks if the API is batched. Passing a count of API calls instead would misstate
per-query latency.

### C++ and Python

The executable drivers warm up for the budget duration and then time complete probe
loops, stopping sampling when either the requested sample count or the measurement
time is reached. Each elapsed time is divided by the number of probes. These drivers
report an arithmetic mean with a normal-approximation interval, plus median and
dispersion statistics. Rust's Criterion path samples and analyses differently, so
consult the methodology when changing either procedure.

Stored throughput is `1e9 / mean_latency_ns`. The perf runner wraps the entire
driver process, which means its counters include setup and analysis as well as query
execution.

## Build settings

| Setting | Purpose |
| --- | --- |
| `source.pinned_ref`, `source.sha` | Release and expected source identity: a Git commit or the SHA-256 of a PyPI distribution |
| `driver.min_supported_semver`, `driver.max_supported_semver` | Adapter compatibility interval, inclusive minimum and exclusive maximum |
| `toolchain.min_rustc` | Minimum Rust toolchain used when selecting a common toolchain |
| `driver.features`, `driver.rustflags` | Rust library features and compiler flags |
| `driver.compile_time`, extension `compile_time` | Keys requiring separate generated Rust instantiations |
| `build.compile_time_dims` | Dimensions compiled into a C++ shim's dispatch |

PyPI digests are written in the manifest as 64 hexadecimal characters without a
prefix; the run record adds `sha256:` when it resolves that distribution digest.
When the manifest carries no digest, the engine records the resolved source identity
instead. Passing `--subject-path NAME=DIR` lets a Rust library checkout stand in
during development and marks the run with working-tree provenance. C++ recipes
additionally specify include paths and any optional source-build dependencies, while
Python recipes prepare a virtual environment.

## Implementation references

- [Manifest types](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/manifest.rs)
- [Run and registration protocol](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/harness.rs)
- [Rust measurement helper](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-measure/src/lib.rs)
- [Executable preparation](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/exec.rs)

The contract was checked against core
[`2104bfe`](https://github.com/spatial-bench/spatial-bench-core/tree/2104bfe), and
the driver examples against benchers
[`1972c50`](https://github.com/spatial-bench/spatial-bench-benchers/tree/1972c50).