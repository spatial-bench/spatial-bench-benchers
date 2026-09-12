#!/usr/bin/env python3
"""SciPy KDTree/cKDTree harness driver."""
import json
import subprocess
import sys
import time

import numpy as np

MASK64 = (1 << 64) - 1

def fail(message):
    sys.stderr.write(f"scipy driver: {message}\n")
    raise SystemExit(5)

def point(tags, samples):
    samples.sort()
    mean = sum(samples) / len(samples)
    sd = (sum((x - mean) ** 2 for x in samples) / len(samples)) ** 0.5
    half = 1.96 * sd / len(samples) ** 0.5
    return {"tags": tags, "metrics": {"latency_ns": {"point": mean, "lower": mean-half, "upper": mean+half, "unit": "ns/query"}, "throughput_qps": {"point": 1e9/mean, "unit": "queries/s"}}, "stats": {"samples": len(samples), "ci": .95, "std_dev_ns": sd, "median_ns": samples[len(samples)//2], "mad_ns": sorted(abs(x-samples[len(samples)//2]) for x in samples)[len(samples)//2]}}

def main():
    if "--list" in sys.argv:
        print('{"compile_time":[]}')
        return 0
    from scipy.spatial import KDTree, cKDTree
    spec = json.load(sys.stdin)
    if spec.get("harness_version") != 2:
        fail("unsupported harness version")
    budget = spec["budget"]
    for case in spec["cases"]:
        tags = case["tags"]
        axis = tags["axis"]
        dtype = np.float32 if axis == "f32" else np.float64
        if tags["parallelism"] != "single_threaded": fail("driver fixes workers=1")
        raw = subprocess.run([case["dataset_generator"], "--kind", case["dataset"], "--dims", str(tags["dims"]), "--dtype", axis, "--tree-count", str(tags["tree_size"]), "--query-count", str(tags["query_count"]), "--seed", str(case["random_seed"])], stdout=subprocess.PIPE, check=True).stdout
        n = int.from_bytes(raw[13:21], sys.byteorder)
        values = np.frombuffer(raw, dtype=dtype, offset=29)
        data = np.ascontiguousarray(values[:n*tags["dims"]].reshape(n, tags["dims"]))
        probes = np.ascontiguousarray(values[n*tags["dims"]:].reshape(tags["query_count"], tags["dims"]))
        cls = KDTree if tags["scipy.tree"] == "KDTree" else cKDTree
        tree = cls(data)
        k = int(tags["k"])
        def body():
            checksum = 0
            for probe in probes:
                _, ids = tree.query(probe, k=k, workers=1)
                checksum = (checksum + int(np.asarray(ids).reshape(-1)[-1])) & MASK64
            return checksum
        warm = time.perf_counter_ns()
        while (time.perf_counter_ns()-warm)/1e6 < budget["warm_up_ms"]: body()
        samples=[]; start=time.perf_counter_ns()
        while len(samples) < budget["sample_size"] and (time.perf_counter_ns()-start)/1e6 < budget["measurement_ms"]:
            t=time.perf_counter_ns(); body(); samples.append((time.perf_counter_ns()-t)/len(probes))
        if not samples: fail("no samples")
        print(json.dumps(point(tags, samples)), flush=True)
    return 0
if __name__ == "__main__": raise SystemExit(main())
