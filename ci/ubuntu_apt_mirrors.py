"""Rank Ubuntu archive mirrors by transfer speed on the GitHub Actions runner."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import platform
import re
import subprocess


SOURCES = Path("/etc/apt/sources.list.d/ubuntu.sources")
MIRROR_LIST = Path("/etc/apt/onetagger-ubuntu-mirrors.list")
RUNNER_MIRROR_LIST = Path("/etc/apt/apt-mirrors.txt")
RUNNER_MIRROR_URI = f"mirror+file:{RUNNER_MIRROR_LIST}"
UBUNTU_ARCHIVE = r"https?://(?:[a-z0-9.-]+\.)?archive\.ubuntu\.com/ubuntu/?"
ARCHIVE_URI = re.compile(
    rf"^URIs:[ \t]+({UBUNTU_ARCHIVE}|{re.escape(RUNNER_MIRROR_URI)})[ \t]*$",
    re.MULTILINE,
)
RUNNER_MIRROR = re.compile(
    rf"({UBUNTU_ARCHIVE}|https?://security\.ubuntu\.com/ubuntu/?)(?:[ \t]+priority:(\d+))?",
)
ARCHIVE_SUITES = {"noble", "noble-updates", "noble-backports"}
SUITES = re.compile(r"^Suites:[ \t]+([^\n]+)$", re.MULTILINE)
ALTERNATIVES = (
    "http://azure.archive.ubuntu.com/ubuntu",
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


def runner_mirrors(contents):
    """Read the runner image's Ubuntu archives in their configured order."""
    mirrors = {}
    for line in contents.splitlines():
        match = RUNNER_MIRROR.fullmatch(line.strip())
        if match:
            uri, priority = match.groups()
            mirrors.setdefault(uri.rstrip("/"), int(priority) if priority else 2**31)
    return sorted(mirrors, key=mirrors.__getitem__)


def mirror_list(fallbacks, speeds):
    """Give APT measured mirrors in speed order, then the runner's fallbacks."""
    ordered = sorted(
        (mirror for mirror, speed in speeds.items() if speed > 0),
        key=speeds.__getitem__,
        reverse=True,
    )
    ordered.extend(mirror for mirror in fallbacks if mirror not in ordered)
    return "".join(f"{mirror}/\tpriority:{i}\n" for i, mirror in enumerate(ordered, 1))


def archive_settings(source):
    """Rewrite only regular Noble archive stanzas, even if security uses the same URI."""
    blocks = re.split(r"(\n[ \t]*\n)", source)
    originals = set()
    for index in range(0, len(blocks), 2):
        block = blocks[index]
        suites = SUITES.search(block)
        if not suites or not set(suites.group(1).split()) <= ARCHIVE_SUITES:
            continue
        matches = list(ARCHIVE_URI.finditer(block))
        if len(matches) != 1:
            continue
        originals.add(matches[0].group(1).rstrip("/"))
        blocks[index] = ARCHIVE_URI.sub(f"URIs: mirror+file:{MIRROR_LIST}", block, count=1)
    if len(originals) == 1:
        return originals.pop(), "".join(blocks)
    return None


def main():
    if (platform.freedesktop_os_release().get("VERSION_CODENAME") != "noble"
            or platform.machine() != "x86_64" or not SOURCES.is_file()):
        print("Keeping the default APT sources on this runner.")
        return
    source = SOURCES.read_text(encoding="utf-8")
    configuration = archive_settings(source)
    if configuration is None:
        print("Keeping the default APT sources: no single regular Noble archive was identified.")
        return
    original, updated = configuration
    if original == RUNNER_MIRROR_URI:
        fallbacks = runner_mirrors(RUNNER_MIRROR_LIST.read_text(encoding="utf-8"))
        if not fallbacks:
            print("Keeping the runner mirror list: no recognized Ubuntu archive was found.")
            return
    else:
        fallbacks = [original]
    archives = [mirror for mirror in fallbacks if re.fullmatch(UBUNTU_ARCHIVE, mirror)]
    candidates = tuple(dict.fromkeys((*archives, *ALTERNATIVES)))
    with ThreadPoolExecutor(max_workers=len(candidates)) as pool:
        speeds = dict(zip(candidates, pool.map(probe_speed, candidates)))

    for mirror, speed in speeds.items():
        result = f"{speed / 1024:,.0f} KiB/s" if speed else "probe did not finish"
        print(f"Ubuntu mirror {mirror}: {result}", flush=True)
    if not any(speeds.values()):
        print("Keeping the default APT archive: no mirror passed the probe.")
        return

    ranked = mirror_list(fallbacks, speeds)
    # Change only the archive URI. Keep the original suites, components,
    # archive signing key, and the separate security repository.
    for path, contents in ((MIRROR_LIST, ranked), (SOURCES, updated)):
        subprocess.run(
            ["sudo", "tee", str(path)], input=contents, text=True,
            stdout=subprocess.DEVNULL, check=True,
        )
    print("APT archive mirror priority:\n" + ranked, flush=True)


if __name__ == "__main__":
    main()
