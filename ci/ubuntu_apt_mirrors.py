"""Rank Ubuntu archive mirrors by transfer speed on the GitHub Actions runner."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import platform
import re
import subprocess


SOURCES = Path("/etc/apt/sources.list.d/ubuntu.sources")
MIRROR_LIST = Path("/etc/apt/onetagger-ubuntu-mirrors.list")
ARCHIVE_URI = re.compile(
    r"^URIs:[ \t]+(https?://(?:[a-z0-9.-]+\.)?archive\.ubuntu\.com/ubuntu/?)[ \t]*$",
    re.MULTILINE,
)
ALTERNATIVES = (
    "https://archive.ubuntu.com/ubuntu",
    "https://mirrors.edge.kernel.org/ubuntu",
    "https://mirror.math.princeton.edu/pub/ubuntu",
)
PROBE = "dists/noble/main/binary-amd64/Packages.xz"


def probe_speed(mirror):
    """Measure a complete, identical archive index download in bytes per second."""
    result = subprocess.run(
        [
            "curl", "--fail", "--silent", "--connect-timeout", "3",
            "--max-time", "10", "--output", "/dev/null", "--write-out",
            "%{http_code} %{size_download} %{speed_download}",
            f"{mirror}/{PROBE}",
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return 0
    try:
        code, size, speed = result.stdout.split()
        if code == "200" and int(size) >= 1024 * 1024:
            return int(float(speed))
    except (ValueError, OverflowError):
        pass
    return 0


def mirror_list(original, speeds):
    """Give APT measured mirrors in speed order and keep the runner's original."""
    ordered = sorted(
        (mirror for mirror, speed in speeds.items() if speed > 0),
        key=speeds.__getitem__,
        reverse=True,
    )
    if original not in ordered:
        ordered.append(original)
    return "".join(f"{mirror}/\tpriority:{i}\n" for i, mirror in enumerate(ordered, 1))


def main():
    if (platform.freedesktop_os_release().get("VERSION_CODENAME") != "noble"
            or platform.machine() != "x86_64" or not SOURCES.is_file()):
        print("Keeping the default APT sources on this runner.")
        return
    source = SOURCES.read_text(encoding="utf-8")
    matches = list(ARCHIVE_URI.finditer(source))
    if len(matches) != 1:
        print("Keeping the default APT sources: expected one Ubuntu archive URI.")
        return

    archive = matches[0]
    original = archive.group(1).rstrip("/")
    candidates = tuple(dict.fromkeys((original, *ALTERNATIVES)))
    with ThreadPoolExecutor(max_workers=len(candidates)) as pool:
        speeds = dict(zip(candidates, pool.map(probe_speed, candidates)))

    for mirror, speed in speeds.items():
        result = f"{speed / 1024:,.0f} KiB/s" if speed else "probe did not finish"
        print(f"Ubuntu mirror {mirror}: {result}", flush=True)
    if not any(speeds.get(mirror, 0) for mirror in ALTERNATIVES if mirror != original):
        print("Keeping the default APT archive: no alternate mirror passed the probe.")
        return

    ranked = mirror_list(original, speeds)
    # Change only the archive URI. Keep the original suites, components,
    # archive signing key, and the separate security repository.
    updated = source[:archive.start()] + f"URIs: mirror+file:{MIRROR_LIST}" + source[archive.end():]
    for path, contents in ((MIRROR_LIST, ranked), (SOURCES, updated)):
        subprocess.run(
            ["sudo", "tee", str(path)], input=contents, text=True,
            stdout=subprocess.DEVNULL, check=True,
        )
    print("APT archive mirror priority:\n" + ranked, flush=True)


if __name__ == "__main__":
    main()
