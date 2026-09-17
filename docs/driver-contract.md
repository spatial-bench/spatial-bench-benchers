# Driver contract

The engine selects cases, builds a driver and sends it a run specification. The
driver obtains the input points, calls the library and returns measurements. Rust,
C++ and Python drivers share that transport contract; their statistical procedures
are described in the [methodology](https://spatial-bench.org/methodology).

## Cases and registrations

A manifest expands fixed `tags`, enumerated `matrix` values and selector-controlled
`params` into cases. The [onboarding guide](adding-a-library.md#describe-the-workload)
shows how to declare them and their default settings.

Rust `driver.compile_time` lists core keys requiring separate instantiations.
Library-specific vocabulary entries have their own `compile_time` flag. A driver's
`--list` output advertises each resulting combination, one JSON object per line:

```json
{"compile_time":[["axis","f32"]]}
{"compile_time":[["axis","f64"]]}
```

An executable that handles all cases through runtime dispatch emits
`{"compile_time":[]}`. Listing reads no run specification. These are the
registrations checked by `spatial-bench conform`.

## Input and output

Without `--list`, the executable reads one JSON `RunSpec` from stdin. The current
`harness_version` is **2**; reject unsupported versions. The top-level fields are:

| Field | Meaning |
| --- | --- |
| `harness_version` | Protocol version |
| `budget` | `warm_up_ms`, `measurement_ms`, `sample_size` |
| `cases` | Cases to measure, each containing `id`, resolved `tags`, `dataset_generator`, `dataset`, `random_seed` |

Write one JSON `Point` per completed case to stdout, preserving its tags. Send
diagnostics to stderr and exit nonzero on failure. The
[Point schema](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/schema.rs)
defines `metrics`, optional `stats` and `provenance`. Latency is recorded as
`latency_ns` in `ns/query`, with a mean point estimate and optional lower/upper
bounds. `throughput_qps` is queries per second; unavailable optional statistics
may be omitted or null according to the schema.

### Shared input generator

Invoke `case.dataset_generator` with `--kind`, `--dims`, `--dtype`, `--tree-count`,
`--query-count` and `--seed`, using the resolved case values. The supplied generator
supports uniform and Gaussian distributions. Its SBDS output contains a header
followed by dimension-contiguous tree points and then query points. Reuse the
existing Rust, C++ or Python decoder in the example driver. See the core
[dataset guide](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-datasets.md)
for the format and extension path.

The query implementation must match the case's distance units, ordering and radius
boundary. Best-item-within-radius and nearest-by-distance-within-radius are
different operations. Query count counts probes, including every probe inside a
batch API call. Reject a case your driver does not implement.

## Measurement

For a query benchmark, generate inputs and construct the index before timing.
Include the API call, required per-call conversions and allocations, and result
consumption in the timed body. If an API accepts reusable output storage, prepare
it outside the loop and describe that choice in the case's implementation. Keep
a checksum dependent on the returned results so the computation is consumed.

### Rust

A rust-codegen crate exports the macro named in the manifest and `harness_main`.
The engine injects the pinned library dependency into a generated crate and invokes
the macro for each selected compile-time combination. The macro returns a
registration whose function accepts `CaseSpec` and `Budget`.

The [kdtree driver](../subjects/kdtree/driver/src/lib.rs) calls
`spatial_bench_measure::measure(case, budget, probes.len() as u64, body)` after
building its tree. `measure` passes the body's return value through `black_box`,
uses Criterion and divides batch estimates by the supplied probe count. That count
must match the work done by one complete body invocation, across all chunks if the
API uses batches. A count of API calls would misstate per-query latency.

### C++ and Python

The existing executable drivers warm up for the budget duration, then time complete
probe loops. Sampling stops when either the requested count or measurement time
has been reached. Each elapsed time is divided by the number of probes. These
drivers report an arithmetic mean with a normal-approximation interval, plus
median and dispersion statistics. Rust's Criterion path uses its own sampling and
analysis; consult the methodology when changing either procedure.

Stored throughput is `1e9 / mean_latency_ns`. Perf wraps the entire driver process,
so its counters include setup and analysis as well as query execution.

## Build settings

| Setting | Purpose |
| --- | --- |
| `source.pinned_ref`, `source.sha` | Release and expected source identity: a Git commit or the SHA-256 of a PyPI distribution |
| `driver.min_supported_semver`, `driver.max_supported_semver` | Adapter compatibility interval, inclusive minimum and exclusive maximum |
| `toolchain.min_rustc` | Minimum Rust toolchain used when selecting a common toolchain |
| `driver.features`, `driver.rustflags` | Rust library features and compiler flags |
| `driver.compile_time`, extension `compile_time` | Keys requiring separate generated Rust instantiations |
| `build.compile_time_dims` | Dimensions compiled into a C++ shim's dispatch |

Write a PyPI digest in the manifest as 64 hexadecimal characters without a
prefix. The run record adds `sha256:` to the resolved distribution digest. Without
a manifest digest, the engine records the resolved source identity. `--subject-path NAME=DIR` allows
a Rust library checkout during development and marks working-tree provenance.
C++ recipes also specify include paths and optional source-build dependencies;
Python recipes prepare a virtual environment.

## Implementation references

- [Manifest types](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/manifest.rs)
- [Run and registration protocol](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/harness.rs)
- [Rust measurement helper](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-measure/src/lib.rs)
- [Executable preparation](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/exec.rs)

Contract checked against core
[`2104bfe`](https://github.com/spatial-bench/spatial-bench-core/tree/2104bfe) and
driver examples against benchers
[`1972c50`](https://github.com/spatial-bench/spatial-bench-benchers/tree/1972c50).
