"""Remove only this repository's closed-PR Rust caches; preserve open PRs/master."""
import json
import os
import re
from urllib.error import HTTPError
from urllib.request import Request, urlopen


def cache_pr(cache):
    if not cache.get("key", "").startswith("v1-onetagger-deps-"):
        return None
    match = re.fullmatch(r"refs/pull/([1-9][0-9]*)/merge", cache.get("ref", ""))
    return int(match[1]) if match else None


def eligible(cache, closed, selected=None):
    number = cache_pr(cache)
    return number in closed and (selected is None or number == selected)


def main():
    repository = os.environ["GITHUB_REPOSITORY"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid repository")
    selected = int(os.environ["CLOSED_PR"]) if os.environ.get("CLOSED_PR") else None
    dry_run = os.environ.get("DRY_RUN", "true") != "false"

    def api(path, method="GET"):
        request = Request(
            f"https://api.github.com/repos/{repository}/{path}", method=method,
            headers={"Authorization": f"Bearer {os.environ['GH_TOKEN']}",
                     "Accept": "application/vnd.github+json",
                     "X-GitHub-Api-Version": "2022-11-28"})
        with urlopen(request, timeout=30) as response:
            data = response.read()
            return json.loads(data) if data else None

    # Snapshot all pages before deleting: deleting while paging would skip entries.
    caches = []
    page = 1
    while True:
        batch = api(f"actions/caches?per_page=100&page={page}")["actions_caches"]
        caches.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    numbers = {cache_pr(c) for c in caches} - {None}
    if selected is not None:
        numbers &= {selected}
    # Read PR state from GitHub even for a close event; fail closed on API errors.
    closed = {n for n in numbers if api(f"pulls/{n}")["state"] == "closed"}
    count, size = 0, 0
    for cache in caches:
        if not eligible(cache, closed, selected):
            continue
        cache_id = int(cache["id"])
        print(f"{'Would delete' if dry_run else 'Deleting'} cache {cache_id} ({cache['ref']})")
        if not dry_run:
            try:
                api(f"actions/caches/{cache_id}", "DELETE")
            except HTTPError as error:
                if error.code != 404:  # An eviction may win the race.
                    raise
        count += 1
        size += cache.get("size_in_bytes", 0)
    summary = f"{'Eligible' if dry_run else 'Removed'} {count} closed-PR Rust caches ({size / 2**30:.2f} GiB)."
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write(summary + "\n")


if __name__ == "__main__":
    main()
