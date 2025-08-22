import requests
from typing import Any, Dict, Optional, List, Union
import time, json, os, random


class AgentStateClient:
    """
    AgentState Python SDK - "Firebase for AI Agents"
    
    Provides a simple interface for managing AI agent state with:
    - Real-time state updates
    - Rich querying by tags
    - Persistent storage
    - High performance and reliability
    
    Example:
        client = AgentStateClient("http://localhost:8080", "my-app", api_key="your-api-key")
        
        # Create an agent
        agent = client.create_agent("chatbot", 
                                   {"name": "CustomerBot", "status": "active"},
                                   {"team": "support"})
        
        # Query agents
        agents = client.query_agents({"team": "support"})
    """
    
    def __init__(self, base_url: str = "http://localhost:8080", namespace: str = "default", api_key: Optional[str] = None):
        """
        Initialize AgentState client.
        
        Args:
            base_url: AgentState server URL (e.g., "http://localhost:8080")
            namespace: Namespace for organizing agents (e.g., "production", "staging")
            api_key: API key for authentication (optional, can also be set via AGENTSTATE_API_KEY env var)
        """
        self.base_url = base_url.rstrip('/')
        self.namespace = namespace
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'agentstate-python-sdk/1.0.1'
        })
        
        # Set up authentication if API key is provided
        api_key = api_key or os.environ.get('AGENTSTATE_API_KEY')
        if api_key:
            self.session.headers['Authorization'] = f'Bearer {api_key}'

    def create_agent(self, agent_type: str, body: Dict[str, Any], 
                    tags: Optional[Dict[str, str]] = None, 
                    agent_id: Optional[str] = None) -> Dict[str, Any]:
        """
        Create or update an agent.
        
        Args:
            agent_type: Type of agent (e.g., "chatbot", "workflow", "classifier")  
            body: Agent state data (any JSON-serializable object)
            tags: Key-value pairs for querying and organization
            agent_id: Specific ID to use (for updates), auto-generated if None
            
        Returns:
            Created agent object with id, type, body, tags, commit_seq, commit_ts
        """
        payload = {
            "type": agent_type,
            "body": body,
            "tags": tags or {}
        }
        if agent_id:
            payload["id"] = agent_id
            
        response = self.session.post(f"{self.base_url}/v1/{self.namespace}/objects", json=payload)
        response.raise_for_status()
        return response.json()

    def get_agent(self, agent_id: str) -> Dict[str, Any]:
        """
        Get agent by ID.
        
        Args:
            agent_id: Unique agent identifier
            
        Returns:
            Agent object with id, type, body, tags, commit_seq, commit_ts
        """
        response = self.session.get(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
        return response.json()

    def query_agents(self, tags: Optional[Dict[str, str]] = None) -> List[Dict[str, Any]]:
        """
        Query agents by tags.
        
        Args:
            tags: Tag filters (e.g., {"team": "support", "status": "active"})
            
        Returns:
            List of matching agent objects
        """
        query = {}
        if tags:
            query["tags"] = tags
            
        response = self.session.post(f"{self.base_url}/v1/{self.namespace}/query", json=query)
        response.raise_for_status()
        return response.json()
    
    def delete_agent(self, agent_id: str) -> None:
        """
        Delete an agent.
        
        Args:
            agent_id: Unique agent identifier
        """
        response = self.session.delete(f"{self.base_url}/v1/{self.namespace}/objects/{agent_id}")
        response.raise_for_status()
    
    def health_check(self) -> bool:
        """
        Check if AgentState server is healthy.
        
        Returns:
            True if server is healthy, False otherwise
        """
        try:
            # Create a separate session without auth headers for health check
            # The health endpoint should not require authentication
            import requests
            response = requests.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200 and response.text.strip() == "ok"
        except:
            return False

    # Legacy API compatibility
    def put(self, typ: str, body: Dict[str, Any], tags: Optional[Dict[str, str]] = None, 
            ttl_seconds: Optional[int] = None, id: Optional[str] = None, 
            idempotency_key: Optional[str] = None) -> Dict[str, Any]:
        """Legacy method for backward compatibility. Use create_agent() instead."""
        return self.create_agent(typ, body, tags, id)

    def get(self, id: str) -> Dict[str, Any]:
        """Legacy method for backward compatibility. Use get_agent() instead."""
        return self.get_agent(id)

    def query(self, tag_filter: Optional[Dict[str, str]] = None, **kwargs) -> List[Dict[str, Any]]:
        """Legacy method for backward compatibility. Use query_agents() instead."""
        return self.query_agents(tag_filter)

    def watch(self,
              filter: Optional[Dict[str, Any]] = None,
              from_commit: Optional[int] = None,
              on_gap: Optional[Any] = None,
              checkpoint_path: Optional[str] = None,
              backoff_min_ms: int = 250,
              backoff_max_ms: int = 4000):
        """SSE-based watch with auto-resume and jittered backoff.
        Prefer gRPC by using the provided Python client (not bundled) if desired.
        """
        last = from_commit or 0
        if checkpoint_path and os.path.exists(checkpoint_path):
            try:
                with open(checkpoint_path, 'r') as f:
                    last = int(f.read().strip())
            except Exception:
                pass

        def save_checkpoint(n: int):
            if not checkpoint_path:
                return
            try:
                with open(checkpoint_path, 'w') as f:
                    f.write(str(n))
                    f.flush()
                    os.fsync(f.fileno())
            except Exception:
                pass

        backoff = backoff_min_ms
        while True:
            try:
                # SSE stream
                url = f"{self.base_url}/v1/{self.namespace}/watch"
                headers = {}
                # from_commit is passed by SSE id resume via Last-Event-ID if supported; simplest: filter server-side by ignoring, but we embed in URL only via gRPC normally
                # We will just resume client-side by filtering events < last
                with requests.get(url, stream=True, headers=headers, timeout=60) as r:
                    r.raise_for_status()
                    buf = ""
                    for line in r.iter_lines(decode_unicode=True):
                        if line is None:
                            continue
                        if line.startswith(":"):
                            continue
                        if line.startswith("id:"):
                            try:
                                last = max(last, int(line.split(":",1)[1].strip()))
                                save_checkpoint(last)
                            except Exception:
                                pass
                        elif line.startswith("data:"):
                            data = line.split(":",1)[1].strip()
                            try:
                                evt = json.loads(data)
                            except Exception:
                                continue
                            if evt.get("error") == "overflow":
                                if on_gap:
                                    try: on_gap(last)
                                    except Exception: pass
                                break  # reconnect
                            commit = evt.get("commit_seq") or evt.get("commit") or last
                            if commit and int(commit) <= last:
                                continue
                            last = int(commit)
                            save_checkpoint(last)
                            yield evt
                # overflow or end, backoff and resume
                jitter = random.randint(0, max(1, backoff//4))
                time.sleep((backoff + jitter) / 1000.0)
                backoff = min(backoff_max_ms, max(backoff_min_ms, backoff * 2))
            except requests.RequestException:
                jitter = random.randint(0, max(1, backoff//4))
                time.sleep((backoff + jitter) / 1000.0)
                backoff = min(backoff_max_ms, max(backoff_min_ms, backoff * 2))

    def lease_acquire(self, key: str, owner: str, ttl: int):
        r = requests.post(f"{self.base_url}/v1/{self.namespace}/lease/acquire", json={"key": key, "owner": owner, "ttl": ttl})
        r.raise_for_status()
        return r.json()

    def lease_renew(self, key: str, owner: str, token: int, ttl: int):
        r = requests.post(f"{self.base_url}/v1/{self.namespace}/lease/renew", json={"key": key, "owner": owner, "token": token, "ttl": ttl})
        r.raise_for_status()
        return r.json()

    def lease_release(self, key: str, owner: str, token: int):
        r = requests.post(f"{self.base_url}/v1/{self.namespace}/lease/release", json={"key": key, "owner": owner, "token": token})
        r.raise_for_status()
        return True
