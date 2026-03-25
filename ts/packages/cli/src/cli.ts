#!/usr/bin/env node

/**
 * Orchestra CLI — Run graph-based orchestration workflows.
 *
 * Usage:
 *   orchestra-ts run <graph.yml> [--config key=value ...]
 *   orchestra-ts validate <graph.yml>
 */

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import {
  parseGraphYaml,
  buildGraph,
  validateGraph,
  execute,
  AgentExecutor,
  RunExecutor,
  DecisionExecutor,
  FanExecutor,
} from "@orchestra/core";
import type { ExecutorRegistry } from "@orchestra/core";

async function main() {
  const args = process.argv.slice(2);
  const command = args[0];

  if (!command || command === "--help" || command === "-h") {
    printUsage();
    process.exit(0);
  }

  switch (command) {
    case "run":
      await runGraph(args.slice(1));
      break;
    case "validate":
      await validateGraphCmd(args.slice(1));
      break;
    default:
      console.error(`Unknown command: ${command}`);
      printUsage();
      process.exit(1);
  }
}

async function runGraph(args: string[]) {
  const graphPath = args[0];
  if (!graphPath) {
    console.error("Error: graph file path required");
    process.exit(1);
  }

  // Parse --config flags
  const config: Record<string, unknown> = {};
  for (let i = 1; i < args.length; i++) {
    if (args[i] === "--config" && args[i + 1]) {
      const [key, value] = args[i + 1].split("=", 2);
      config[key] = value;
      i++;
    }
  }

  const yamlContent = await readFile(resolve(graphPath), "utf-8");
  const definition = parseGraphYaml(yamlContent);
  const graph = buildGraph(definition, config);

  // Validate
  const errors = validateGraph(graph);
  if (errors.length > 0) {
    console.error("Validation errors:");
    for (const err of errors) {
      const prefix = err.node ? `[${err.node}]` : err.edge ? `[${err.edge}]` : "";
      console.error(`  ${prefix} ${err.message}`);
    }
    process.exit(1);
  }

  // Build executors
  const agentExecutor = new AgentExecutor();
  const runExecutor = new RunExecutor();
  const decisionExecutor = new DecisionExecutor();
  const fanExecutor = new FanExecutor(agentExecutor, (id) => graph.nodes.get(id));

  const executors: ExecutorRegistry = {
    agent: agentExecutor,
    run: runExecutor,
    decision: decisionExecutor,
    fan: fanExecutor,
  };

  console.log(`Executing graph: ${graph.name}`);
  const result = await execute(graph, executors);

  console.log(`\nResult: ${result.status}`);
  if (result.error) {
    console.error(`Error: ${result.error}`);
    process.exit(1);
  }

  // Print summary of node results
  for (const [nodeId, nodeResult] of result.context.results) {
    const icon = nodeResult.status === "success" ? "✓" : nodeResult.status === "skipped" ? "○" : "✗";
    console.log(`  ${icon} ${nodeId}: ${nodeResult.status} (${nodeResult.duration_ms}ms)`);
  }
}

async function validateGraphCmd(args: string[]) {
  const graphPath = args[0];
  if (!graphPath) {
    console.error("Error: graph file path required");
    process.exit(1);
  }

  const yamlContent = await readFile(resolve(graphPath), "utf-8");
  const definition = parseGraphYaml(yamlContent);
  const graph = buildGraph(definition);

  const errors = validateGraph(graph);
  if (errors.length === 0) {
    console.log("Graph is valid.");
  } else {
    console.error(`Found ${errors.length} validation error(s):`);
    for (const err of errors) {
      const prefix = err.node ? `[${err.node}]` : err.edge ? `[${err.edge}]` : "";
      console.error(`  ${prefix} ${err.message}`);
    }
    process.exit(1);
  }
}

function printUsage() {
  console.log(`
Orchestra TS — Graph-based orchestration engine

Usage:
  orchestra-ts run <graph.yml> [--config key=value ...]
  orchestra-ts validate <graph.yml>

Commands:
  run        Execute an orchestration graph
  validate   Check graph definition for errors
`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
