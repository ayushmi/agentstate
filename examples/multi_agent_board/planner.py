from sdk-py.agentstate.client import State
import time, uuid

def main():
    s = State("http://localhost:8080/v1/acme")
    for i in range(10):
        tid = str(uuid.uuid4())
        obj = s.put("task", {"id": tid, "text": f"Task {i}"}, tags={"topic":"board","status":"open"}, idempotency_key=f"task-{tid}")
        print("planned", obj.get("id"))
        time.sleep(0.5)

if __name__ == '__main__':
    main()

