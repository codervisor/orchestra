import { describe, it, expect } from "vitest";
import { interpolate, resolveVariable } from "../vars.js";
import type { ExecutionContext } from "../types.js";

function makeContext(overrides?: Partial<ExecutionContext>): ExecutionContext {
  return {
    config: { name: "factory", max_rework: 3 },
    results: new Map(),
    variables: new Map(),
    ...overrides,
  };
}

describe("interpolate", () => {
  it("interpolates config variables", () => {
    const ctx = makeContext();
    expect(interpolate("Pipeline: ${config.name}", ctx)).toBe("Pipeline: factory");
  });

  it("interpolates nested config", () => {
    const ctx = makeContext();
    expect(interpolate("Max: ${config.max_rework}", ctx)).toBe("Max: 3");
  });

  it("interpolates step results", () => {
    const ctx = makeContext();
    ctx.results.set("build", {
      status: "success",
      output: { verdict: "APPROVE", files: ["a.ts"] },
      duration_ms: 100,
    });
    expect(interpolate("Verdict: ${steps.build.verdict}", ctx)).toBe(
      "Verdict: APPROVE"
    );
  });

  it("interpolates step status", () => {
    const ctx = makeContext();
    ctx.results.set("gate", {
      status: "failure",
      output: "test failed",
      duration_ms: 50,
    });
    expect(interpolate("Gate: ${steps.gate.status}", ctx)).toBe("Gate: failure");
  });

  it("throws on unresolved variable", () => {
    const ctx = makeContext();
    expect(() => interpolate("${config.nonexistent}", ctx)).toThrow(
      "Unresolved variable"
    );
  });

  it("interpolates explicit variables", () => {
    const ctx = makeContext({
      variables: new Map([["spec.path", "/path/to/spec.md"]]),
    });
    expect(interpolate("Spec: ${spec.path}", ctx)).toBe("Spec: /path/to/spec.md");
  });

  it("handles multiple variables in one template", () => {
    const ctx = makeContext();
    expect(
      interpolate("${config.name} max=${config.max_rework}", ctx)
    ).toBe("factory max=3");
  });

  it("returns string unchanged when no variables", () => {
    const ctx = makeContext();
    expect(interpolate("no vars here", ctx)).toBe("no vars here");
  });
});

describe("resolveVariable", () => {
  it("resolves config paths", () => {
    const ctx = makeContext();
    expect(resolveVariable("config.name", ctx)).toBe("factory");
  });

  it("resolves step output", () => {
    const ctx = makeContext();
    ctx.results.set("build", {
      status: "success",
      output: "build done",
      duration_ms: 100,
    });
    expect(resolveVariable("steps.build.output", ctx)).toBe("build done");
  });

  it("returns undefined for missing", () => {
    const ctx = makeContext();
    expect(resolveVariable("steps.missing.output", ctx)).toBeUndefined();
  });
});
