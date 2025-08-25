import os
import subprocess
import time
import shlex


def sh(cmd: str, check=True) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, shell=True, check=check, capture_output=True, text=True)


def pause(container: str, seconds: int = 5):
    sh(f"docker pause {shlex.quote(container)}")
    time.sleep(seconds)
    sh(f"docker unpause {shlex.quote(container)}")


def kill(container: str):
    # SIGKILL to simulate crash
    sh(f"docker kill {shlex.quote(container)}")


def ensure_up(compose_files):
    files = " ".join(f"-f {shlex.quote(f)}" for f in compose_files)
    sh(f"docker compose {files} up -d --build")


def ensure_down(compose_files):
    files = " ".join(f"-f {shlex.quote(f)}" for f in compose_files)
    sh(f"docker compose {files} down -v || true", check=False)


def fill_disk(container: str, target_bytes: int):
    # write a large file in /data inside the container
    # fallocate if available, otherwise dd
    cmd = f"bash -lc 'command -v fallocate >/dev/null 2>&1 && fallocate -l {target_bytes} /data/fill || dd if=/dev/zero of=/data/fill bs=1M count=$(( {target_bytes} / 1048576 )) conv=fsync'"
    # run via sh to allow bash -lc; Debian image has bash
    sh(f"docker exec {shlex.quote(container)} {cmd}")


def free_disk(container: str):
    sh(f"docker exec {shlex.quote(container)} rm -f /data/fill || true", check=False)

