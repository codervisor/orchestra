/**
 * @orchestra/core — Graph-based orchestration engine.
 *
 * Provides a lightweight DAG executor for AI coding workflows.
 */

export type {
  AgentConfig,
  DecisionConfig,
  EdgeDefinition,
  ExecutionContext,
  ExecutionResult,
  ExecutorRegistry,
  FanConfig,
  GraphDefinition,
  GraphEdge,
  GraphNode,
  NodeConfig,
  NodeExecutor,
  NodeMiddleware,
  NodeResult,
  NodeType,
  OrchestrationGraph,
  PollConfig,
  RunConfig,
} from "./types.js";

export { GraphDefinitionSchema } from "./schema.js";
export { parseGraphYaml, buildGraph, resolveEdges } from "./parser.js";
export { execute, findEntryNode } from "./executor.js";
export { interpolate, resolveVariable } from "./vars.js";
export { validateGraph } from "./validate.js";
export type { ValidationError } from "./validate.js";

export {
  AgentExecutor,
  RunExecutor,
  DecisionExecutor,
  FanExecutor,
} from "./executors/index.js";
