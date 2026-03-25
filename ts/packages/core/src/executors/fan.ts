/**
 * Fan node executor.
 * Executes a body node over a collection in parallel or sequential mode.
 */

import type {
  ExecutionContext,
  FanConfig,
  GraphNode,
  NodeExecutor,
  NodeResult,
} from "../types.js";
import { resolveVariable } from "../vars.js";

export class FanExecutor implements NodeExecutor {
  private bodyExecutor: NodeExecutor;
  private resolveNode: (id: string) => GraphNode | undefined;

  constructor(
    bodyExecutor: NodeExecutor,
    resolveNode: (id: string) => GraphNode | undefined
  ) {
    this.bodyExecutor = bodyExecutor;
    this.resolveNode = resolveNode;
  }

  async execute(node: GraphNode, context: ExecutionContext): Promise<NodeResult> {
    const config = node.config as FanConfig;
    const start = Date.now();

    // Resolve the collection to iterate over
    const collection = resolveCollection(config.over, context);
    if (!Array.isArray(collection)) {
      return {
        status: "failure",
        output: `Fan 'over' resolved to non-array: ${typeof collection}`,
        duration_ms: Date.now() - start,
      };
    }

    // Resolve the body node
    const bodyNode = this.resolveNode(config.body);
    if (!bodyNode) {
      return {
        status: "failure",
        output: `Fan body node '${config.body}' not found`,
        duration_ms: Date.now() - start,
      };
    }

    const results: NodeResult[] = [];

    if (config.mode === "parallel") {
      const promises = collection.map((item, index) => {
        const itemContext = createItemContext(context, item, index);
        return this.bodyExecutor.execute(bodyNode, itemContext);
      });
      results.push(...(await Promise.all(promises)));
    } else {
      // Sequential
      for (let i = 0; i < collection.length; i++) {
        const itemContext = createItemContext(context, collection[i], i);
        const result = await this.bodyExecutor.execute(bodyNode, itemContext);
        results.push(result);
      }
    }

    const allSucceeded = results.every((r) => r.status === "success");

    return {
      status: allSucceeded ? "success" : "failure",
      output: results.map((r) => r.output),
      duration_ms: Date.now() - start,
    };
  }
}

function resolveCollection(ref: string, context: ExecutionContext): unknown {
  const value = resolveVariable(ref, context);
  if (Array.isArray(value)) return value;
  if (typeof value === "string") {
    try {
      const parsed = JSON.parse(value);
      if (Array.isArray(parsed)) return parsed;
    } catch {
      // Split comma-separated
      return value.split(",").map((s) => s.trim());
    }
  }
  return value;
}

function createItemContext(
  parent: ExecutionContext,
  item: unknown,
  index: number
): ExecutionContext {
  const variables = new Map(parent.variables);
  variables.set("loop.item", typeof item === "string" ? item : JSON.stringify(item));
  variables.set("loop.index", String(index));

  if (typeof item === "object" && item !== null) {
    for (const [key, val] of Object.entries(item as Record<string, unknown>)) {
      variables.set(`loop.item.${key}`, String(val));
    }
  }

  return {
    config: parent.config,
    results: parent.results,
    variables,
  };
}
