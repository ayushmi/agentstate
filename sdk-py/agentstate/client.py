import requests
from typing import Any, Dict, Optional
import time, json, os, random


class State:
    def __init__(self, base: str):
        # base: http://host:8080/v1/{ns}
        self.base = base.rstrip('/')

    def put(self, typ: str, body: Dict[str, Any], tags: Optional[Dict[str, str]] = None, ttl_seconds: Optional[int] = None, id: Optional[str] = None, idempotency_key: Optional[str] = None) -> Dict[str, Any]:
        payload = {"type": typ, "body": body}
        if tags:
            payload["tags"] = tags
        if ttl_seconds is not None:
            payload["ttl_seconds"] = ttl_seconds
        if id:
            payload["id"] = id
        headers = {}
        if idempotency_key:
            headers["Idempotency-Key"] = idempotency_key
        r = requests.post(f"{self.base}/objects", json=payload, headers=headers)
        r.raise_for_status()
        return r.json()

    def get(self, id: str) -> Dict[str, Any]:
        r = requests.get(f"{self.base}/objects/{id}")
        r.raise_for_status()
        return r.json()

    def query(self, tag_filter: Optional[Dict[str, str]] = None, jsonpath: Optional[str] = None, limit: Optional[int] = None, fields: Optional[list[str]] = None):
        payload: Dict[str, Any] = {}
        if tag_filter:
            payload["tag_filter"] = tag_filter
        if jsonpath:
            payload["jsonpath"] = {"equals": {jsonpath: True}}
        if limit is not None:
            payload["limit"] = limit
        if fields is not None:
            payload["fields"] = fields
        r = requests.post(f"{self.base}/query", json=payload)
        r.raise_for_status()
        return r.json()

    def watch(self,
              filter: Optional[Dict[str, Any]] = None,
              from_commit: Optional[int] = None,
              on_gap: Optional[Any] = None,
              checkpoint_path: Optional[str] = None,
              backoff_min_ms: int = 250,
              backoff_max_ms: int = 4000):
        """SSE-based watch with auto-resume and jittered backoff.
        Prefer gRPC by using the provided Python client (not bundled) if desired.
        """
        last = from_commit or 0
        if checkpoint_path and os.path.exists(checkpoint_path):
            try:
                with open(checkpoint_path, 'r') as f:
                    last = int(f.read().strip())
            except Exception:
                pass

        def save_checkpoint(n: int):
            if not checkpoint_path:
                return
            try:
                with open(checkpoint_path, 'w') as f:
                    f.write(str(n))
                    f.flush()
                    os.fsync(f.fileno())
            except Exception:
                pass

        backoff = backoff_min_ms
        while True:
            try:
                # SSE stream
                url = f"{self.base}/watch"
                headers = {}
                # from_commit is passed by SSE id resume via Last-Event-ID if supported; simplest: filter server-side by ignoring, but we embed in URL only via gRPC normally
                # We will just resume client-side by filtering events < last
                with requests.get(url, stream=True, headers=headers, timeout=60) as r:
                    r.raise_for_status()
                    buf = ""
                    for line in r.iter_lines(decode_unicode=True):
                        if line is None:
                            continue
                        if line.startswith(":"):
                            continue
                        if line.startswith("id:"):
                            try:
                                last = max(last, int(line.split(":",1)[1].strip()))
                                save_checkpoint(last)
                            except Exception:
                                pass
                        elif line.startswith("data:"):
                            data = line.split(":",1)[1].strip()
                            try:
                                evt = json.loads(data)
                            except Exception:
                                continue
                            if evt.get("error") == "overflow":
                                if on_gap:
                                    try: on_gap(last)
                                    except Exception: pass
                                break  # reconnect
                            commit = evt.get("commit_seq") or evt.get("commit") or last
                            if commit and int(commit) <= last:
                                continue
                            last = int(commit)
                            save_checkpoint(last)
                            yield evt
                # overflow or end, backoff and resume
                jitter = random.randint(0, max(1, backoff//4))
                time.sleep((backoff + jitter) / 1000.0)
                backoff = min(backoff_max_ms, max(backoff_min_ms, backoff * 2))
            except requests.RequestException:
                jitter = random.randint(0, max(1, backoff//4))
                time.sleep((backoff + jitter) / 1000.0)
                backoff = min(backoff_max_ms, max(backoff_min_ms, backoff * 2))

    def lease_acquire(self, key: str, owner: str, ttl: int):
        r = requests.post(f"{self.base}/lease/acquire", json={"key": key, "owner": owner, "ttl": ttl})
        r.raise_for_status()
        return r.json()

    def lease_renew(self, key: str, owner: str, token: int, ttl: int):
        r = requests.post(f"{self.base}/lease/renew", json={"key": key, "owner": owner, "token": token, "ttl": ttl})
        r.raise_for_status()
        return r.json()

    def lease_release(self, key: str, owner: str, token: int):
        r = requests.post(f"{self.base}/lease/release", json={"key": key, "owner": owner, "token": token})
        r.raise_for_status()
        return True
