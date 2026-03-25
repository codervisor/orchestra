/**
 * Pre-execution validation for orchestration graphs.
 * Checks structural integrity before running.
 */

import type { OrchestrationGraph } from "./types.js";

export interface ValidationError {
  node?: string;
  edge?: string;
  message: string;
}

/**
 * Validate a graph before execution.
 * Returns an array of errors (empty = valid).
 */
export function validateGraph(graph: OrchestrationGraph): ValidationError[] {
  const errors: ValidationError[] = [];

  if (!graph.name) {
    errors.push({ message: "Graph must have a name" });
  }

  if (graph.nodes.size === 0) {
    errors.push({ message: "Graph must have at least one node" });
  }

  // Validate nodes
  for (const [id, node] of graph.nodes) {
    if (node.type === "agent") {
      const config = node.config as { prompt?: string };
      if (!config.prompt) {
        errors.push({ node: id, message: "Agent node must have a prompt" });
      }
    }

    if (node.type === "decision") {
      const config = node.config as { input?: string; cases?: Record<string, string> };
      if (!config.input) {
        errors.push({ node: id, message: "Decision node must have an input" });
      }
      if (!config.cases || Object.keys(config.cases).length === 0) {
        errors.push({ node: id, message: "Decision node must have at least one case" });
      }
    }

    if (node.type === "fan") {
      const config = node.config as { over?: string; body?: string };
      if (!config.over) {
        errors.push({ node: id, message: "Fan node must have 'over'" });
      }
      if (!config.body) {
        errors.push({ node: id, message: "Fan node must have 'body'" });
      }
    }

    // Validate on_fail targets
    if (node.middleware?.on_fail) {
      const onFail = node.middleware.on_fail;
      if (onFail !== "escalate") {
        const match = onFail.match(/^rework\((.+)\)$/);
        if (match) {
          if (!graph.nodes.has(match[1])) {
            errors.push({
              node: id,
              message: `on_fail rework target '${match[1]}' not found`,
            });
          }
        } else {
          errors.push({
            node: id,
            message: `Invalid on_fail value: '${onFail}'. Use 'escalate' or 'rework(target)'`,
          });
        }
      }
    }
  }

  // Validate edges reference existing nodes
  for (const edge of graph.edges) {
    if (!graph.nodes.has(edge.from)) {
      errors.push({
        edge: `${edge.from}->${edge.to}`,
        message: `Edge source '${edge.from}' not found`,
      });
    }
    if (!graph.nodes.has(edge.to)) {
      errors.push({
        edge: `${edge.from}->${edge.to}`,
        message: `Edge target '${edge.to}' not found`,
      });
    }

    if (edge.exhaust && edge.exhaust !== "escalate" && !graph.nodes.has(edge.exhaust)) {
      errors.push({
        edge: `${edge.from}->${edge.to}`,
        message: `Edge exhaust target '${edge.exhaust}' not found`,
      });
    }
  }

  // Check for entry node
  const hasIncoming = new Set(graph.edges.map((e) => e.to));
  const entryNodes = [...graph.nodes.keys()].filter((id) => !hasIncoming.has(id));
  if (entryNodes.length === 0) {
    errors.push({ message: "Graph has no entry node (all nodes have incoming edges — possible cycle)" });
  }
  if (entryNodes.length > 1) {
    errors.push({
      message: `Graph has multiple entry nodes: ${entryNodes.join(", ")}. Expected exactly one.`,
    });
  }

  return errors;
}
