# Update a library entry

A version bump or a configuration change is usually an edit to
`subjects/<name>/subject.toml` alone. If the library's API or the operation being
measured has changed, the driver needs editing too. Either way, use the
[setup guide](../CONTRIBUTING.md) to run the entry locally before and after.

## Update a release pin

Change `source.pinned_ref` together with its matching `sha`: a full commit for a Git
source, or the selected distribution's unprefixed 64-character SHA-256 for a
hash-pinned PyPI source. Checking the digest at the same time as the version is
what ensures the build resolves the artifact you intended.

Make sure the new release still falls inside the driver's supported version
interval, whose minimum is inclusive and maximum exclusive; kdtree's driver covers
`[0.8.0, 0.9.0)`. If it does not, extend the bound only after testing the adapter
against the new release. Also revisit `[defaults]` when a release changes the
library's ordinary configuration, since the engine classifies `default` and `tuned`
cases by comparing against those values.

## Adapt an API or configuration

When the change stays compatible with the driver's supported releases, update the
existing driver. A different API family is a different matter: add a named driver
with its own path and a disjoint version range, as the
[Kiddo manifest](../subjects/kiddo/subject.toml) does with its separate v5 and v6
drivers. Once a manifest declares more than one driver, cases have to name the one
they use.

The driver crate's `version` identifies the adapter package, not the library
release, so keep it distinct in your head and keep it consistent with the manifest
when you release an adapter change. Driver features and Rust flags affect the
generated build, whereas options chosen at runtime belong in case tags and
parameters rather than in the build settings.

To expose a tuning option, declare a namespaced key in `[vocab]`, add its values to
the case matrix, and record the ordinary value in `[defaults]`. The
[onboarding example](adding-a-library.md#describe-the-workload) explains how the
engine classifies the resulting cases.

## Extend or correct a measurement

Adding query cases requires implementing their semantics and a reference check
first. If the operation itself is new to core, this becomes a coordinated
[query-type change](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-query-types.md).
In either case, check [corpus.toml](../corpus.toml) for coverage of the new cases.

A change to allocation, batching or result consumption alters the measured
operation even when the library API is untouched, so explain the timing change in
the PR. If you are correcting an existing measurement, identify the historical
records that are affected and propose the correction through the
[results repository](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md);
updating a driver does not rewrite already-published data.

## Validate the update

Run the [PR checks](../CONTRIBUTING.md#prepare-a-pull-request) and a small selection
that exercises the changed path, including an existing case if you have extended
coverage. Before attributing a before-and-after difference to library performance,
compare the source pins, workload tags and timing boundaries across the two runs.
Include the commands you ran and the relevant correctness evidence in the PR.

## Maintainer automation

The [release watcher](../.github/workflows/release-watcher.yml) checks GitHub
releases for Git sources and PyPI for Python packages twice daily, with manual
dispatch also available, and prepares pin-update PRs for review.
[Standard Corpus](../.github/workflows/standard-corpus.yml) runs weekly or on
demand, while [Manual Benchmark](../.github/workflows/manual-trigger.yml) accepts a
subject and an optional version. These benchmark workflows use a configured
benchmark runner, retain the run artifacts, and submit records to the results
repository.

> The release-watcher script and the [pin-bump PR workflow](../.github/workflows/bench-pr.yml)
> currently target `main`, but this repository uses `master`. Ask maintainers to
> arrange any benchmark run you need rather than assuming a pin-update PR will
> trigger one.

Contributor validation does not depend on these workflows; everything described
above runs locally.