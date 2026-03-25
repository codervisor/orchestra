/**
 * Core DAG walker / graph executor.
 * Walks the orchestration graph, executing nodes and following edges.
 */

import type {
  ExecutionContext,
  ExecutionResult,
  ExecutorRegistry,
  GraphEdge,
  GraphNode,
  NodeResult,
  OrchestrationGraph,
} from "./types.js";
import { interpolate } from "./vars.js";

/**
 * Execute an orchestration graph from its entry node to completion.
 */
export async function execute(
  graph: OrchestrationGraph,
  executors: ExecutorRegistry
): Promise<ExecutionResult> {
  const edgeTraversals = new Map<string, number>();
  let current = findEntryNode(graph);

  while (current) {
    // Check condition middleware
    if (current.middleware?.condition) {
      const condValue = interpolate(
        `\${${current.middleware.condition}}`,
        graph.context
      );
      if (
        condValue === "" ||
        condValue === "false" ||
        condValue === "0" ||
        condValue === "null"
      ) {
        graph.context.results.set(current.id, {
          status: "skipped",
          duration_ms: 0,
        });
        current = advanceToNext(current, graph, edgeTraversals);
        continue;
      }
    }

    // Execute with retries
    let result: NodeResult;
    const maxRetries = current.middleware?.retry ?? 0;
    let attempt = 0;

    while (true) {
      const executor = executors[current.type];
      const start = Date.now();

      try {
        if (current.middleware?.timeout) {
          result = await withTimeout(
            executor.execute(current, graph.context),
            current.middleware.timeout
          );
        } else {
          result = await executor.execute(current, graph.context);
        }
      } catch (err) {
        result = {
          status: "failure",
          output: err instanceof Error ? err.message : String(err),
          duration_ms: Date.now() - start,
        };
      }

      result.duration_ms = result.duration_ms || Date.now() - start;

      if (result.status === "success" || attempt >= maxRetries) break;
      attempt++;
    }

    graph.context.results.set(current.id, result);

    // Handle failure with on_fail
    if (result.status === "failure" && current.middleware?.on_fail) {
      const onFail = current.middleware.on_fail;
      if (onFail === "escalate") {
        return {
          status: "failed",
          context: graph.context,
          error: `Node '${current.id}' failed and escalated`,
        };
      }
      // rework(target) syntax
      const reworkMatch = onFail.match(/^rework\((.+)\)$/);
      if (reworkMatch) {
        const target = reworkMatch[1];
        const targetNode = graph.nodes.get(target);
        if (!targetNode) {
          return {
            status: "failed",
            context: graph.context,
            error: `on_fail rework target '${target}' not found`,
          };
        }
        current = targetNode;
        continue;
      }
    }

    // Resolve next node via edges
    const nextEdge = resolveNextEdge(
      current,
      result,
      graph.edges,
      graph.context,
      edgeTraversals
    );

    if (!nextEdge) {
      // No outgoing edge — graph complete
      return { status: "completed", context: graph.context };
    }

    // Check loop cap
    const eKey = edgeKey(nextEdge);
    const traversals = edgeTraversals.get(eKey) ?? 0;

    if (nextEdge.max && traversals >= nextEdge.max) {
      if (nextEdge.exhaust) {
        if (nextEdge.exhaust === "escalate") {
          return {
            status: "exhausted",
            context: graph.context,
            error: `Edge ${eKey} exhausted after ${nextEdge.max} traversals`,
          };
        }
        // exhaust points to a node
        const exhaustNode = graph.nodes.get(nextEdge.exhaust);
        if (exhaustNode) {
          current = exhaustNode;
          continue;
        }
      }
      return {
        status: "exhausted",
        context: graph.context,
        error: `Edge ${eKey} exhausted after ${nextEdge.max} traversals`,
      };
    }

    edgeTraversals.set(eKey, traversals + 1);
    current = graph.nodes.get(nextEdge.to) ?? null;
  }

  return { status: "completed", context: graph.context };
}

/**
 * Find the entry node — the node that has no incoming edges.
 */
export function findEntryNode(graph: OrchestrationGraph): GraphNode | null {
  const hasIncoming = new Set<string>();
  for (const edge of graph.edges) {
    hasIncoming.add(edge.to);
  }

  for (const [id, node] of graph.nodes) {
    if (!hasIncoming.has(id)) {
      return node;
    }
  }

  // Fallback: first node
  const first = graph.nodes.values().next();
  return first.done ? null : first.value;
}

/**
 * Resolve which edge to follow after executing a node.
 * For decision nodes, uses the result output to match edge conditions.
 * For other nodes, follows the unconditional edge.
 */
function resolveNextEdge(
  current: GraphNode,
  result: NodeResult,
  edges: GraphEdge[],
  context: ExecutionContext,
  _traversals: Map<string, number>
): GraphEdge | null {
  const outgoing = edges.filter((e) => e.from === current.id);
  if (outgoing.length === 0) return null;

  // Decision node: match condition from result
  if (current.type === "decision") {
    const verdict = extractVerdict(result);
    // Find edge matching the verdict
    const matched = outgoing.find(
      (e) => e.condition && verdict.toLowerCase().includes(e.condition.toLowerCase())
    );
    if (matched) return matched;
    // Fall through to unconditional edge
  }

  // Unconditional edge (no condition)
  const unconditional = outgoing.find((e) => !e.condition);
  if (unconditional) return unconditional;

  // If all edges are conditional and none matched, return first edge as fallback
  return outgoing[0] ?? null;
}

/**
 * Advance to the next node following the default (unconditional) edge.
 * Used when a node is skipped.
 */
function advanceToNext(
  current: GraphNode,
  graph: OrchestrationGraph,
  traversals: Map<string, number>
): GraphNode | null {
  const outgoing = graph.edges.filter((e) => e.from === current.id);
  const unconditional = outgoing.find((e) => !e.condition);
  if (!unconditional) return null;

  const eKey = edgeKey(unconditional);
  traversals.set(eKey, (traversals.get(eKey) ?? 0) + 1);
  return graph.nodes.get(unconditional.to) ?? null;
}

function extractVerdict(result: NodeResult): string {
  if (!result.output) return "";
  if (typeof result.output === "string") return result.output;
  if (typeof result.output === "object" && result.output !== null) {
    const obj = result.output as Record<string, unknown>;
    if (typeof obj.verdict === "string") return obj.verdict;
    // Try nested access
    return JSON.stringify(result.output);
  }
  return String(result.output);
}

function edgeKey(edge: GraphEdge): string {
  return `${edge.from}->${edge.to}${edge.condition ? `[${edge.condition}]` : ""}`;
}

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Timed out after ${timeoutMs}ms`)), timeoutMs)
    ),
  ]);
}
