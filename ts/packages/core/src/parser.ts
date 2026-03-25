/**
 * YAML parser for graph definitions.
 * Parses YAML into validated GraphDefinition and resolves into OrchestrationGraph.
 */

import { parse as parseYaml } from "yaml";
import { GraphDefinitionSchema } from "./schema.js";
import type {
  GraphDefinition,
  GraphEdge,
  GraphNode,
  OrchestrationGraph,
  EdgeDefinition,
} from "./types.js";

/**
 * Parse a YAML string into a validated GraphDefinition.
 */
export function parseGraphYaml(yamlContent: string): GraphDefinition {
  const raw = parseYaml(yamlContent);
  return GraphDefinitionSchema.parse(raw);
}

/**
 * Resolve edge definitions into flat GraphEdge array.
 * Chain edges like `{ chain: ["a", "b", "c"] }` expand to
 * `[{from:"a", to:"b"}, {from:"b", to:"c"}]`.
 */
export function resolveEdges(defs: EdgeDefinition[]): GraphEdge[] {
  const edges: GraphEdge[] = [];

  for (const def of defs) {
    if ("chain" in def) {
      for (let i = 0; i < def.chain.length - 1; i++) {
        edges.push({ from: def.chain[i], to: def.chain[i + 1] });
      }
    } else {
      edges.push({
        from: def.from,
        to: def.to,
        condition: def.on,
        max: def.max,
        exhaust: def.exhaust,
      });
    }
  }

  return edges;
}

/**
 * Build an OrchestrationGraph from a GraphDefinition.
 */
export function buildGraph(
  definition: GraphDefinition,
  initialConfig?: Record<string, unknown>
): OrchestrationGraph {
  const nodes = new Map<string, GraphNode>();

  for (const [id, nodeDef] of Object.entries(definition.nodes)) {
    nodes.set(id, { id, ...nodeDef } as GraphNode);
  }

  const edges = resolveEdges(definition.edges);

  // Validate edge references
  for (const edge of edges) {
    if (!nodes.has(edge.from)) {
      throw new Error(`Edge references unknown node: '${edge.from}'`);
    }
    if (!nodes.has(edge.to)) {
      throw new Error(`Edge references unknown node: '${edge.to}'`);
    }
  }

  return {
    name: definition.name,
    nodes,
    edges,
    context: {
      config: { ...definition.config, ...initialConfig },
      results: new Map(),
      variables: new Map(),
    },
  };
}
