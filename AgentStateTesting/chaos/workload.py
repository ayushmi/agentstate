import os
import threading
import time
import json
import random
import string
from typing import Dict, Tuple

import requests


def _rand(n: int = 8) -> str:
    return ''.join(random.choice(string.ascii_lowercase + string.digits) for _ in range(n))


class Workload:
    def __init__(self, base_url: str, token: str, namespace: str = "chaos"):
        self.base_url = base_url.rstrip('/')
        self.ns = namespace
        self.token = token
        self.session = requests.Session()
        self.session.headers["Authorization"] = f"Bearer {self.token}" if self.token else ""
        self.stop = threading.Event()
        self.state_lock = threading.Lock()
        # tracked latest values per key: (version, body)
        self.latest: Dict[str, Tuple[int, dict]] = {}

    def put_once(self, key: str, version: int) -> bool:
        body = {"id": key, "ns": self.ns, "type": "note", "body": {"msg": f"v{version}"}}
        try:
            r = self.session.post(f"{self.base_url}/v1/{self.ns}/objects", json=body, timeout=5)
            ok = r.status_code == 200
            if ok:
                with self.state_lock:
                    self.latest[key] = (version, body["body"])  # track intended body
            return ok
        except Exception:
            return False

    def get_and_check(self, key: str) -> bool:
        try:
            r = self.session.get(f"{self.base_url}/v1/{self.ns}/objects/{key}", timeout=5)
            if r.status_code != 200:
                return False
            data = r.json()
            with self.state_lock:
                expected = self.latest.get(key)
            if not expected:
                return True
            _, body = expected
            return data.get("body") == body
        except Exception:
            return False

    def writer(self, keys=16, delay=0.01):
        versions = [0] * keys
        ids = [f"k-{i}-{_rand(4)}" for i in range(keys)]
        while not self.stop.is_set():
            idx = random.randrange(keys)
            versions[idx] += 1
            self.put_once(ids[idx], versions[idx])
            time.sleep(delay)

    def reader(self, keys=16, delay=0.01):
        ids = [f"k-{i}" for i in range(keys)]
        while not self.stop.is_set():
            # check a random known key if present
            key = random.choice(list(self.latest.keys()) or [f"k-0"])
            self.get_and_check(key)
            time.sleep(delay)

    def start(self, writers=2, readers=2):
        self.threads = []
        for _ in range(writers):
            t = threading.Thread(target=self.writer, daemon=True)
            t.start()
            self.threads.append(t)
        for _ in range(readers):
            t = threading.Thread(target=self.reader, daemon=True)
            t.start()
            self.threads.append(t)

    def stop_all(self):
        self.stop.set()
        for t in self.threads:
            t.join(timeout=2)


class Watcher:
    def __init__(self, base_url: str, token: str, namespace: str = "chaos"):
        self.base_url = base_url.rstrip('/')
        self.ns = namespace
        self.token = token
        self.session = requests.Session()
        if self.token:
            self.session.headers["Authorization"] = f"Bearer {self.token}"
        self.stop = threading.Event()
        self.thread = None
        self.global_last_seq = -1
        self.seq_monotonic_violation = False
        self.overflow_seen = False
        self.lock = threading.Lock()
        # last seen per id
        self.last: Dict[str, Tuple[int, dict]] = {}

    def _consume(self):
        while not self.stop.is_set():
            try:
                with self.session.get(f"{self.base_url}/v1/{self.ns}/watch", stream=True, timeout=10) as r:
                    if r.status_code != 200:
                        time.sleep(0.5)
                        continue
                    event_seq = None
                    for line in r.iter_lines(decode_unicode=True):
                        if self.stop.is_set():
                            break
                        if not line:
                            continue
                        if line.startswith('id: '):
                            try:
                                event_seq = int(line[4:].strip())
                            except Exception:
                                event_seq = None
                        elif line.startswith('data: '):
                            data = line[6:]
                            try:
                                ev = json.loads(data)
                            except Exception:
                                continue
                            if 'error' in ev and ev.get('error') == 'overflow':
                                self.overflow_seen = True
                                return
                            # commit_seq can be on root
                            seq = ev.get('commit_seq', event_seq if event_seq is not None else -1)
                            with self.lock:
                                if seq is not None and isinstance(seq, int):
                                    if seq <= self.global_last_seq:
                                        self.seq_monotonic_violation = True
                                    self.global_last_seq = max(self.global_last_seq, seq)
                                if ev.get('type') == 'put' and isinstance(ev.get('obj'), dict):
                                    oid = ev['obj'].get('id')
                                    body = ev['obj'].get('body')
                                    if oid is not None:
                                        self.last[oid] = (seq if isinstance(seq, int) else -1, body)
            except Exception:
                time.sleep(0.5)

    def start(self):
        self.thread = threading.Thread(target=self._consume, daemon=True)
        self.thread.start()

    def stop_all(self):
        self.stop.set()
        # Close HTTP session to break blocking iter
        try:
            self.session.close()
        except Exception:
            pass
        if self.thread:
            self.thread.join(timeout=2)

