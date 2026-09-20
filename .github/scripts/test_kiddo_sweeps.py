import json
from pathlib import Path
import subprocess
import sys
import tomllib


def test_kiddo_sweeps(binary):
    root = Path(__file__).resolve().parents[2]
    manifest = tomllib.loads((root / "subjects/kiddo/subject.toml").read_text())
    for case in manifest["case"]:
        if case["driver"] == "v5":
            assert all(case["tags"][k] == v for k, v in manifest["defaults"].items())
    common = ("impl=kiddo,axis=f64,query=exact_nn,k=1,dims=3,idx=u32,"
              "parallelism=single_threaded,query_batching=single_query,"
              "dataset=uniform,isa=native,kiddo.storage=heap,kiddo.block_height=3,"
              "tree_size=2^16..2^24")
    baseline = {"kiddo.bucket": 32, "kiddo.leaf": "vec_of_arenas",
                "kiddo.stem": "eytzinger"}

    def points(selector):
        return json.loads(subprocess.check_output([
            binary, "list", "--subjects", str(root / "subjects"),
            "--format", "json", "--select", selector], text=True))

    for key, values in [
        ("kiddo.bucket", [16, 32, 64, 128, 256]),
        ("kiddo.leaf", manifest["vocab"]["leaf"]["values"]),
        ("kiddo.stem", manifest["vocab"]["stem"]["values"]),
    ]:
        selection = common + "," + ",".join(
            f"{k}={v}" for k, v in baseline.items() if k != key)
        selected = points(selection)
        assert len(selected) == len(values) * 9
        assert {p[key] for p in selected} == set(values)
        for value in values:
            assert {p["tree_size"] for p in selected if p[key] == value} == {
                2**n for n in range(16, 25)}
        for point in selected:
            assert point["query_count"] == 1000
            assert point["metric"] == "squared_euclidean"
            assert point["defaults_or_tuned"] == (
                "default" if point[key] == baseline[key] else "tuned")
            assert all(point[k] == v for k, v in baseline.items() if k != key)

    standard = points(common + ",defaults_or_tuned=default")
    assert len(standard) == 9
    assert all(all(p[k] == v for k, v in baseline.items()) for p in standard)


if __name__ == "__main__":
    test_kiddo_sweeps(sys.argv[1] if len(sys.argv) > 1 else "spatial-bench")
    print("Kiddo sweep coverage and default labels passed")
