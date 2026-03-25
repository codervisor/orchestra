import { describe, it, expect } from "vitest";
import { validateGraph } from "../validate.js";
import { parseGraphYaml, buildGraph } from "../parser.js";

describe("validateGraph", () => {
  it("passes for a valid linear graph", () => {
    const yaml = `
name: valid
nodes:
  a:
    type: run
    config:
      command: "echo a"
  b:
    type: run
    config:
      command: "echo b"
edges:
  - chain: [a, b]
`;
    const graph = buildGraph(parseGraphYaml(yaml));
    const errors = validateGraph(graph);
    expect(errors).toHaveLength(0);
  });

  it("detects missing agent prompt", () => {
    const yaml = `
name: test
nodes:
  a:
    type: agent
    config:
      prompt: ""
edges: []
`;
    // This would fail at Zod parse level, so test at validate level with direct graph
    const graph = buildGraph({
      name: "test",
      nodes: {
        a: { type: "agent", config: { prompt: "" } as any },
      },
      edges: [],
    });
    const errors = validateGraph(graph);
    expect(errors.some((e) => e.message.includes("prompt"))).toBe(true);
  });

  it("detects invalid on_fail target", () => {
    const graph = buildGraph({
      name: "test",
      nodes: {
        a: {
          type: "run",
          config: { command: "echo a" },
          middleware: { on_fail: "rework(nonexistent)" },
        },
      },
      edges: [],
    });
    const errors = validateGraph(graph);
    expect(errors.some((e) => e.message.includes("nonexistent"))).toBe(true);
  });

  it("detects multiple entry nodes", () => {
    const graph = buildGraph({
      name: "test",
      nodes: {
        a: { type: "run", config: { command: "echo a" } },
        b: { type: "run", config: { command: "echo b" } },
      },
      edges: [],
    });
    const errors = validateGraph(graph);
    expect(errors.some((e) => e.message.includes("multiple entry"))).toBe(true);
  });
});
