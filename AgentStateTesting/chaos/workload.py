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

