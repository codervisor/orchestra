/**
 * @orchestra/algorithms
 *
 * Typed wrappers around Rust CLI binaries for orchestration algorithms.
 * Each algorithm tool follows the JSON-in/JSON-out pattern via stdin/stdout.
 */

import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

// ─── Generic Caller ────────────────────────────────────────────────────────

/**
 * Call a Rust algorithm CLI binary with JSON input, returning parsed JSON output.
 */
export async function callAlgorithm<I, O>(tool: string, input: I): Promise<O> {
  const binName = `orchestra`;
  const inputJson = JSON.stringify(input);

  // Determine the subcommand based on the tool name
  const [category, subcmd] = parseToolName(tool);

  const args = [category, subcmd, "--input", "-"];

  const { stdout, stderr } = await execFileAsync(binName, args, {
    maxBuffer: 10 * 1024 * 1024,
    timeout: 30_000,
    encoding: "utf-8",
  });

  if (stderr && stderr.trim()) {
    // Non-fatal stderr (warnings)
    console.warn(`[orchestra-${tool}] ${stderr.trim()}`);
  }

  try {
    return JSON.parse(stdout) as O;
  } catch {
    throw new Error(
      `Failed to parse JSON output from orchestra ${category} ${subcmd}: ${stdout.slice(0, 200)}`
    );
  }
}

function parseToolName(tool: string): [string, string] {
  // e.g. "fractal.gate" -> ["fractal", "gate"]
  // e.g. "swarm.checkpoint" -> ["swarm", "checkpoint"]
  const parts = tool.split(".");
  if (parts.length === 2) return [parts[0], parts[1]];
  throw new Error(`Invalid tool name: '${tool}'. Expected 'category.subcmd' format.`);
}

// ─── Typed Algorithm Interfaces ────────────────────────────────────────────

// Fractal: Decompose Gate

export interface DecomposeInput {
  parent: {
    scope: string;
    boundaries: string;
    inputs: string;
    outputs: string;
  };
  children: Array<{
    slug: string;
    scope: string;
    boundaries: string;
    inputs: string;
    outputs: string;
  }>;
}

export interface DecomposeFlag {
  kind: string;
  pair?: [string, string];
  detail: string;
}

export interface DecomposeOutput {
  flags: DecomposeFlag[];
  complexity_score: number;
  budget_allocation: Record<string, { files: number; tokens: number }>;
  dependency_order: string[][];
}

export async function decomposeGate(input: DecomposeInput): Promise<DecomposeOutput> {
  return callAlgorithm("fractal.gate", input);
}

// Fractal: Schedule

export interface ScheduleInput {
  tree: Record<
    string,
    {
      slug: string;
      status: string;
      children: string[];
      inputs: string;
      outputs: string;
    }
  >;
}

export interface ScheduleOutput {
  waves: string[][];
  critical_path: string[];
  critical_path_length: number;
  max_parallelism: number;
  total_leaves: number;
}

export async function schedule(input: ScheduleInput): Promise<ScheduleOutput> {
  return callAlgorithm("fractal.schedule", input);
}

// Fractal: Reunify

export interface ReunifyInput {
  base_ref: string;
  children: Array<{
    slug: string;
    branch: string;
    scope: string;
    outputs: string;
    files: string[];
  }>;
}

export interface ReunifyOutput {
  status: string;
  auto_resolved: Array<{ file: string; strategy: string }>;
  conflicts: Array<{ kind: string; detail: string; files: string[] }>;
  merge_order: string[];
  needs_ai: boolean;
}

export async function reunify(input: ReunifyInput): Promise<ReunifyOutput> {
  return callAlgorithm("fractal.reunify", input);
}

// Fractal: Prune

export interface PruneInput {
  tree: Record<
    string,
    {
      slug: string;
      files: string[];
      children: string[];
    }
  >;
}

export interface PruneOutput {
  prunable: string[];
  reasons: Record<string, string>;
  kept: string[];
  file_coverage: Record<string, string[]>;
  identical_pairs: [string, string][];
  minimal_covering_set: string[];
}

export async function prune(input: PruneInput): Promise<PruneOutput> {
  return callAlgorithm("fractal.prune", input);
}

// Fractal: Complexity

export async function complexity(text: string): Promise<{ complexity_score: number }> {
  return callAlgorithm("fractal.complexity", { text });
}

// Swarm: Checkpoint

export interface SwarmManifest {
  id: string;
  branches: Array<{
    id: string;
    strategy: string;
    files: string[];
    status: string;
  }>;
}

export interface CheckpointOutput {
  similarities: Record<string, number>;
  cross_pollination: Record<string, string[]>;
}

export async function checkpoint(manifest: SwarmManifest): Promise<CheckpointOutput> {
  return callAlgorithm("swarm.checkpoint", manifest);
}

// Swarm: Prune

export interface SwarmPruneInput {
  manifest: SwarmManifest;
  threshold: number;
}

export interface SwarmPruneOutput {
  pruned: string[];
  surviving: string[];
}

export async function swarmPrune(input: SwarmPruneInput): Promise<SwarmPruneOutput> {
  return callAlgorithm("swarm.prune", input);
}
