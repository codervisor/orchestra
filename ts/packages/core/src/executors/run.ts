/**
 * Run node executor.
 * Executes shell commands and gate checks.
 */

import { exec } from "node:child_process";
import { promisify } from "node:util";
import type {
  ExecutionContext,
  GraphNode,
  NodeExecutor,
  NodeResult,
  RunConfig,
} from "../types.js";
import { interpolate } from "../vars.js";

const execAsync = promisify(exec);

export class RunExecutor implements NodeExecutor {
  private gatesPath: string;

  constructor(gatesPath = "gates.yml") {
    this.gatesPath = gatesPath;
  }

  async execute(node: GraphNode, context: ExecutionContext): Promise<NodeResult> {
    const config = node.config as RunConfig;
    const start = Date.now();

    if (config.check) {
      return this.executeGateCheck(config, context, start);
    }

    if (config.command) {
      return this.executeCommand(config, context, start);
    }

    return {
      status: "failure",
      output: "Run node has neither command nor check",
      duration_ms: Date.now() - start,
    };
  }

  private async executeCommand(
    config: RunConfig,
    context: ExecutionContext,
    start: number
  ): Promise<NodeResult> {
    const command = interpolate(config.command!, context);

    try {
      const { stdout, stderr } = await execAsync(command, {
        maxBuffer: 10 * 1024 * 1024,
        timeout: 300_000,
      });

      let output: unknown = stdout.trim();
      try {
        output = JSON.parse(stdout);
      } catch {
        // keep as string
      }

      return {
        status: "success",
        output,
        stdout,
        stderr,
        duration_ms: Date.now() - start,
      };
    } catch (err: unknown) {
      const execErr = err as { stdout?: string; stderr?: string; message?: string };
      return {
        status: "failure",
        output: execErr.stderr || execErr.message || String(err),
        stdout: execErr.stdout,
        stderr: execErr.stderr,
        duration_ms: Date.now() - start,
      };
    }
  }

  private async executeGateCheck(
    config: RunConfig,
    context: ExecutionContext,
    start: number
  ): Promise<NodeResult> {
    // Get changed files
    let changedFiles: string[] = [];
    try {
      const { stdout } = await execAsync(
        "git diff --name-only HEAD~1 HEAD 2>/dev/null || git diff --name-only HEAD",
        { maxBuffer: 1024 * 1024 }
      );
      changedFiles = stdout.trim().split("\n").filter(Boolean);
    } catch {
      changedFiles = [];
    }

    // Filter by match patterns if specified
    if (config.match?.length) {
      changedFiles = filterByGlob(changedFiles, config.match);
    }

    if (changedFiles.length === 0) {
      return {
        status: "success",
        output: { passed: true, skipped: true, message: "No matching files changed" },
        duration_ms: Date.now() - start,
      };
    }

    // Execute gate commands
    // For now, run checks as a simple command (gate system integration)
    const checkGroups = config.check!;
    const failures: string[] = [];

    for (const group of checkGroups) {
      try {
        await execAsync(`orchestra gate run ${group}`, {
          maxBuffer: 5 * 1024 * 1024,
          timeout: 300_000,
        });
      } catch (err: unknown) {
        const execErr = err as { stderr?: string; message?: string };
        failures.push(`Gate '${group}' failed: ${execErr.stderr || execErr.message}`);
      }
    }

    return {
      status: failures.length === 0 ? "success" : "failure",
      output: {
        passed: failures.length === 0,
        failures,
        files_checked: changedFiles,
      },
      duration_ms: Date.now() - start,
    };
  }
}

/**
 * Simple glob-style matching for file paths.
 */
function filterByGlob(files: string[], patterns: string[]): string[] {
  return files.filter((file) =>
    patterns.some((pattern) => matchGlob(file, pattern))
  );
}

function matchGlob(file: string, pattern: string): boolean {
  // Simple glob: *.ext matches any file with that extension
  if (pattern.startsWith("*.")) {
    return file.endsWith(pattern.slice(1));
  }
  // **/*.ext matches recursively
  if (pattern.startsWith("**/*.")) {
    return file.endsWith(pattern.slice(4));
  }
  return file.includes(pattern);
}
