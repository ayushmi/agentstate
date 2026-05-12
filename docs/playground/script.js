(() => {
  const $ = (id) => document.getElementById(id);
  const out = $("out");
  const watchOut = $("watchOut");
  let es;

  const cfg = {
    get baseUrl() { return $("baseUrl").value.trim() || "http://localhost:8080"; },
    get ns() { return $("namespace").value.trim() || "my-app"; },
    get token() { return $("token").value.trim(); }
  };

  function saveConfig() {
    localStorage.setItem("as.baseUrl", cfg.baseUrl);
    localStorage.setItem("as.ns", cfg.ns);
    localStorage.setItem("as.token", cfg.token);
    log("Saved config.");
    renderSnippets();
  }

  function loadConfig() {
    $("baseUrl").value = localStorage.getItem("as.baseUrl") || "http://localhost:8080";
    $("namespace").value = localStorage.getItem("as.ns") || "my-app";
    $("token").value = localStorage.getItem("as.token") || "";
    renderSnippets();
  }

  function log(msg) {
    const s = typeof msg === 'string' ? msg : JSON.stringify(msg, null, 2);
    out.textContent = s;
  }

  function headers() {
    const h = { 'Content-Type': 'application/json' };
    if (cfg.token) h['Authorization'] = `Bearer ${cfg.token}`;
    return h;
  }

  async function fetchJSON(url, opts={}) {
    const res = await fetch(url, opts);
    const text = await res.text();
    let body = null;
    try { body = JSON.parse(text); } catch { body = text; }
    if (!res.ok) throw { status: res.status, body };
    return body;
  }

  async function doPut() {
    const type = $("putType").value.trim() || "chatbot";
    let body = {};
    let tags = {};
    try { body = JSON.parse($("putBody").value || '{}'); } catch (e) { return log({error: 'Invalid body JSON'}); }
    try { tags = JSON.parse($("putTags").value || '{}'); } catch (e) { return log({error: 'Invalid tags JSON'}); }
    try {
      const resp = await fetchJSON(`${cfg.baseUrl}/v1/${encodeURIComponent(cfg.ns)}/objects`, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify({ type, body, tags })
      });
      log(resp);
      if (resp && resp.id) { $("getId").value = resp.id; }
    } catch (e) { log(e); }
  }

  async function doQuery() {
    let tags = {};
    try { tags = JSON.parse($("queryTags").value || '{}'); } catch (e) { return log({error: 'Invalid tags JSON'}); }
    try {
      const resp = await fetchJSON(`${cfg.baseUrl}/v1/${encodeURIComponent(cfg.ns)}/query`, {
        method: 'POST',
        headers: headers(),
        body: JSON.stringify({ tags })
      });
      log(resp);
    } catch (e) { log(e); }
  }

  async function doGet() {
    const id = $("getId").value.trim();
    if (!id) return log({error: 'Missing id'});
    try {
      const resp = await fetchJSON(`${cfg.baseUrl}/v1/${encodeURIComponent(cfg.ns)}/objects/${encodeURIComponent(id)}`, {
        headers: headers()
      });
      log(resp);
    } catch (e) { log(e); }
  }

  function startWatch() {
    if (es) es.close();
    watchOut.textContent = '';
    const url = `${cfg.baseUrl}/v1/${encodeURIComponent(cfg.ns)}/watch`;
    // EventSource doesn't support headers; if token is needed, advise proxy or gateway.
    if (cfg.token) {
      appendWatch({warning: 'SSE cannot set Authorization header. Use a gateway that injects it, or run without caps for dev.'});
    }
    es = new EventSource(url);
    es.onmessage = (ev) => appendWatch({event: 'message', data: safeParse(ev.data)});
    es.onerror = () => appendWatch({event: 'error'});
  }

  function stopWatch() {
    if (es) { es.close(); es = null; appendWatch({event: 'closed'}); }
  }

  function appendWatch(obj) {
    const s = typeof obj === 'string' ? obj : JSON.stringify(obj, null, 2);
    watchOut.textContent += s + "\n";
    watchOut.scrollTop = watchOut.scrollHeight;
  }

  function safeParse(s) {
    try { return JSON.parse(s); } catch { return s; }
  }

  async function loadHosted() {
    // Placeholder: expect a hosted gateway exposing /api/token returning { token, baseUrl, namespace }
    try {
      const gw = localStorage.getItem('as.gateway') || '';
      if (!gw) {
        alert('Set localStorage as.gateway to your hosted gateway URL (e.g., https://play.agentstate.run) and reload.');
        return;
      }
      const resp = await fetch(`${gw}/api/token`, { method: 'POST', credentials: 'include' });
      const data = await resp.json();
      if (data.baseUrl) $("baseUrl").value = data.baseUrl;
      if (data.namespace) $("namespace").value = data.namespace;
      if (data.token) $("token").value = data.token;
      saveConfig();
      log({info: 'Loaded hosted token', ns: data.namespace});
    } catch (e) {
      log({error: 'Failed to load hosted token', detail: e});
    }
  }

  function renderSnippets() {
    const py = `from agentstate import AgentStateClient\n\nclient = AgentStateClient(base_url='${cfg.baseUrl}', namespace='${cfg.ns}', api_key='${cfg.token}')\nagent = client.create_agent('chatbot', {'name': 'CustomerBot', 'status': 'active'})\nprint(agent)`;
    const ts = `import { AgentStateClient } from 'agentstate'\n\nconst client = new AgentStateClient({ baseUrl: '${cfg.baseUrl}', namespace: '${cfg.ns}', apiKey: '${cfg.token}' })\nconst agent = await client.createAgent({ type: 'chatbot', body: { name: 'CustomerBot', status: 'active' } })\nconsole.log(agent)`;
    $("pySnippet").textContent = py;
    $("tsSnippet").textContent = ts;
  }

  $("saveConfig").addEventListener('click', saveConfig);
  $("loadHosted").addEventListener('click', loadHosted);
  $("doPut").addEventListener('click', doPut);
  $("doQuery").addEventListener('click', doQuery);
  $("doGet").addEventListener('click', doGet);
  $("startWatch").addEventListener('click', startWatch);
  $("stopWatch").addEventListener('click', stopWatch);
  loadConfig();
})();

