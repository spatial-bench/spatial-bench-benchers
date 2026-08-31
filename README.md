# spatial-bench-benchers

The reviewed catalog of benchmark subjects for the
[spatial-bench](https://github.com/sdd/spatial-bench) engine: manifests,
drivers and shims, PR-gated — never the subjects' own repos (design §4's
trust model, relocated).

Every subject directory is self-contained: `subject.toml` plus its driver
assets beside it (`driver/` for rust codegen drivers, `shim.cpp` / `driver.py`
for exec subjects).

Version bumps of a subject's library land here as PRs from the release-watcher
workflow (crates.io / GitHub releases / PyPI), each carrying the new
`pinned_ref` and sha so the engine can enforce the pin at build time.
