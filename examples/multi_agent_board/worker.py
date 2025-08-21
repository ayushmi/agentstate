import argparse, time
from sdk-py.agentstate.client import State

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--owner', required=True)
    args = ap.parse_args()
    s = State("http://localhost:8080/v1/acme")
    while True:
        tasks = s.query(tag_filter={"topic":"board","status":"open"}, limit=1)
        if not tasks:
            time.sleep(1)
            continue
        t = tasks[0]
        tid = t["id"]
        lease = s.lease_acquire(tid, args.owner, 15)
        print("lease", tid, lease)
        # simulate work
        time.sleep(1)
        s.put("result", {"task": tid, "owner": args.owner, "ok": True}, tags={"topic":"board","status":"done"}, idempotency_key=f"res-{tid}")
        s.lease_release(tid, args.owner, lease["token"])

if __name__ == '__main__':
    main()

