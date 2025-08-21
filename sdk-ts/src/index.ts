export type Tags = Record<string, string>;

export class State {
  base: string;
  constructor(base: string) { this.base = base.replace(/\/$/, ""); }

  async put(type: string, body: any, tags?: Tags, ttl_seconds?: number, id?: string, idempotency_key?: string) {
    const payload: any = { type, body };
    if (tags) payload.tags = tags;
    if (ttl_seconds !== undefined) payload.ttl_seconds = ttl_seconds;
    if (id) payload.id = id;
    const headers: Record<string,string> = { 'content-type': 'application/json' };
    if (idempotency_key) headers['Idempotency-Key'] = idempotency_key;
    const r = await fetch(`${this.base}/objects`, { method: 'POST', headers, body: JSON.stringify(payload) });
    if (!r.ok) throw new Error(await r.text());
    return await r.json();
  }

  async get(id: string) {
    const r = await fetch(`${this.base}/objects/${id}`);
    if (!r.ok) throw new Error(await r.text());
    return await r.json();
  }

  async query(tag_filter?: Tags, jsonpath?: string, limit?: number, fields?: string[]) {
    const payload: any = {};
    if (tag_filter) payload.tag_filter = tag_filter;
    if (jsonpath) payload.jsonpath = { equals: { [jsonpath]: true } };
    if (limit !== undefined) payload.limit = limit;
    if (fields !== undefined) payload.fields = fields;
    const r = await fetch(`${this.base}/query`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(payload) });
    if (!r.ok) throw new Error(await r.text());
    return await r.json();
  }

  async leaseAcquire(key: string, owner: string, ttl: number) {
    const r = await fetch(`${this.base}/lease/acquire`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, owner, ttl }) });
    if (!r.ok) throw new Error(await r.text());
    return await r.json();
  }
  async leaseRenew(key: string, owner: string, token: number, ttl: number) {
    const r = await fetch(`${this.base}/lease/renew`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, owner, token, ttl }) });
    if (!r.ok) throw new Error(await r.text());
    return await r.json();
  }
  async leaseRelease(key: string, owner: string, token: number) {
    const r = await fetch(`${this.base}/lease/release`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, owner, token }) });
    if (!r.ok) throw new Error(await r.text());
    return true;
  }

  async watch(options?: {
    filter?: Record<string, any>,
    from_commit?: number,
    on_gap?: (lastCommit: number) => void,
    checkpoint_path?: string,
    backoff_min_ms?: number,
    backoff_max_ms?: number,
    grpcEndpoint?: string,
  }) {
    const filter = options?.filter;
    let last = options?.from_commit ?? 0;
    const min = options?.backoff_min_ms ?? 250;
    const max = options?.backoff_max_ms ?? 4000;
    const grpcEndpoint = options?.grpcEndpoint ?? this.base.replace(/^http:\/\//, '').replace(/^https:\/\//, '').replace(/:8080$/, ':9090');

    async function maybeCheckpoint(path: string | undefined, lastCommit: number) {
      if (!path) return;
      try {
        const fs = await import('node:fs');
        await (fs as any).promises.writeFile(path, String(lastCommit) + "\n");
      } catch {}
    }

    async function* sseStream(base: string) {
      let backoff = min;
      for (;;) {
        try {
          const r = await fetch(`${base}/watch`);
          if (!r.ok) throw new Error(`sse status ${r.status}`);
          const reader = (r.body as any).getReader();
          const decoder = new TextDecoder();
          let buf = '';
          while (true) {
            const { value, done } = await reader.read();
            if (done) break;
            buf += decoder.decode(value, { stream: true });
            const lines = buf.split(/\r?\n/);
            buf = lines.pop() || '';
            for (const line of lines) {
              if (!line || line.startsWith(':')) continue;
              if (line.startsWith('id:')) {
                const id = parseInt(line.slice(3).trim(), 10);
                if (!Number.isNaN(id)) { last = Math.max(last, id); await maybeCheckpoint(options?.checkpoint_path, last); }
              } else if (line.startsWith('data:')) {
                const data = line.slice(5).trim();
                try {
                  const evt = JSON.parse(data);
                  if (evt.error === 'overflow') { options?.on_gap?.(last); throw new Error('overflow'); }
                  const commit = parseInt(evt.commit_seq ?? evt.commit ?? '0', 10);
                  if (commit && commit > last) { last = commit; await maybeCheckpoint(options?.checkpoint_path, last); yield evt; }
                } catch {}
              }
            }
          }
        } catch (e) {
          const jitter = Math.floor(Math.random() * (backoff/4));
          await new Promise(res => setTimeout(res, backoff + jitter));
          backoff = Math.min(max, Math.max(min, backoff * 2));
        }
      }
    }

    async function* grpcStream(endpoint: string) {
      let backoff = min;
      for (;;) {
        try {
          const grpc = await import('@grpc/grpc-js');
          const loader = await import('@grpc/proto-loader');
          const PROTO_PATH = new URL('../../proto/agentstate.proto', import.meta.url).pathname;
          const pkg = loader.loadSync(PROTO_PATH, { keepCase: true, longs: String, enums: String, defaults: true });
          const agent: any = (grpc as any).loadPackageDefinition(pkg).agentstate.v1;
          const client = new agent.AgentState(endpoint, (grpc as any).credentials.createInsecure());
          const req = { ns: this.base.split('/v1/')[1], from_commit: last };
          const call = client.Watch(req);
          await new Promise<void>((resolve, reject) => {
            call.on('data', async (ev: any) => { const c = parseInt(ev.commit, 10); if (c && c > last) { last = c; await maybeCheckpoint(options?.checkpoint_path, last); } });
            call.on('error', (err: any) => { const m = String(err.message||''); const m2 = /last_commit=(\d+)/.exec(m); if (m2) { const c = parseInt(m2[1],10); if (c) { options?.on_gap?.(c); last = c; } } reject(err); });
            call.on('end', () => resolve());
          });
        } catch (e) {
          const jitter = Math.floor(Math.random() * (backoff/4));
          await new Promise(res => setTimeout(res, backoff + jitter));
          backoff = Math.min(max, Math.max(min, backoff * 2));
        }
      }
    }

    // Prefer gRPC in Node, else SSE
    const isNode = typeof process !== 'undefined' && !!(process as any).versions?.node;
    return isNode ? grpcStream.call(this, grpcEndpoint) : sseStream(this.base);
  }
}
