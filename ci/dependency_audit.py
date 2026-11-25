"""Audit both lockfiles on GitHub and reject advisories introduced by a PR."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


def findings(ecosystem, data):
    if ecosystem == 'cargo':
        return {(item['package']['name'], item['advisory']['id'])
                for item in data['vulnerabilities']['list']}
    return {(item['module_name'], str(item.get('github_advisory_id') or item['id']))
            for item in data['advisories'].values()}


def audit(ecosystem, folder, output):
    command = (['cargo', 'audit', '--file', str(folder / 'Cargo.lock'), '--json']
               if ecosystem == 'cargo' else ['pnpm', '--dir', str(folder), 'audit', '--json'])
    result = subprocess.run(command, text=True, capture_output=True)
    output.write_text(result.stdout, encoding='utf-8')
    output.with_suffix('.stderr.log').write_text(result.stderr, encoding='utf-8')
    if result.returncode not in (0, 1):
        raise RuntimeError(f'{ecosystem} audit failed: {result.stderr}')
    data = json.loads(result.stdout)
    # A registry/database error must fail this job, rather than look like zero findings.
    if data.get('error') or (ecosystem == 'pnpm' and 'metadata' not in data):
        raise RuntimeError(f'{ecosystem} audit returned an invalid report')
    return findings(ecosystem, data)


def main():
    root = Path.cwd().resolve()
    output = root / 'dependency-audit-results'
    output.mkdir(exist_ok=True)
    base = os.environ['AUDIT_BASE_SHA']
    report = ['# Dependency advisory comparison', '',
              f'Baseline commit: `{base}`. Reports include existing advisories; the gate rejects new ones.', '',
              '| Lockfile | Baseline | Current | Introduced | Resolved |',
              '| --- | ---: | ---: | ---: | ---: |']
    introduced = []
    remaining = []
    with tempfile.TemporaryDirectory() as directory:
        baseline = Path(directory)
        (baseline / 'client').mkdir()
        for name in ('Cargo.lock', 'client/package.json', 'client/pnpm-lock.yaml'):
            result = subprocess.run(['git', 'show', f'{base}:{name}'], check=True, capture_output=True)
            (baseline / name).write_bytes(result.stdout)
        for ecosystem, current, previous in [('cargo', root, baseline), ('pnpm', root / 'client', baseline / 'client')]:
            before = audit(ecosystem, previous, output / f'{ecosystem}-baseline.json')
            after = audit(ecosystem, current, output / f'{ecosystem}-current.json')
            added = after - before
            introduced.extend((ecosystem, *item) for item in sorted(added))
            remaining.extend((ecosystem, *item) for item in sorted(after))
            report.append(f'| {ecosystem} | {len(before)} | {len(after)} | {len(added)} | {len(before - after)} |')
    report += ['', '## Current advisories', '']
    report += [f'- {ecosystem}: `{package}` — {advisory}' for ecosystem, package, advisory in remaining] or ['None reported.']
    summary = '\n'.join(report) + '\n'
    (output / 'report.md').write_text(summary, encoding='utf-8')
    print(summary, flush=True)
    if os.environ.get('GITHUB_STEP_SUMMARY'):
        with open(os.environ['GITHUB_STEP_SUMMARY'], 'a', encoding='utf-8') as handle:
            handle.write(summary)
    if introduced:
        raise SystemExit(f'New dependency advisories: {introduced}')


if __name__ == '__main__':
    main()
