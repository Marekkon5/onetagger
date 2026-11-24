"""Build, run and summarize the production scheduler on GitHub runners."""
import csv
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess


def main():
    from provider_requests import main as request_benchmarks
    request_benchmarks()
    output = Path("provider-benchmark-results")
    output.mkdir(exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    tests = (output / f"scheduler-tests{suffix}").resolve()
    binary = (output / f"provider-benchmark{suffix}").resolve()
    subprocess.run(["rustc", "--edition=2021", "--test", "-D", "warnings",
                    "crates/onetagger-autotag/src/scheduler.rs", "-o", str(tests)], check=True)
    result = subprocess.run([str(tests), "--nocapture"], text=True, capture_output=True)
    (output / "tests.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    print(result.stdout, flush=True)
    if result.returncode:
        print(result.stderr, flush=True)
        result.check_returncode()
    subprocess.run(["rustc", "--edition=2021", "-O", "-D", "warnings",
                    "ci/provider_benchmark.rs", "-o", str(binary)], check=True)
    result = subprocess.run([str(binary)], text=True, capture_output=True)
    (output / "raw.csv").write_text(result.stdout, encoding="utf-8")
    (output / "benchmark.log").write_text(result.stderr, encoding="utf-8")
    result.check_returncode()
    rows = list(csv.DictReader(result.stdout.splitlines()))
    assert len(rows) == 70, "Missing benchmark samples"
    environment = {
        "commit": os.environ.get("GITHUB_SHA"), "runner_os": os.environ.get("RUNNER_OS"),
        "platform": platform.platform(), "cpu": platform.processor(), "logical_cpus": os.cpu_count(),
        "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
        "samples_per_mode": 5, "tracks": 64, "global_workers": 4,
        "measurement": "Synthetic provider waits; logical attempts, not HTTP requests or bytes",
    }
    summary = []
    for case in dict.fromkeys(row["case"] for row in rows):
        modes = {}
        for mode in ("Batched", "Pipelined"):
            samples = [row for row in rows if row["case"] == case and row["schedule"] == mode]
            modes[mode] = {key: statistics.median(float(row[key]) for row in samples)
                           for key in rows[0] if key not in ("case", "schedule", "trial")}
        assert modes["Batched"]["requests"] == modes["Pipelined"]["requests"]
        summary.append({"case": case, "modes": modes,
                        "speedup": modes["Batched"]["elapsed_ms"] / modes["Pipelined"]["elapsed_ms"]})
    (output / "summary.json").write_text(json.dumps({"environment": environment, "results": summary},
                                                   indent=2) + "\n", encoding="utf-8")
    lines = ["## Provider pipeline benchmark", "",
             "Five samples per mode, medians. Synthetic waits through the production scheduler; "
             "these are not live-provider speed or bandwidth measurements. Timing is informational; "
             "provider order, outcomes, request counts, limits and cancellation assertions are required.", "",
             "| Scenario | Batch ms | Pipeline ms | Speedup | Attempts | First result ms (batch / pipeline) | P95 ms (batch / pipeline) |",
             "|---|---:|---:|---:|---:|---:|---:|"]
    for item in summary:
        batch, pipe = item["modes"]["Batched"], item["modes"]["Pipelined"]
        lines.append(f"| {item['case']} | {batch['elapsed_ms']:.1f} | {pipe['elapsed_ms']:.1f} | "
                     f"{item['speedup']:.2f}x | {pipe['requests']:.0f} | "
                     f"{batch['first_terminal_ms']:.1f} / {pipe['first_terminal_ms']:.1f} | "
                     f"{batch['p95_terminal_ms']:.1f} / {pipe['p95_terminal_ms']:.1f} |")
    cancel = next(item for item in summary if item["case"] == "cancellation")["modes"]
    lines += ["", f"Stop-to-drain median: batch {cancel['Batched']['cancel_drain_ms']:.1f} ms; "
              f"pipeline {cancel['Pipelined']['cancel_drain_ms']:.1f} ms. "
              "Already-running calls finish; queued calls are discarded.", ""]
    markdown = "\n".join(lines)
    (output / "summary.md").write_text(markdown, encoding="utf-8")
    print(markdown, flush=True)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(markdown)


if __name__ == "__main__":
    main()
