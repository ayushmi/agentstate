import express from 'express';
import { createProxyMiddleware } from 'http-proxy-middleware';
import crypto from 'crypto';
import { nanoid } from 'nanoid';
import cookie from 'cookie';

// Config
const PORT = process.env.PORT || 8787;
const UPSTREAM_BASE = process.env.UPSTREAM_BASE || 'http://localhost:8080';
const CAP_ID = process.env.CAP_KEY_ACTIVE_ID || 'active';
const CAP_SECRET = process.env.CAP_KEY_ACTIVE || 'dev-secret';
const FREE_MAX_QPS = parseInt(process.env.FREE_MAX_QPS || '5', 10);
const FREE_MAX_BYTES = parseInt(process.env.FREE_MAX_BYTES || '262144', 10); // 256KB

// Simple in-memory usage counters (replace with Redis/Postgres in prod)
const usage = new Map(); // key: ns, value: { ops: number, lastFlush: number }

const app = express();
app.use(express.json());

// Utility: base64url without padding
function b64url(buf) {
  return Buffer.from(buf).toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

function createCapToken({ ns, verbs, expSec = 24 * 3600, maxQps = FREE_MAX_QPS, maxBytes = FREE_MAX_BYTES, region = '' }) {
  const exp = Math.floor(Date.now() / 1000) + expSec;
  const jti = nanoid();
  const claims = { ns: [ns], verbs, exp, max_qps: maxQps, max_bytes: maxBytes, region, jti };
  const payload = b64url(JSON.stringify(claims));
  const mac = crypto.createHmac('sha256', CAP_SECRET);
  mac.update(Buffer.from(payload));
  const sig = b64url(mac.digest());
  return `${CAP_ID}.${payload}.${sig}`;
}

// Very lightweight demo auth: namespace cookie or anonymous new ns
function getOrCreateNamespace(req, res) {
  const cookies = cookie.parse(req.headers.cookie || '');
  let ns = cookies['as_ns'];
  if (!ns) {
    ns = `demo-${nanoid(8)}`;
    res.setHeader('Set-Cookie', cookie.serialize('as_ns', ns, { httpOnly: true, sameSite: 'Lax', path: '/', maxAge: 86400 * 7 }));
  }
  return ns;
}

app.post('/api/token', (req, res) => {
  const ns = getOrCreateNamespace(req, res);
  const token = createCapToken({ ns, verbs: ['put', 'get', 'query', 'delete', 'watch', 'lease'] });
  res.json({ baseUrl: process.env.PUBLIC_BASE || `http://localhost:${PORT}`, namespace: ns, token });
});

// Usage middleware: counts 1 op per request (tunable per method)
function meter(req, res, next) {
  // Only count API paths
  if (!req.path.startsWith('/v1/')) return next();
  const ns = (req.params && req.params.ns) || (req.path.split('/')[2] || 'unknown');
  const rec = usage.get(ns) || { ops: 0, lastFlush: Date.now() };
  rec.ops += 1;
  usage.set(ns, rec);
  next();
}

// Inject Authorization on the way to upstream. Note: SSE watch must go through this gateway to carry auth.
const injectAuth = (proxyReq, req, res) => {
  const cookies = cookie.parse(req.headers.cookie || '');
  const ns = cookies['as_ns'] || (req.path.split('/')[2] || 'demo');
  const token = createCapToken({ ns, verbs: ['put', 'get', 'query', 'delete', 'watch', 'lease'] });
  proxyReq.setHeader('Authorization', `Bearer ${token}`);
};

app.use('/v1', meter, createProxyMiddleware({
  target: UPSTREAM_BASE,
  changeOrigin: true,
  pathRewrite: (path) => path, // keep as-is
  onProxyReq: injectAuth,
  ws: true,
}));

app.get('/health', (_req, res) => res.json({ ok: true }));

app.listen(PORT, () => {
  console.log(`AgentState gateway listening on :${PORT}, upstream=${UPSTREAM_BASE}`);
});

