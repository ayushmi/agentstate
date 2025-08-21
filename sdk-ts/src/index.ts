import axios, { AxiosInstance } from 'axios';

export type Tags = Record<string, string>;

export interface Agent {
  id: string;
  type: string;
  body: any;
  tags: Tags;
  commit_seq: number;
  commit_ts: string;
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
    agentId?: string
  ): Promise<Agent> {
    const payload: any = {
      type: agentType,
      body,
      tags: tags || {}
    };
    
    if (agentId) {
      payload.id = agentId;
    }

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

  // Legacy API compatibility
  private get base(): string {
    return `${this.baseUrl}/v1/${this.namespace}`;
  }

  /**
   * @deprecated Use createAgent() instead
   */
  async put(type: string, body: any, tags?: Tags, ttl_seconds?: number, id?: string): Promise<Agent> {
    return this.createAgent(type, body, tags, id);
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