# Python SDK (MVP)

Install (local): `pip install -e .` from `sdk-py` directory.

Usage:

```python
from agentstate import State
s = State("http://localhost:8080/v1/acme")
o = s.put("note", {"text":"hello"}, tags={"topic":"demo"})
g = s.get(o["id"]) 
print(g)
```

