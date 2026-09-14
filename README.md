# spatial-bench-benchers

The library catalog for [spatial-bench](https://spatial-bench.org): reviewed
manifests, source pins and benchmark adapters. Each directory under `subjects/`
contains a `subject.toml` and the driver assets needed to exercise that library.
The upstream libraries remain in their own repositories.

Library authors can [add a library](docs/adding-a-library.md) or
[update an existing entry](docs/updating-a-library.md). Start with
[contributor setup and validation](CONTRIBUTING.md); the
[driver contract](docs/driver-contract.md) explains what a measurement includes.

The current catalog contains [Kiddo](subjects/kiddo/subject.toml),
[kdtree](subjects/kdtree/subject.toml), [nanoflann](subjects/nanoflann/subject.toml)
and [pykdtree](subjects/pykdtree/subject.toml). A manifest declares driver
coverage; it does not establish that every configuration has published results.
Use the [explorer](https://spatial-bench.org/explore) and
[coverage guide](https://spatial-bench.org/coverage) to inspect measurements.

| Repository | Responsibility |
| --- | --- |
| [Core](https://github.com/spatial-bench/spatial-bench-core) | Selection, generation, measurement, run documents and collation |
| This repository | Library manifests, adapters, pins and corpus selections |
| [Results](https://github.com/spatial-bench/spatial-bench-results) | Recorded runs, machines and snapshot publication |
| [Web](https://github.com/spatial-bench/spatial-bench-web) | Explorer and public Markdown documentation |

For interpretation, see the [reading guide](https://spatial-bench.org/guide) and
[methodology](https://spatial-bench.org/methodology). For new datasets or query
vocabulary, start with the [core contributor guides](https://github.com/spatial-bench/spatial-bench-core/blob/main/CONTRIBUTING.md).
