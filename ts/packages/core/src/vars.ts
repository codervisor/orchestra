/**
 * Variable interpolation for the orchestration engine.
 * Supports ${scope.field} patterns with config, steps, spec, loop, manifest scopes.
 */

import type { ExecutionContext } from "./types.js";

const VAR_PATTERN = /\$\{([^}]+)\}/g;

/**
 * Interpolate variables in a template string using the execution context.
 * Supports dot-notation paths: ${config.name}, ${steps.build.output}, etc.
 */
export function interpolate(
  template: string,
  context: ExecutionContext
): string {
  return template.replace(VAR_PATTERN, (match, path: string) => {
    const value = resolveVariable(path, context);
    if (value === undefined) {
      throw new Error(`Unresolved variable: ${match}`);
    }
    return String(value);
  });
}

/**
 * Resolve a dot-notation variable path against the execution context.
 */
export function resolveVariable(
  path: string,
  context: ExecutionContext
): unknown {
  // Check explicit variables map first
  const direct = context.variables.get(path);
  if (direct !== undefined) return direct;

  const parts = path.split(".");
  const scope = parts[0];
  const rest = parts.slice(1);

  switch (scope) {
    case "config":
      return resolvePath(context.config, rest);

    case "steps": {
      const stepId = parts[1];
      const result = context.results.get(stepId);
      if (!result) return undefined;
      const fieldParts = parts.slice(2);
      const firstField = fieldParts[0];
      if (firstField === "status") return result.status;
      if (firstField === "stdout") return result.stdout;
      if (firstField === "stderr") return result.stderr;
      if (firstField === "output") {
        if (fieldParts.length === 1) return stringify(result.output);
        // Deep access into output: steps.build.output.verdict
        if (result.output && typeof result.output === "object") {
          return resolvePath(result.output as Record<string, unknown>, fieldParts.slice(1));
        }
        return undefined;
      }
      // Direct field access into output (e.g. steps.build.verdict)
      if (result.output && typeof result.output === "object") {
        return resolvePath(result.output as Record<string, unknown>, fieldParts);
      }
      return undefined;
    }

    default:
      // Try full path in variables map
      return context.variables.get(path);
  }
}

function resolvePath(
  obj: Record<string, unknown>,
  parts: string[]
): unknown {
  let current: unknown = obj;
  for (const part of parts) {
    if (current === null || current === undefined) return undefined;
    if (typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return current;
}

function stringify(value: unknown): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}
