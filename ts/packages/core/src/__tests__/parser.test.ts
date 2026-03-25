import { describe, it, expect } from "vitest";
import { parseGraphYaml, buildGraph, resolveEdges } from "../parser.js";
import type { EdgeDefinition } from "../types.js";

describe("parseGraphYaml", () => {
  it("parses a minimal valid graph", () => {
    const yaml = `
name: test
nodes:
  start:
    type: run
    config:
      command: "echo hello"
  end:
    type: run
    config:
      command: "echo done"
edges:
  - chain: [start, end]
`;
    const def = parseGraphYaml(yaml);
    expect(def.name).toBe("test");
    expect(Object.keys(def.nodes)).toEqual(["start", "end"]);
  });

  it("parses a graph with all node types", () => {
    const yaml = `
name: full
config:
  max_rework: 3
nodes:
  build:
    type: agent
    config:
      prompt: build.md
      tools: [Read, Write]
  gate:
    type: run
    config:
      command: "npm test"
  route:
    type: decision
    config:
      input: steps.build.output.verdict
      cases:
        approve: done
        rework: build
  spread:
    type: fan
    config:
      mode: parallel
      over: steps.build.output.items
      body: build
  done:
    type: run
    config:
      command: "echo done"
edges:
  - chain: [build, gate, route]
  - from: route
    to: done
    on: approve
  - from: route
    to: build
    on: rework
    max: 3
    exhaust: escalate
`;
    const def = parseGraphYaml(yaml);
    expect(def.name).toBe("full");
    expect(def.config?.max_rework).toBe(3);
    expect(Object.keys(def.nodes)).toHaveLength(5);
    expect(def.edges).toHaveLength(3);
  });

  it("rejects invalid YAML", () => {
    expect(() => parseGraphYaml("name: ''")).toThrow();
  });
});

describe("resolveEdges", () => {
  it("expands chain edges", () => {
    const defs: EdgeDefinition[] = [{ chain: ["a", "b", "c", "d"] }];
    const edges = resolveEdges(defs);
    expect(edges).toEqual([
      { from: "a", to: "b" },
      { from: "b", to: "c" },
      { from: "c", to: "d" },
    ]);
  });

  it("preserves conditional edges", () => {
    const defs: EdgeDefinition[] = [
      { from: "route", to: "done", on: "approve", max: 3, exhaust: "escalate" },
    ];
    const edges = resolveEdges(defs);
    expect(edges).toEqual([
      { from: "route", to: "done", condition: "approve", max: 3, exhaust: "escalate" },
    ]);
  });

  it("handles mixed edge types", () => {
    const defs: EdgeDefinition[] = [
      { chain: ["a", "b", "c"] },
      { from: "c", to: "a", on: "retry", max: 2 },
    ];
    const edges = resolveEdges(defs);
    expect(edges).toHaveLength(3);
  });
});

describe("buildGraph", () => {
  it("builds an OrchestrationGraph from a definition", () => {
    const yaml = `
name: test
nodes:
  start:
    type: run
    config:
      command: "echo hello"
  end:
    type: run
    config:
      command: "echo done"
edges:
  - chain: [start, end]
`;
    const def = parseGraphYaml(yaml);
    const graph = buildGraph(def, { extra: "value" });

    expect(graph.name).toBe("test");
    expect(graph.nodes.size).toBe(2);
    expect(graph.edges).toHaveLength(1);
    expect(graph.context.config.extra).toBe("value");
  });

  it("throws on invalid edge references", () => {
    const yaml = `
name: test
nodes:
  start:
    type: run
    config:
      command: "echo hello"
edges:
  - from: start
    to: nonexistent
`;
    const def = parseGraphYaml(yaml);
    expect(() => buildGraph(def)).toThrow("unknown node: 'nonexistent'");
  });
});
