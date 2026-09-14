# Driver input, output and measurement

An adapter maps a shared workload to a library API and reports a point per
resolved case. Common transport and result shapes do not make runtimes, sampling
procedures or API costs identical. The [methodology](https://spatial-bench.org/methodology)
is the canonical explanation of statistical interpretation.

## Harness v2

The engine sends one JSON `RunSpec` on stdin: `harness_version`, `budget`, and
`cases`. Each `CaseSpec` has an ID, resolved tags, `dataset_generator` path,
dataset kind and `random_seed`. The executable contract is
[harness.rs](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/harness.rs).
Reject unsupported versions and invalid inputs. Write serialized `Point` records
as JSON Lines on stdout, diagnostics on stderr, and exit nonzero on failure.
Do not mix progress logs into the JSON stream.

`--list` returns the harness registration-list format. Rust registrations identify
compiled tag combinations, not every runtime tree size or query count.
`conform --subject NAME` compares that list with the catalog. Use core serializers
in Rust and the exec examples for framing; do not invent another schema.

The output types live in [schema.rs](https://github.com/spatial-bench/spatial-bench-core/blob/main/crates/spatial-bench-core/src/schema.rs).
Preserve case identity, tags, metric units and available statistics. The engine
adds run provenance; a failed query must not become a zero-cost measurement.

## Inputs and query semantics

Invoke the supplied generator with the requested distribution, dimensions,
scalar type, tree/query counts and seed. Its `SBDS` stream has a header followed
by construction and query coordinates. The [generator source](https://github.com/spatial-bench/spatial-bench-core/tree/main/crates/spatial-bench-dataset)
is the byte-format authority. Validate header, counts and body consistency.
Current drivers decode native-endian values: this local subprocess protocol is
not a portable archival format.

Implemented distributions are uniform and Gaussian. The CLI defaults to seed
42; the generator uses the next wrapping seed for query inputs. Legacy fixed-seed
constants and standalone generator self-checks do not describe the harness-v2
path. Do not substitute independently generated input.

Specify distance units, exactness, radius inclusion, result ordering, payload
semantics and batching where relevant. Library capability differs from adapter
coverage. Verify answers outside timing, including relevant ties and boundaries;
registration conformance establishes none of these properties.

## Timing and normalization

For existing query cases, generation and construction precede timing. The closure
executes Q queries, consumes results, and passes Q to the measurement helper.
Divide batch time by Q exactly once. Q is queries executed, not neighbours
returned. Batch APIs also need their actual query count as denominator. Preserve
observable consumption so optimized builds cannot discard the work.

Kdtree includes result allocation and checksum work in timing. Python includes
conversions and dispatch inside its loop; a native extension call does not remove
Python overhead. For a construction operation, define timing and denominator
separately rather than reusing a query-cost label. Review preprocessing and
allocation boundaries explicitly.

Default budgets are 3,000 ms warmup, 5,000 ms measurement and 30 samples. Rust
uses Criterion via `spatial-bench-measure`. Current C++/Python loops stop when
either the sample limit or time budget is reached. They compute population SD,
upper-middle median, unscaled MAD and a normal approximation interval around the
mean. Criterion's MAD scaling, sample SD, resampling and sampling strategy differ;
consult the methodology before comparing estimators.

Stored `latency_ns` is the mean; throughput is its reciprocal scaled to seconds.
The explorer normally plots median with mean fallback. Latency bounds remain
bounds around the mean, not a median confidence interval.

`perf` wraps the whole driver process, including setup, generation, warmup,
sampling and analysis. Its counters are not per-query timed-region measurements.
Review ISA and threading labels against actual compiler flags and runtime
behaviour, particularly for exec drivers.

## Review evidence

Supply commands, adapter/catalog and upstream revisions, small-case output,
conformance and a separate answer-correctness check. Describe library-specific
timed work and configuration. Preserve seed and environment externally when the
run schema omits them. New harness fields or output meanings need a coordinated
core change and compatibility discussion; case additions under the existing
contract generally do not.

Verified against core `2104bfe` and benchers `1972c50` on 2026-09-14. Recheck when
harness, dataset format, sampling or adapter boundaries change.
