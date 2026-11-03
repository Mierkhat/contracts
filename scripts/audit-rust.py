#!/usr/bin/env python3
"""Audit every locked Rust dependency; enforce a narrow native-host exception."""
from datetime import date
import json
from pathlib import Path
import subprocess


def validate(report, policy, graphs, today=None):
    today = today or date.today()
    if today >= date.fromisoformat(policy['expires']):
        raise SystemExit('RustSec native-host exception expired; update upstream or review policy')
    expected = ('RUSTSEC-2026-0387', 'crypto-bigint', '0.5.5')
    if (policy['advisory'], policy['package'], policy['version']) != expected:
        raise SystemExit('Unexpected RustSec exception scope')
    if len(graphs) != 2:
        raise SystemExit("Both deployed contract dependency graphs are required")
    for graph in graphs:
        if 'crypto-bigint ' in graph or 'soroban-env-host ' in graph:
            raise SystemExit('Native-only advisory dependency present in deployed WASM graph')
    vulnerabilities = report['vulnerabilities']['list']
    if report['vulnerabilities']['count'] != len(vulnerabilities):
        raise SystemExit('Inconsistent RustSec report')
    for entry in vulnerabilities:
        actual = (entry['advisory']['id'], entry['package']['name'], entry['package']['version'])
        if actual != expected:
            raise SystemExit(f'Unexcepted vulnerability: {actual}')
    if report.get('warnings', {}).get('unsound'):
        raise SystemExit('RustSec unsoundness finding requires resolution')
    if not vulnerabilities:
        raise SystemExit('Advisory no longer present: remove obsolete exception')


if __name__ == '__main__':
    policy = json.loads(Path('.github/rustsec-native-host.json').read_text())
    graphs = [subprocess.check_output(['cargo', '+1.93.0', 'tree', '--locked', '-p', package, '--target', 'wasm32v1-none', '--edges', 'normal', '--prefix', 'none'], text=True) for package in ('mierkhat-guard', 'mierkhat-migration')]
    process = subprocess.run(['cargo', 'audit', '--file', 'Cargo.lock', '--json'], text=True, capture_output=True)
    if process.returncode not in (0, 1):
        raise SystemExit(process.stderr)
    report = json.loads(process.stdout)
    validate(report, policy, graphs)
    print(json.dumps(report, indent=2))
    print('PASS: RustSec audit; exact temporary native-host advisory absent from both deployed WASM dependency graphs')
