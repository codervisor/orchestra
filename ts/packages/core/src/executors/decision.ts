/**
 * Decision node executor.
 * Reads a verdict from a previous step's output and resolves to a case value.
 * The actual routing is handled by the graph executor via edge conditions.
 */

import type {
  DecisionConfig,
  ExecutionContext,
  GraphNode,
  NodeExecutor,
  NodeResult,
} from "../types.js";
import { resolveVariable } from "../vars.js";

export class DecisionExecutor implements NodeExecutor {
  async execute(node: GraphNode, context: ExecutionContext): Promise<NodeResult> {
    const config = node.config as DecisionConfig;
    const start = Date.now();

    // Resolve the input variable
    const inputValue = resolveVariable(config.input, context);

    if (inputValue === undefined) {
      return {
        status: "failure",
        output: `Decision input '${config.input}' could not be resolved`,
        duration_ms: Date.now() - start,
      };
    }

    // Extract the verdict string
    const verdict = extractVerdictString(inputValue);

    // Match against cases
    for (const [pattern, target] of Object.entries(config.cases)) {
      if (verdict.toLowerCase().includes(pattern.toLowerCase())) {
        return {
          status: "success",
          output: { verdict, matched: pattern, target },
          duration_ms: Date.now() - start,
        };
      }
    }

    // No case matched — return the verdict anyway, let edge resolution handle it
    return {
      status: "success",
      output: { verdict, matched: null },
      duration_ms: Date.now() - start,
    };
  }
}

function extractVerdictString(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "object" && value !== null) {
    const obj = value as Record<string, unknown>;
    if (typeof obj.verdict === "string") return obj.verdict;
  }
  return String(value);
}
