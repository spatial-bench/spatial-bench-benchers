import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap


def test_manual_select():
    root = Path(__file__).resolve().parents[2]
    workflow = (root / ".github/workflows/manual-trigger.yml").read_text()
    step = workflow.split("      - name: Run selected benchmarks\n", 1)[1]
    script = textwrap.dedent(step.split("        run: |\n", 1)[1].split("\n      - name:", 1)[0])
    corpus = [line.split('"')[1] for line in (root / "corpus.toml").read_text().splitlines()
              if line.startswith("select = ")]
    with tempfile.TemporaryDirectory() as directory:
        work = Path(directory)
        (work / "corpus.toml").write_text((root / "corpus.toml").read_text())
        binary = work / "engine/target/release/spatial-bench"
        binary.parent.mkdir(parents=True)
        binary.write_text(textwrap.dedent('''\
            #!/usr/bin/env python3
            import json, os, sys
            with open(os.environ["CALL_LOG"], "a") as log:
                log.write(json.dumps(sys.argv[1:]) + "\\n")
            if sys.argv[1] == "list":
                if os.environ.get("FAIL_LIST"):
                    sys.exit(2)
                print("1 cases, 9 points")
            '''))
        binary.chmod(0o755)
        log = work / "calls.jsonl"
        custom = "query=exact_nn,k=5|20,parallelism=multi_threaded,tree_size=2^16..2^24"
        for selector, expected in [("", corpus), (custom, [custom]),
                                   ("query=$(touch injected)", ["query=$(touch injected)"])]:
            log.write_text("")
            env = {**os.environ, "SUBJECT": "kiddo", "SELECTS": selector,
                   "GITHUB_WORKSPACE": directory, "CALL_LOG": str(log)}
            subprocess.run(["bash", "-e", "-c", script], cwd=work, env=env,
                           check=True, capture_output=True)
            calls = [json.loads(line) for line in log.read_text().splitlines()]
            assert [call[0] for call in calls] == ["list", "run"] * len(expected)
            assert [call[-1] for call in calls] == [
                f"impl=kiddo,{selection}" for selection in expected for _ in range(2)]
            assert not (work / "injected").exists()
        log.write_text("")
        result = subprocess.run(["bash", "-e", "-c", script], cwd=work,
                                env={**env, "SELECTS": custom, "FAIL_LIST": "1"},
                                capture_output=True)
        assert result.returncode != 0
        assert len(log.read_text().splitlines()) == 1


if __name__ == "__main__":
    test_manual_select()
    print("manual selector checks passed")
