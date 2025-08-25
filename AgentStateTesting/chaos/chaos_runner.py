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
from workload import Workload

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

        print('chaos run completed')
        return 0
    finally:
        wl.stop_all() if 'wl' in locals() else None
        ensure_down(COMPOSE)


if __name__ == '__main__':
    sys.exit(main())

