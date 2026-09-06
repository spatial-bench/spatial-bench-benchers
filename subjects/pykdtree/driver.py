#!/usr/bin/env python3
"""The pykdtree exec driver for spatial-bench (day 1.5).

The harness contract (core/src/harness.rs) is the only interface, identical
for every subject regardless of language:
  - `--list` prints the registration list (§13) and reads no stdin;
  - otherwise a RunSpec arrives as JSON on stdin and JSONL points leave on
    stdout; diagnostics go to stderr.

The engine owns the harness: the seeds come from the spec, the data generator
below is numpy-vectorized ChaCha8 **byte-exact** with the rust drivers'
generator (self-checked at startup against pinned vectors — a rand upgrade
that changes the stream makes this driver refuse rather than measure), and the
timed region is exactly the per-probe query loop. pykdtree contributes its
public API and nothing else.

One deviation from the C++ shim, documented here: tag values are echoed
value-faithfully through json round-tripping rather than text-faithfully — an
int tag stays `1048576` and a string tag stays `"f64"`, but a float tag's
spelling may differ while its value does not.
"""

import json
import subprocess
import sys
import time

import numpy as np

HARNESS_VERSION = 2
MASK32 = 0xFFFFFFFF
MASK64 = 0xFFFFFFFFFFFFFFFF

# Self-check vectors: the first two f64 points of the rust drivers' generator
# for the contract's point seed (see the shim's startup check — same vectors).
SELFTEST_SEED = 0x5EED000000000301
# Point 0 element 0, and point 1 element 1 — the same positions the C++ shim
# checks (rust ground truth from the bencher drift vectors).
SELFTEST_BITS = [(0, 0, 0x3FC0EFC570511B9C), (1, 1, 0x3FCBA6B42D1D3AD4)]

CONSTANTS = (0x61707865, 0x3320646E, 0x79622D32, 0x6B206574)


# ---------------------------------------------------------------------------
# The generator: ChaCha8 + rand conversions, numpy-vectorized over blocks.
# ---------------------------------------------------------------------------

def _seed_words(state: int) -> np.ndarray:
    """rand_core's `seed_from_u64`: a PCG32 stream fills the 32-byte seed,
    4 bytes at a time, little-endian."""
    MUL = 6364136223846793005
    INC = 11634580027462260723
    seed = bytearray(32)
    for chunk in range(8):
        state = (state * MUL + INC) & MASK64
        xorshifted = (((state >> 18) ^ state) >> 27) & MASK32
        rot = (state >> 59) & 31
        x = ((xorshifted >> rot) | (xorshifted << ((32 - rot) & 31))) & MASK32
        seed[chunk * 4 : chunk * 4 + 4] = x.to_bytes(4, "little")
    return np.frombuffer(bytes(seed), dtype="<u4").copy()


def _rotl(x: np.ndarray, n: int) -> np.ndarray:
    return ((x << n) | (x >> (32 - n))) & MASK32


def _quarter(w: np.ndarray, a: int, b: int, c: int, d: int) -> None:
    A, B, C, D = w[:, a], w[:, b], w[:, c], w[:, d]
    A = A + B
    D = D ^ A
    D = _rotl(D, 16)
    C = C + D
    B = B ^ C
    B = _rotl(B, 12)
    A = A + B
    D = D ^ A
    D = _rotl(D, 8)
    C = C + D
    B = B ^ C
    B = _rotl(B, 7)
    w[:, a], w[:, b], w[:, c], w[:, d] = A, B, C, D


def stream_words(seed: int, count: int) -> np.ndarray:
    """The first `count` u32 words of the ChaCha8 stream seeded with `seed`:
    64-byte blocks (16 words each), counter from zero, words in order — the
    same stream rand_chacha's BlockRng consumes."""
    key = _seed_words(seed)
    nblocks = (count + 15) // 16
    counters = np.arange(nblocks, dtype="<u8")
    st = np.empty((nblocks, 16), dtype="<u4")
    st[:, 0:4] = CONSTANTS
    st[:, 4:12] = key
    st[:, 12] = (counters & MASK32).astype("<u4")
    st[:, 13] = (counters >> 32).astype("<u4")
    st[:, 14:16] = 0
    w = st.copy()
    # 8 rounds = 4 double rounds (4 column quarter-rounds + 4 diagonal).
    for _ in range(4):
        _quarter(w, 0, 4, 8, 12)
        _quarter(w, 1, 5, 9, 13)
        _quarter(w, 2, 6, 10, 14)
        _quarter(w, 3, 7, 11, 15)
        _quarter(w, 0, 5, 10, 15)
        _quarter(w, 1, 6, 11, 12)
        _quarter(w, 2, 7, 8, 13)
        _quarter(w, 3, 4, 9, 14)
    out = w + st
    return out.reshape(-1)[:count]


def generate(count: int, seed: int, dims: int, dtype) -> np.ndarray:
    """The harness generator: one uniform point per step, elements filled in
    order — byte-identical to the rust drivers' `generate(count, seed)` for
    the same dtype and dimensionality."""
    f64 = dtype is np.float64
    words_per_point = dims * (2 if f64 else 1)
    words = stream_words(seed, count * words_per_point)
    if f64:
        # next_u64 assembles two consecutive words little-endian; the value
        # is (u64 >> 11) * 2^-53.
        pairs = words.reshape(-1, 2)
        u64 = pairs[:, 0].astype("<u8") | (pairs[:, 1].astype("<u8") << 32)
        vals = ((u64 >> 11).astype(np.float64)) * (2.0**-53)
    else:
        vals = ((words >> 8).astype(np.float32)) * (np.float32(2.0) ** np.float32(-24))
    return vals.reshape(count, dims)


def selfcheck() -> None:
    got = generate(2, SELFTEST_SEED, 3, np.float64)
    bits = {
        (i, d): int.from_bytes(np.float64(v).tobytes(), "little")
        for i, p in enumerate(got)
        for d, v in enumerate(p)
    }
    if any(bits[(i, d)] != want for i, d, want in SELFTEST_BITS):
        sys.stderr.write(
            "pykdtree driver: the data generator disagrees with the rust "
            f"drivers {bits!r} — refusing to measure (a rand upgrade that "
            "changed the stream must land everywhere at once)\n"
        )
        sys.exit(6)


# ---------------------------------------------------------------------------
# The contract: --list, spec parsing, measurement, point emission.
# ---------------------------------------------------------------------------

def run_list() -> int:
    # pykdtree dispatches at run time (axis, k, leafsize) and declares no
    # compile-time tag keys: one registration serves every case.
    print(json.dumps({"compile_time": []}))
    return 0


def fail(message: str) -> None:
    sys.stderr.write(f"pykdtree driver: {message}\n")
    sys.exit(5)


def main() -> int:
    selfcheck()
    if len(sys.argv) > 1 and sys.argv[1] == "--list":
        return run_list()

    try:
        from pykdtree.kdtree import KDTree
    except ImportError as e:  # pragma: no cover
        fail(f"pykdtree is not installed in this environment: {e}")

    spec = json.load(sys.stdin)
    if spec.get("harness_version") != HARNESS_VERSION:
        fail("spec is not harness version 1")
    budget = spec["budget"]
    warm_up_ms = float(budget["warm_up_ms"])
    measurement_ms = float(budget["measurement_ms"])
    sample_size = int(budget["sample_size"])

    for case in spec["cases"]:
        tags = case["tags"]
        tree_size = int(tags["tree_size"])
        query_count = int(tags["query_count"])
        k = int(tags["k"])
        dims = int(tags["dims"])
        leafsize = int(tags["pykdtree.leafsize"])
        axis = tags["axis"]
        query_kind = tags["query"]
        batching = tags.get("query_batching", "single_query")
        parallelism = tags.get("parallelism", "single_threaded")
        batch_size = max(1, int(tags.get("query_batch_size", query_count)))
        if query_kind != "exact_nn":
            fail(f"query kind `{query_kind}` is not implemented")
        if batching not in ("single_query", "batch_query"):
            fail(f"unknown query_batching `{batching}`")
        if parallelism != "single_threaded":
            fail("pykdtree declares no multi_threaded executor")
        if axis not in ("f32", "f64"):
            fail(f"unknown axis `{axis}`")
        dtype = np.float64 if axis == "f64" else np.float32

        # Harness v2: points come from the engine's dataset generator binary.
        # Header (native endian): SBDS + u32 version + u32 dims + u8 dtype +
        # u64 tree_count (13..21) + u64 query_count (21..29); body from byte
        # 29 is tree points then query points, dims-contiguous.
        proc = subprocess.run(
            [
                case["dataset_generator"],
                "--kind", case["dataset"],
                "--dims", str(dims),
                "--dtype", axis,
                "--tree-count", str(tree_size),
                "--query-count", str(query_count),
                "--seed", str(case["random_seed"]),
            ],
            stdout=subprocess.PIPE,
            check=True,
        )
        raw = proc.stdout
        tree_count = int.from_bytes(raw[13:21], sys.byteorder)
        data = np.frombuffer(raw, dtype=dtype, offset=29)
        points = np.ascontiguousarray(data[: tree_count * dims]).reshape(
            tree_count, dims
        )
        probes = np.ascontiguousarray(data[tree_count * dims :]).reshape(
            query_count, dims
        )

        tree = KDTree(np.ascontiguousarray(points), leafsize=leafsize)

        def body() -> int:
            checksum = 0
            if batching == "single_query":
                for probe in probes:
                    # One query per probe — the same operation shape the rust
                    # drivers' query loop performs (§11's comparability model).
                    # pykdtree squeezes the result for single-point queries, so
                    # index the ravelled form for either shape.
                    _, idx = tree.query(probe.reshape(1, -1).astype(dtype), k=k)
                    checksum = (checksum + int(np.asarray(idx).ravel()[k - 1])) & MASK64
            else:
                # pykdtree's query API is batch-native: one call takes the
                # whole chunk matrix. query_batch_size paces the call size.
                for start in range(0, len(probes), batch_size):
                    chunk = np.ascontiguousarray(
                        probes[start : start + batch_size], dtype=dtype
                    )
                    _, idx = tree.query(chunk, k=k)
                    checksum = (
                        checksum + int(np.asarray(idx).astype(np.int64).sum())
                    ) & MASK64
            return checksum

        warm_start = time.perf_counter_ns()
        while (time.perf_counter_ns() - warm_start) / 1e6 < warm_up_ms:
            body()

        samples = []
        measure_start = time.perf_counter_ns()
        while len(samples) < sample_size and (
            (time.perf_counter_ns() - measure_start) / 1e6
        ) < measurement_ms:
            t0 = time.perf_counter_ns()
            body()
            samples.append((time.perf_counter_ns() - t0) / query_count)
        if not samples:
            fail("measurement produced no samples")

        samples.sort()
        n = len(samples)
        mean = sum(samples) / n
        variance = sum((s - mean) ** 2 for s in samples) / n
        sd = variance**0.5
        median = samples[n // 2]
        mad = sorted(abs(s - median) for s in samples)[n // 2]
        half = 1.96 * sd / n**0.5

        point = {
            "tags": tags,
            "metrics": {
                "latency_ns": {
                    "point": mean,
                    "lower": mean - half,
                    "upper": mean + half,
                    "unit": "ns/query",
                },
                "throughput_qps": {"point": 1e9 / mean, "unit": "queries/s"},
            },
            "stats": {
                "samples": n,
                "ci": 0.95,
                "std_dev_ns": sd,
                "median_ns": median,
                "mad_ns": mad,
            },
        }
        print(json.dumps(point), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
