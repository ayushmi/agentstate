# TypeScript SDK (MVP)

Install (local): `npm install` then `npm pack` or use via workspace link.

Usage:

```ts
import { State } from "@agentstate/sdk";
const s = new State("http://localhost:8080/v1/acme");
const obj = await s.put("note", { text: "hello" }, { topic: "demo" });
console.log(await s.get(obj.id));
```

