/**
 * Zod schemas for validating graph definitions loaded from YAML.
 */

import { z } from "zod";

// ─── Node Config Schemas ───────────────────────────────────────────────────

export const AgentConfigSchema = z.object({
  prompt: z.string(),
  tools: z.array(z.string()).optional(),
  max_turns: z.number().int().positive().optional(),
  isolation: z.literal("worktree").optional(),
  output_schema: z.string().optional(),
  context: z.record(z.string()).optional(),
});

export const PollConfigSchema = z.object({
  interval: z.number().int().positive(),
  timeout: z.number().int().positive(),
});

export const RunConfigSchema = z.object({
  command: z.string().optional(),
  check: z.array(z.string()).optional(),
  match: z.array(z.string()).optional(),
  poll: PollConfigSchema.optional(),
}).refine(
  (data) => data.command !== undefined || data.check !== undefined,
  { message: "Run node must have either 'command' or 'check'" }
);

export const DecisionConfigSchema = z.object({
  input: z.string(),
  cases: z.record(z.string()),
});

export const FanConfigSchema = z.object({
  mode: z.enum(["parallel", "sequential"]),
  over: z.string(),
  body: z.string(),
});

// ─── Node Schema ───────────────────────────────────────────────────────────

export const MiddlewareSchema = z.object({
  retry: z.number().int().nonnegative().optional(),
  timeout: z.number().int().positive().optional(),
  log: z.string().optional(),
  on_fail: z.string().optional(),
  condition: z.string().optional(),
});

const BaseNodeSchema = z.object({
  middleware: MiddlewareSchema.optional(),
});

export const AgentNodeSchema = BaseNodeSchema.extend({
  type: z.literal("agent"),
  config: AgentConfigSchema,
});

export const RunNodeSchema = BaseNodeSchema.extend({
  type: z.literal("run"),
  config: RunConfigSchema,
});

export const DecisionNodeSchema = BaseNodeSchema.extend({
  type: z.literal("decision"),
  config: DecisionConfigSchema,
});

export const FanNodeSchema = BaseNodeSchema.extend({
  type: z.literal("fan"),
  config: FanConfigSchema,
});

export const GraphNodeSchema = z.discriminatedUnion("type", [
  AgentNodeSchema,
  RunNodeSchema,
  DecisionNodeSchema,
  FanNodeSchema,
]);

// ─── Edge Schema ───────────────────────────────────────────────────────────

const ChainEdgeSchema = z.object({
  chain: z.array(z.string()).min(2),
});

const ConditionalEdgeSchema = z.object({
  from: z.string(),
  to: z.string(),
  on: z.string().optional(),
  max: z.number().int().positive().optional(),
  exhaust: z.string().optional(),
});

export const EdgeDefinitionSchema = z.union([
  ChainEdgeSchema,
  ConditionalEdgeSchema,
]);

// ─── Graph Schema ──────────────────────────────────────────────────────────

export const GraphDefinitionSchema = z.object({
  name: z.string().min(1),
  config: z.record(z.unknown()).optional(),
  nodes: z.record(GraphNodeSchema),
  edges: z.array(EdgeDefinitionSchema),
});
