/**
 * Core types for the graph-based orchestration engine.
 */

// ─── Node Types ────────────────────────────────────────────────────────────

export interface AgentConfig {
  prompt: string;
  tools?: string[];
  max_turns?: number;
  isolation?: "worktree";
  output_schema?: string;
  context?: Record<string, string>;
}

export interface RunConfig {
  command?: string;
  check?: string[];
  match?: string[];
  poll?: PollConfig;
}

export interface PollConfig {
  interval: number;
  timeout: number;
}

export interface DecisionConfig {
  input: string;
  cases: Record<string, string>; // verdict value -> target node id
}

export interface FanConfig {
  mode: "parallel" | "sequential";
  over: string;
  body: string; // node id or subgraph name to execute per item
}

export type NodeConfig = AgentConfig | RunConfig | DecisionConfig | FanConfig;

export type NodeType = "agent" | "run" | "decision" | "fan";

export interface GraphNode {
  id: string;
  type: NodeType;
  config: NodeConfig;
  middleware?: NodeMiddleware;
}

export interface NodeMiddleware {
  retry?: number;
  timeout?: number;
  log?: string;
  on_fail?: string;
  condition?: string;
}

// ─── Edge Types ────────────────────────────────────────────────────────────

export interface GraphEdge {
  from: string;
  to: string;
  condition?: string;
  max?: number;
  exhaust?: string;
}

// ─── Graph Definition ──────────────────────────────────────────────────────

export interface GraphDefinition {
  name: string;
  config?: Record<string, unknown>;
  nodes: Record<string, Omit<GraphNode, "id">>;
  edges: EdgeDefinition[];
}

export type EdgeDefinition =
  | { chain: string[] }
  | { from: string; to: string; on?: string; max?: number; exhaust?: string };

// ─── Execution Types ───────────────────────────────────────────────────────

export interface ExecutionContext {
  config: Record<string, unknown>;
  results: Map<string, NodeResult>;
  variables: Map<string, string>;
}

export interface NodeResult {
  status: "success" | "failure" | "skipped";
  output?: unknown;
  stdout?: string;
  stderr?: string;
  duration_ms: number;
}

export interface ExecutionResult {
  status: "completed" | "failed" | "exhausted";
  context: ExecutionContext;
  error?: string;
}

// ─── Orchestration Graph (resolved) ────────────────────────────────────────

export interface OrchestrationGraph {
  name: string;
  nodes: Map<string, GraphNode>;
  edges: GraphEdge[];
  context: ExecutionContext;
}

// ─── Node Executor Interface ───────────────────────────────────────────────

export interface NodeExecutor {
  execute(node: GraphNode, context: ExecutionContext): Promise<NodeResult>;
}

export interface ExecutorRegistry {
  agent: NodeExecutor;
  run: NodeExecutor;
  decision: NodeExecutor;
  fan: NodeExecutor;
}
