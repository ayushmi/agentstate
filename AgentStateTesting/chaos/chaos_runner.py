#!/usr/bin/env python3
import os
import sys
import time
import json
import signal
import subprocess
from pathlib import Path

import requests

from nemeses import ensure_up, ensure_down, pause, kill, fill_disk, free_disk
from workload import Workload, Watcher

ROOT = Path(__file__).resolve().parents[2]
COMPOSE = [str(ROOT / 'docker-compose.yml'), str(ROOT / 'AgentStateTesting/chaos/docker-compose.chaos.yml')]
CONTAINER = 'agentstate-chaos'
BASE_URL = os.environ.get('AGENTSTATE_BASE_URL', 'http://localhost:8080')
NAMESPACE = os.environ.get('AGENTSTATE_NS', 'chaos')


def wait_healthy(timeout=60):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            r = requests.get(f"{BASE_URL}/health", timeout=2)
            if r.status_code == 200:
                return True
        except Exception:
            pass
        time.sleep(1)
    return False


def gen_token(namespaces, verbs):
    # prefer using the helper script
    kid = os.environ.get('CAP_KEY_ACTIVE_ID', 'active')
    secret = os.environ.get('CAP_KEY_ACTIVE', 'dev-secret')
    cmd = [sys.executable, str(ROOT / 'scripts/generate_cap_token.py'),
           '--kid', kid, '--secret', secret]
    for ns in namespaces:
        cmd += ['--ns', ns]
    for v in verbs:
        cmd += ['--verb', v]
    out = subprocess.check_output(cmd).decode().strip()
    return out


def main():
    # Bring up stack
    ensure_up(COMPOSE)
    try:
        if not wait_healthy(90):
            print('error: server did not become healthy', file=sys.stderr)
            return 2

        token = gen_token([NAMESPACE], ['put', 'get', 'delete', 'query', 'lease', 'admin'])
        wl = Workload(BASE_URL, token, NAMESPACE)
        watcher = Watcher(BASE_URL, token, NAMESPACE)
        watcher.start()
        wl.start(writers=4, readers=2)

        # Warm up
        time.sleep(3)

        # Nemesis: pause/unpause
        print('nemesis: pause/unpause for 5s')
        pause(CONTAINER, seconds=5)
        time.sleep(2)

        # Nemesis: crash/recovery
        print('nemesis: crash (kill) then restart')
        kill(CONTAINER)
        ensure_up(COMPOSE)
        if not wait_healthy(60):
            print('error: server did not recover after crash', file=sys.stderr)
            return 2
        time.sleep(2)

        # Nemesis: disk-full (64MiB tmpfs). Fill ~56MiB to force ENOSPC quickly.
        print('nemesis: disk-full simulation (fill tmpfs)')
        bytes_to_fill = 56 * 1024 * 1024
        fill_disk(CONTAINER, bytes_to_fill)
        # Try a few writes, allow failures, but ensure process stays healthy
        fail_count = 0
        for _ in range(20):
            ok = wl.put_once('df-key', _)
            if not ok:
                fail_count += 1
            time.sleep(0.05)
        if fail_count == 0:
            print('warning: disk-full not observed (runner may have more space). Continuing.', file=sys.stderr)
        # Free space and verify writes succeed again
        free_disk(CONTAINER)
        recovered = False
        for _ in range(30):
            if wl.put_once('df-key', _):
                recovered = True
                break
            time.sleep(0.1)
        if not recovered:
            print('error: writes did not recover after freeing space', file=sys.stderr)
            return 2

        # Assertions: watch stream and metrics
        ok = assert_metrics()
        ok = assert_watch(wl, watcher) and ok
        print('chaos run completed' + ('' if ok else ' with assertion failures'))
        return 0 if ok else 2
    finally:
        wl.stop_all() if 'wl' in locals() else None
        watcher.stop_all() if 'watcher' in locals() else None
        ensure_down(COMPOSE)


if __name__ == '__main__':
    sys.exit(main())


def fetch_metrics():
    try:
        r = requests.get(f"{BASE_URL}/metrics", timeout=5)
        if r.status_code != 200:
            return ''
        return r.text
    except Exception:
        return ''


def parse_metric(text: str, name: str, label_filter: dict | None = None) -> float:
    # very small parser for Prometheus text exposition
    val = 0.0
    for line in text.splitlines():
        if not line or line.startswith('#'):
            continue
        if not line.startswith(name):
            continue
        # e.g., watch_drops_total{reason="overflow"} 0
        try:
            metric, num = line.split(' ', 1)
            labels = {}
            if '{' in metric and metric.endswith('}'+metric.split('}')[1]):
                pass  # guard, but not needed
            if '{' in metric:
                base, lab = metric.split('{', 1)
                lab = lab.rstrip('}')
                for kv in lab.split(','):
                    if not kv:
                        continue
                    k, v = kv.split('=', 1)
                    labels[k] = v.strip('"')
            if label_filter is None or all(labels.get(k) == v for k, v in label_filter.items()):
                try:
                    val += float(num.strip())
                except Exception:
                    continue
        except Exception:
            continue
    return val


def assert_metrics() -> bool:
    text = fetch_metrics()
    if not text:
        print('warning: could not fetch /metrics')
        return False
    ok = True
    drops = parse_metric(text, 'watch_drops_total', {'reason': 'overflow'})
    if drops != 0.0:
        print(f'assertion failed: watch_drops_total{{reason="overflow"}} == 0, got {drops}')
        ok = False
    clients = parse_metric(text, 'watch_clients', {'proto': 'sse'})
    if clients < 1.0:
        print(f'assertion failed: watch_clients{{proto="sse"}} >= 1, got {clients}')
        ok = False
    emitted = parse_metric(text, 'watch_events_total')
    if emitted <= 0.0:
        print('assertion failed: watch_events_total > 0')
        ok = False
    return ok


def assert_watch(wl: Workload, watcher: Watcher) -> bool:
    # give the watcher a moment to catch up
    time.sleep(1.0)
    ok = True
    if watcher.overflow_seen:
        print('assertion failed: watcher saw overflow')
        ok = False
    if watcher.seq_monotonic_violation:
        print('assertion failed: watcher saw non-monotonic sequence ids')
        ok = False
    # Compare a sample of keys that writers touched
    with wl.state_lock:
        sample = list(wl.latest.items())[:5]
    for key, (_ver, body) in sample:
        with watcher.lock:
            w = watcher.last.get(key)
        if not w:
            print(f'assertion failed: watcher did not see key {key}')
            ok = False
            continue
        _, wbody = w
        if wbody != body:
            print(f'assertion failed: watcher body mismatch for {key}')
            ok = False
    return ok
