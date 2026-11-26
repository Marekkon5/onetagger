"""Validate and measure production request reuse on GitHub runners only."""
import csv
import json
import os
from pathlib import Path
import statistics
import subprocess


def main():
    output = Path("provider-benchmark-results")
    output.mkdir(exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    for module in ("request_cache", "request_pacer"):
        binary = (output / (module + "-tests" + suffix)).resolve()
        subprocess.run(["rustc", "--edition=2021", "--test", "-D", "warnings",
                        f"crates/onetagger-platforms/src/{module}.rs", "-o", str(binary)], check=True)
        result = subprocess.run([str(binary), "--nocapture"], text=True, capture_output=True)
        (output / (module + ".log")).write_text(result.stdout + result.stderr, encoding="utf-8")
        print(result.stdout, flush=True)
        if result.returncode:
            print(result.stderr, flush=True)
            result.check_returncode()
    binary = (output / ("request-benchmark" + suffix)).resolve()
    subprocess.run(["rustc", "--edition=2021", "-O", "-D", "warnings",
                    "ci/request_benchmark.rs", "-o", str(binary)], check=True)
    result = subprocess.run([str(binary)], text=True, capture_output=True)
    (output / "request-reuse.csv").write_text(result.stdout, encoding="utf-8")
    (output / "request-reuse.log").write_text(result.stderr, encoding="utf-8")
    if result.returncode:
        print(result.stderr, flush=True)
        result.check_returncode()
    rows = list(csv.DictReader(result.stdout.splitlines()))
    assert len(rows) == 30, "Missing request benchmark samples"
    summaries = []
    lines = ["## Provider release request reuse", "",
             "1,000 synthetic songs, 16 workers, five alternating samples per mode; median timings. "
             "Actual loopback HTTP requests with a 2 ms fixture response delay, through the production cache. "
             "Every per-song metadata result must equal the uncached result. "
             "This measures detail-fetch reuse, not live provider search speed or match accuracy.", "",
             "| Fixture | Uncached / cached ms | Uncached / cached detail HTTP requests |",
             "|---|---:|---:|"]
    for case in dict.fromkeys(row["case"] for row in rows):
        modes = {}
        for cached in ("false", "true"):
            samples = [row for row in rows if row["case"] == case and row["cached"] == cached]
            assert len(samples) == 5
            modes[cached] = {key: statistics.median(float(row[key]) for row in samples)
                             for key in ("elapsed_ms", "http_requests")}
        summaries.append({"case": case, "modes": modes})
        lines.append(f"| {case} | {modes['false']['elapsed_ms']:.1f} / {modes['true']['elapsed_ms']:.1f} | "
                     f"{modes['false']['http_requests']:.0f} / {modes['true']['http_requests']:.0f} |")
    lines += ["", "Unique releases still require individual requests. Search requests remain unchanged. "
              "A ten-song release can share one detail response; the 90% detail-request reduction is "
              "not a promise of a 90% reduction in whole-run time.", ""]
    markdown = "\n".join(lines)
    (output / "request-reuse.md").write_text(markdown, encoding="utf-8")
    (output / "request-reuse.json").write_text(json.dumps({"commit": os.environ.get("GITHUB_SHA"),
        "runner_os": os.environ.get("RUNNER_OS"), "measurement": "loopback HTTP fixture", "results": summaries},
        indent=2) + "\n", encoding="utf-8")
    print(markdown, flush=True)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(markdown)


if __name__ == "__main__":
    main()
