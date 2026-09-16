# Update a library entry

Edit `subjects/<name>/subject.toml` for a new source release or configuration.
Change its driver when the API or measured operation changes. Use the
[setup guide](../CONTRIBUTING.md) to run the entry locally.

## Update a release pin

Change `source.pinned_ref` and its matching `sha`: a full commit for Git sources,
or the selected distribution's SHA-256 for a hash-pinned PyPI source. The engine
accepts a `sha256:` prefix for PyPI. Check the digest together with the version so
the build verifies the intended artifact.

Check that the release falls within the driver's supported version interval.
The minimum is inclusive and the maximum exclusive; kdtree's driver covers
`[0.8.0, 0.9.0)`. Extend a bound after testing the adapter against the new release.
Revisit `[defaults]` if the library changes its ordinary configuration.

## Adapt an API or configuration

Update the existing driver when the change remains compatible with its supported
releases. For a different API family, add a named driver with its own path and a
disjoint version range. The [Kiddo manifest](../subjects/kiddo/subject.toml) shows
separate v5 and v6 drivers. Cases name their driver explicitly when more than one
is declared.

The driver crate's `version` identifies the adapter package; it is separate from
the library's source release. Keep the manifest and driver package versions
consistent when releasing an adapter change. Driver features and Rust flags affect
the generated build; runtime options belong in case tags and parameters.

For a tuning option, declare a namespaced key in `[vocab]`, add its values to the
case matrix and record its ordinary value in `[defaults]`. The
[onboarding example](adding-a-library.md#describe-the-workload) explains how the
engine classifies those cases.

## Extend or correct a measurement

Add query cases after implementing their semantics and a reference check. A new
core operation requires a coordinated
[query-type change](https://github.com/spatial-bench/spatial-bench-core/blob/main/docs/adding-query-types.md).
Check [corpus.toml](../corpus.toml) for coverage of the new cases.

Changes to allocation, batching or result consumption can change the measured
operation even when the library API is unchanged. Explain the timing change in
the PR and identify affected historical records when correcting a measurement.
Propose corrections to those records through the
[results repository](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md);
a driver update does not rewrite published data.

## Validate the update

Run the [PR checks](../CONTRIBUTING.md#prepare-a-pull-request) and a small selection
covering the changed path. Include an existing case when extending coverage.
Compare source pins, workload tags and timing boundaries before attributing a
before/after difference to library performance. Include the commands and relevant
correctness evidence in the PR.

## Maintainer automation

The [release watcher](../.github/workflows/release-watcher.yml) checks GitHub
releases for Git sources and PyPI for Python packages twice daily, with manual
dispatch also available. It prepares pin-update PRs for review.
[Standard Corpus](../.github/workflows/standard-corpus.yml) runs weekly or manually;
[Manual Benchmark](../.github/workflows/manual-trigger.yml) accepts a subject and
optional version. These benchmark workflows use a configured benchmark runner,
retain run artifacts and submit records to the results repository.

The release-watcher script and [pin-bump PR workflow](../.github/workflows/bench-pr.yml)
currently target `main`, while this repository uses `master`. Ask maintainers to
arrange any required benchmark run instead of assuming a pin-update PR will trigger
one. Local validation in the contributor guide does not require these workflows.
