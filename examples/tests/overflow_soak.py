import asyncio, os, random
from agentstate import State

async def writer(state: State):
  i = 0
  while True:
    state.put("evt", {"i": i, "text": "x"*256}, tags={"t":"soak"})
    i += 1
    await asyncio.sleep(0)

async def watcher(state: State, idx: int):
  last = 0
  async for ev in state.watch(filter={"tags":{"t":"soak"}}, from_commit=None, on_gap=lambda c: print(f"[w{idx}] gap at {c}")):
    cs = int(ev.get("commit_seq") or ev.get("commit") or 0)
    if cs <= last:
      raise RuntimeError("non-monotonic stream")
    last = cs
    if random.random() < 0.0005:
      await asyncio.sleep(0.5)

async def main():
  os.environ.setdefault("WATCH_BUFFER_EVENTS", "100")
  s = State("http://localhost:8080/v1/soak")
  await asyncio.gather(writer(s), *(watcher(s, i) for i in range(10)))

if __name__ == '__main__':
  asyncio.run(main())

