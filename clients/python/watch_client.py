import argparse
import asyncio
import grpc
import agentstate_pb2 as pb
import agentstate_pb2_grpc as pbg


async def watch(ns: str, from_commit: int, endpoint: str):
    async with grpc.aio.insecure_channel(endpoint) as channel:
        stub = pbg.AgentStateStub(channel)
        req = pb.WatchRequest(ns=ns, from_commit=from_commit)
        last = from_commit
        try:
            async for ev in stub.Watch(req):
                if ev.type == "put":
                    print("PUT", ev.commit, ev.id)
                else:
                    print("DEL", ev.commit, ev.id)
                last = ev.commit
        except grpc.aio.AioRpcError as e:
            print("stream closed:", e)
            return last


async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--ns', required=True)
    ap.add_argument('--from-commit', type=int, default=0)
    ap.add_argument('--endpoint', default='localhost:9090')
    args = ap.parse_args()
    backoff = 1
    last = args.from_commit
    while True:
        last = await watch(args.ns, last, args.endpoint)
        await asyncio.sleep(backoff)
        backoff = min(30, backoff * 2)

if __name__ == '__main__':
    asyncio.run(main())

