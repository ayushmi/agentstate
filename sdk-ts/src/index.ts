import axios, { AxiosInstance } from 'axios';

export type Tags = Record<string, string>;

/** A single rule in a namespace invariant spec. */
export interface InvariantRule {
  field: string;                      // "body.<key>" or "tags.<key>"
  required?: boolean;
  type?: 'string' | 'number' | 'bool' | 'array' | 'object';
  eq?: any;
  one_of?: any[];
  gte?: number;
  lte?: number;
  gt?: number;
  lt?: number;
  regex?: string;
}

/** Namespace invariant spec — a set of rules enforced on every write. */
export interface InvariantSpec {
  rules: InvariantRule[];
}

// ── Claim Verification Types ────────────────────────────────────────────────

export interface ClaimAssertion {
  predicate: string;
  subject?: any;
  object?: any;
  params?: any;
}

export interface PremiseRef {
  kind: 'source' | 'wal_state' | 'domain_axiom' | 'prior_claim';
  id?: string;
  role?: string;
  ns?: string;
  object_id?: string;
  field?: string;
  at_commit?: string;
  axiom_id?: string;
  claim_id?: string;
}

export interface Consequence {
  predicate: any;
  check_after_hours?: number;
  status?: 'pending' | 'holds' | 'violated';
}

export interface ClaimRequest {
  domain: string;
  template?: string;
  assertion: ClaimAssertion;
  premises?: PremiseRef[];
  consequences?: Consequence[];
  scope?: { namespace: string; valid_until?: string };
  cause?: { actor?: string; trigger?: string; note?: string };
}

export interface InferenceStep {
  step_id: number;
  kind: 'ground' | 'inference' | 'conclusion';
  rule: string;
  premises_used: number[];
  conclusion_predicate: string;
  conclusion_desc: string;
  justified_by?: string;
}

export interface ProofProperties {
  self_consistent: boolean;
  minimal: boolean;
  has_predictive_constraint: boolean;
  verifiable: boolean;
  sound: boolean;
  monotonic: boolean;
}

export interface Proof {
  proof_id: string;
  claim_id: string;
  ns: string;
  domain: string;
  status: 'proving' | 'proved' | 'refuted' | 'inconclusive' | 'challenged';
  properties: ProofProperties;
  conclusion: any;
  confidence: string;
  steps: InferenceStep[];
  assumptions: string[];
  refutation_reason?: string;
  ts: string;
  commit: string;
}

export interface DomainInfo {
  domain: string;
  version: string;
  description: string;
  templates: string[];
}

/** Records why a state change happened. All fields are optional. */
export interface Cause {
  actor?: string;   // agent ID making this change
  trigger?: string; // commit hash that triggered this change
  note?: string;    // human-readable reason
}

export interface Agent {
  id: string;
  type: string;
  body: any;
  tags: Tags;
  commit_seq: number;
  commit_ts: string;
  cause?: Cause;
}

/**
 * AgentState TypeScript SDK - "Firebase for AI Agents"
 * 
 * Provides a simple interface for managing AI agent state with:
 * - Real-time state updates
 * - Rich querying by tags
 * - Persistent storage
 * - High performance and reliability
 * 
 * @example
 * ```typescript
 * import { AgentStateClient } from 'agentstate';
 * 
 * const client = new AgentStateClient('http://localhost:8080', 'my-app', 'your-api-key');
 * 
 * // Create an agent
 * const agent = await client.createAgent('chatbot', {
 *   name: 'CustomerBot',
 *   status: 'active'
 * }, {
 *   team: 'support'
 * });
 * 
 * // Query agents
 * const agents = await client.queryAgents({ team: 'support' });
 * ```
 */
export class AgentStateClient {
  private baseUrl: string;
  private namespace: string;
  private http: AxiosInstance;

  /**
   * Initialize AgentState client.
   * 
   * @param baseUrl AgentState server URL (e.g., "http://localhost:8080")
   * @param namespace Namespace for organizing agents (e.g., "production", "staging")
   * @param apiKey API key for authentication (optional, can also be set via AGENTSTATE_API_KEY env var)
   */
  constructor(baseUrl: string = 'http://localhost:8080', namespace: string = 'default', apiKey?: string) {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.namespace = namespace;
    
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'User-Agent': 'agentstate-typescript-sdk/1.0.1'
    };
    
    // Set up authentication if API key is provided
    const key = apiKey || process.env.AGENTSTATE_API_KEY;
    if (key) {
      headers['Authorization'] = `Bearer ${key}`;
    }
    
    this.http = axios.create({
      headers,
      timeout: 30000
    });
  }

  /**
   * Create or update an agent.
   * 
   * @param agentType Type of agent (e.g., "chatbot", "workflow", "classifier")
   * @param body Agent state data (any JSON-serializable object)
   * @param tags Key-value pairs for querying and organization
   * @param agentId Specific ID to use (for updates), auto-generated if undefined
   * @returns Created agent object with id, type, body, tags, commit_seq, commit_ts
   */
  async createAgent(
    agentType: string,
    body: any,
    tags?: Tags,
    agentId?: string,
    cause?: Cause
  ): Promise<Agent> {
    const payload: any = {
      type: agentType,
      body,
      tags: tags || {}
    };

    if (agentId) payload.id = agentId;
    if (cause) payload.cause = cause;

    const response = await this.http.post(
      `${this.baseUrl}/v1/${this.namespace}/objects`,
      payload
    );

    return response.data;
  }

  /**
   * Get agent by ID.
   * 
   * @param agentId Unique agent identifier
   * @returns Agent object with id, type, body, tags, commit_seq, commit_ts
   */
  async getAgent(agentId: string): Promise<Agent> {
    const response = await this.http.get(
      `${this.baseUrl}/v1/${this.namespace}/objects/${agentId}`
    );
    
    return response.data;
  }

  /**
   * Query agents by tags.
   * 
   * @param tags Tag filters (e.g., { team: "support", status: "active" })
   * @returns List of matching agent objects
   */
  async queryAgents(tags?: Tags): Promise<Agent[]> {
    const query: any = {};
    if (tags) {
      query.tags = tags;
    }

    const response = await this.http.post(
      `${this.baseUrl}/v1/${this.namespace}/query`,
      query
    );
    
    return response.data;
  }

  /**
   * Delete an agent.
   * 
   * @param agentId Unique agent identifier
   */
  async deleteAgent(agentId: string): Promise<void> {
    await this.http.delete(
      `${this.baseUrl}/v1/${this.namespace}/objects/${agentId}`
    );
  }

  /**
   * Check if AgentState server is healthy.
   * 
   * @returns True if server is healthy, false otherwise
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.http.get(`${this.baseUrl}/health`, { timeout: 5000 });
      return response.status === 200 && response.data.trim() === 'ok';
    } catch {
      return false;
    }
  }

  // ── Invariant management ────────────────────────────────────────────────────

  /**
   * Set a namespace invariant that is enforced on every write.
   * @param ns - Namespace to protect (can differ from the client's namespace).
   * @param rules - Array of rule objects, e.g. [{field:"body.status", required:true}]
   * @returns The stored spec as returned by the server.
   */
  async setInvariant(ns: string, rules: InvariantRule[]): Promise<InvariantSpec> {
    const resp = await this.http.post(`${this.baseUrl}/admin/namespaces/${ns}/invariants`, { rules });
    return resp.data;
  }

  /**
   * Retrieve the current invariant spec for a namespace.
   * @returns The spec, or null if no invariant is set.
   */
  async getInvariant(ns: string): Promise<InvariantSpec | null> {
    try {
      const resp = await this.http.get(`${this.baseUrl}/admin/namespaces/${ns}/invariants`);
      return resp.data;
    } catch (e: any) {
      if (e?.response?.status === 404) return null;
      throw e;
    }
  }

  // Legacy API compatibility
  private get base(): string {
    return `${this.baseUrl}/v1/${this.namespace}`;
  }

  /**
   * @deprecated Use createAgent() instead
   */
  async put(type: string, body: any, tags?: Tags, ttl_seconds?: number, id?: string, cause?: Cause): Promise<Agent> {
    return this.createAgent(type, body, tags, id, cause);
  }

  // ── Claim Verification ────────────────────────────────────────────────────

  /** Submit a claim for formal verification. Returns the claim and proof. */
  async submitClaim(ns: string, req: ClaimRequest): Promise<{ claim: any; proof: Proof }> {
    const resp = await this.http.post(`${this.baseUrl}/admin/namespaces/${ns}/claims`, req);
    return resp.data;
  }

  /** Get a stored claim by ID. */
  async getClaim(ns: string, claimId: string): Promise<any> {
    const resp = await this.http.get(`${this.baseUrl}/admin/namespaces/${ns}/claims/${claimId}`);
    return resp.data;
  }

  /** Get the formal proof artifact for a claim. */
  async getProof(ns: string, claimId: string): Promise<Proof> {
    const resp = await this.http.get(`${this.baseUrl}/admin/namespaces/${ns}/claims/${claimId}/proof`);
    return resp.data;
  }

  /** List all claims in a namespace. */
  async listClaims(ns: string): Promise<any[]> {
    const resp = await this.http.get(`${this.baseUrl}/admin/namespaces/${ns}/claims`);
    return resp.data;
  }

  /** Submit a challenge against a claim's proof. */
  async challengeClaim(ns: string, claimId: string, reason: string, opts?: {
    challenged_step?: number;
    counter_evidence?: string[];
  }): Promise<any> {
    const resp = await this.http.post(`${this.baseUrl}/admin/namespaces/${ns}/claims/${claimId}/challenge`, {
      reason,
      challenged_step: opts?.challenged_step,
      counter_evidence: opts?.counter_evidence ?? [],
    });
    return resp.data;
  }

  /** List all available domain packs. */
  async listDomains(): Promise<DomainInfo[]> {
    const resp = await this.http.get(`${this.baseUrl}/admin/domains`);
    return resp.data;
  }

  /**
   * @deprecated Use getAgent() instead
   */
  async get(id: string): Promise<Agent> {
    return this.getAgent(id);
  }

  /**
   * @deprecated Use queryAgents() instead
   */
  async query(tag_filter?: Tags): Promise<Agent[]> {
    return this.queryAgents(tag_filter);
  }
}

// Legacy State class for backward compatibility
export class State extends AgentStateClient {
  constructor(base: string) {
    // Extract namespace from legacy format: "http://host:8080/v1/namespace"
    const url = new URL(base);
    const pathParts = url.pathname.split('/');
    const namespace = pathParts[pathParts.length - 1] || 'default';
    const baseUrl = `${url.protocol}//${url.host}`;
    
    super(baseUrl, namespace);
  }
}

// Default export
export default AgentStateClient;