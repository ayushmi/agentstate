# Python gRPC Watch Client (MVP)

Prereqs:
- `pip install grpcio grpcio-tools`

Generate stubs:
- `python -m grpc_tools.protoc -I ../../proto --python_out=. --grpc_python_out=. ../../proto/agentstate.proto`

Usage:
- `python watch_client.py --ns acme --from-commit 0`
