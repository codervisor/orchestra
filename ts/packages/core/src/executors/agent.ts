/**
 * Agent node executor.
 * Spawns a Claude agent with the configured prompt, tools, and constraints.
 */

import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile } from "node:fs/promises";
import type {
  AgentConfig,
  ExecutionContext,
  GraphNode,
  NodeExecutor,
  NodeResult,
} from "../types.js";
import { interpolate } from "../vars.js";

const execFileAsync = promisify(execFile);

export class AgentExecutor implements NodeExecutor {
  async execute(node: GraphNode, context: ExecutionContext): Promise<NodeResult> {
    const config = node.config as AgentConfig;
    const start = Date.now();

    // Resolve prompt — file path or inline
    let prompt: string;
    if (config.prompt.endsWith(".md") || config.prompt.includes("/")) {
      prompt = await readFile(config.prompt, "utf-8");
    } else {
      prompt = config.prompt;
    }

    // Interpolate variables in prompt
    prompt = interpolate(prompt, context);

    // Build context sections from config.context references
    if (config.context) {
      const sections: string[] = [];
      for (const [label, varRef] of Object.entries(config.context)) {
        const value = interpolate(`\${${varRef}}`, context);
        sections.push(`## ${label}\n${value}`);
      }
      prompt += "\n\n" + sections.join("\n\n");
    }

    // Build claude CLI args
    const args: string[] = ["-p", prompt, "--output-format", "json"];

    if (config.tools?.length) {
      for (const tool of config.tools) {
        args.push("--allowedTools", tool);
      }
    }

    if (config.max_turns) {
      args.push("--max-turns", String(config.max_turns));
    }

    if (config.output_schema) {
      args.push("--output-schema", config.output_schema);
    }

    try {
      const { stdout, stderr } = await execFileAsync("claude", args, {
        maxBuffer: 10 * 1024 * 1024, // 10MB
        timeout: 600_000, // 10 min default
      });

      let output: unknown;
      try {
        output = JSON.parse(stdout);
      } catch {
        output = stdout;
      }

      return {
        status: "success",
        output,
        stdout,
        stderr,
        duration_ms: Date.now() - start,
      };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return {
        status: "failure",
        output: message,
        duration_ms: Date.now() - start,
      };
    }
  }
}
