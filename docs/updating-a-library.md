# Update an existing entry

Start from the manifest and exact upstream revision. State whether the change
updates software, broadens coverage or corrects measurement; these have different
consequences for interpreting old results.

| Change | Review and validation |
| --- | --- |
| Compatible release | Update pin and SHA where applicable; check resolution, conformance, query answers and a small run. |
| New query/configuration | Add implemented cases; check vocabulary, timed work, labels, defaults and corpus selection. |
| Changed API | Adapt within a proven compatibility range, or add a driver generation with separate cases and bounds. |
| Corrected measurement | Explain the error and affected runs; coordinate results review and methodology changes. |

## Change a pin

Resolve a new Git release tag to its commit and update both `pinned_ref` and
`sha`. For PyPI, update the exact version and any explicit distribution SHA for
the artifact being installed. Never carry forward a previous release's hash.
Read the [source guidance](adding-a-library.md#describe-the-source-and-compatible-driver)
and retain output showing the resolved revision or artifact.

The driver supported-version interval must contain the new version. A compatible
API can stay in the same driver with tests for the retained range. An incompatible
API should get a new driver when old versions must remain runnable; Kiddo's
`driver/` and `driver-v5/` demonstrate this. Keep ranges unambiguous and case
references explicit. A pin-only change can otherwise select no compatible cases.

Run the [small validation workflow](../CONTRIBUTING.md#validate-a-small-selection)
with supported tags for your subject. Performance comparisons need matching
workloads, build conditions and machine state, with both revisions retained.
A single smoke run does not establish a regression or improvement.

## Extend or correct cases

Review compile-time versus runtime parameters before adding matrix dimensions.
Use `list` for selector expansion and `conform` for registration coverage. Check
new APIs against known answers. Shared query or dataset names can require a
coordinated [core extension](https://github.com/spatial-bench/spatial-bench-core/blob/main/CONTRIBUTING.md).

Review `[defaults]` when changing constructor options, batching, flags or
parallelism. Classification follows declarations and can change without changing
the algorithm. Explain why a default represents the upstream configuration.

The [standard corpus](../corpus.toml) selects uniform 3D inputs, default
configurations, both scalar types and four query families. A subject runs its
supported subset. Explicit tree-size ranges expand; unconstrained runtime
parameters retain defaults. Preview selections instead of assuming every
parameter is swept. Radius descriptions mentioning approximately 100 results are
design intent; see the radius/metric limitation in the
[methodology](https://spatial-bench.org/methodology).

For a measurement correction, identify affected runs and explain whether they
need exclusion, removal or replacement under maintainer review. Follow the
[results guide](https://github.com/spatial-bench/spatial-bench-results/blob/main/CONTRIBUTING.md);
do not relabel historical measurements as if the corrected adapter produced them.

## Maintainer automation

These workflows publish changes or results; they are not local contributor checks.

- [Release Watcher](../.github/workflows/release-watcher.yml) checks twice daily
  at 06:00 and 18:00 UTC. Its [script](../.github/scripts/check_releases.py) checks
  GitHub releases and PyPI and proposes pins; it does not implement a general
  crates.io watcher. Review source resolution and compatibility independently.
- [Bench Pin Bump](../.github/workflows/bench-pr.yml) selects the first changed
  subject, attempts a small run, uploads documents and comments a metric table.
  It does not compute a verified old-versus-new comparison. It targets/diffs
  `main`, while this repository's default branch is `master`; the watcher also
  assumes `main`. Do not rely on these as working gates for PRs to `master`.
  The PR job also lacks an explicit dataset-generator build and its comment
  step searches for filenames containing the subject, which current run-document
  filenames do not contain.
- [Standard Corpus](../.github/workflows/standard-corpus.yml) runs weekly and on
  dispatch on a self-hosted benchmark runner. Its configurable subject matrix
  defaults to Kiddo, nanoflann and pykdtree; a catalog entry alone does not add a
  library to that matrix. It builds both engine binaries and submits supported
  corpus selections to the results repository.
- [Manual benchmark](../.github/workflows/manual-trigger.yml) can override subject
  versions and run/submit corpus measurements. It is not required to open a PR.

Verified against benchers `1972c50` and core `2104bfe` on 2026-09-14. Recheck
workflow branch assumptions and source-resolution behaviour when changing them.
