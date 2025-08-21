import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = { vus: 50, duration: '60s' };

export default function () {
  const url = __ENV.HOST || 'http://localhost:8080/v1/ns/objects';
  const cap = __ENV.CAP || '';
  let res = http.post(url, JSON.stringify({type:"t", body:{x:1}}), {
    headers: { 'Authorization': `Bearer ${cap}`, 'Content-Type': 'application/json' }
  });
  check(res, { 'ok or 429': r => r.status === 200 || r.status === 429 });
  sleep(0.0);
}

