# Capability Key Rotation (Zero-Downtime)

Assumptions
- HMAC caps with kid; server accepts tokens signed by active or next keys.
- Env vars:
  - `CAP_KEY_ACTIVE_ID`, `CAP_KEY_ACTIVE`
  - `CAP_KEY_NEXT_ID`, `CAP_KEY_NEXT`

## Steps

1) Generate new key (base64):

```
NEW_ID=rot-$(date +%s)
NEW_KEY=$(openssl rand -base64 32)
```

2) Stage as NEXT (all pods):

Set:

```
CAP_KEY_NEXT_ID=$NEW_ID
CAP_KEY_NEXT=$NEW_KEY
```

Apply (Helm):

```
helm upgrade ... \
  --set env.CAP_KEY_NEXT_ID=$NEW_ID \
  --set env.CAP_KEY_NEXT=$NEW_KEY
```

Verify server readiness probes pass.

Start issuing new tokens signed with NEXT (kid=$NEW_ID). Old tokens continue to work.

3) Promote to ACTIVE:

Set:

```
CAP_KEY_ACTIVE_ID=$NEW_ID
CAP_KEY_ACTIVE=$NEW_KEY
CAP_KEY_NEXT_ID=
CAP_KEY_NEXT=
```

Helm upgrade again. (Rolling update; no downtime.)

4) Audit:
- Check metrics/logs for reject spikes.
- Optionally invalidate old tokens by shortening `exp` during the overlap window.

If you later switch to public-key JWS (EdDSA), expose `/admin/jwks` for verifiers and follow the same active/next pattern.

## Token mint (Python)

```python
import base64, hmac, hashlib, json, time

def cap_token(kid, key_b64, claims):
    header = {"alg":"HS256","typ":"JWT","kid":kid}
    b64 = lambda b: base64.urlsafe_b64encode(b).rstrip(b'=')
    enc = lambda o: b64(json.dumps(o, separators=(',',':')).encode())
    signing_input = enc(header) + b'.' + enc(claims)
    sig = hmac.new(base64.b64decode(key_b64), signing_input, hashlib.sha256).digest()
    return f"{kid}.{signing_input.decode()}.{b64(sig).decode()}"

claims = {
  "ns":"agent://acme", "verbs":["put","get","query","watch","lease","admin"],
  "exp": int(time.time()) + 3600, "region":"eu-west-1", "max_bytes":1048576, "max_qps":200, "jti":"demo-1"
}
print(cap_token("active", "<CAP_KEY_ACTIVE>", claims))
```

