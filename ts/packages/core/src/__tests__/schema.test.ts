import { describe, it, expect } from "vitest";
import { GraphDefinitionSchema } from "../schema.js";

describe("GraphDefinitionSchema", () => {
  it("validates a minimal graph", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        start: {
          type: "run",
          config: { command: "echo hi" },
        },
      },
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("validates an agent node", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        build: {
          type: "agent",
          config: {
            prompt: "build.md",
            tools: ["Read", "Write"],
            max_turns: 30,
            output_schema: "schemas/build.json",
          },
        },
      },
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("validates a decision node", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        route: {
          type: "decision",
          config: {
            input: "steps.build.output.verdict",
            cases: { approve: "done", rework: "build" },
          },
        },
      },
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("validates a fan node", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        spread: {
          type: "fan",
          config: {
            mode: "parallel",
            over: "steps.schedule.output.waves",
            body: "solve-leaf",
          },
        },
      },
      edges: [],
    });
    expect(result.success).toBe(true);
  });

  it("rejects run node without command or check", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        bad: {
          type: "run",
          config: {},
        },
      },
      edges: [],
    });
    expect(result.success).toBe(false);
  });

  it("validates chain edges", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        a: { type: "run", config: { command: "echo a" } },
        b: { type: "run", config: { command: "echo b" } },
      },
      edges: [{ chain: ["a", "b"] }],
    });
    expect(result.success).toBe(true);
  });

  it("validates conditional edges", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "test",
      nodes: {
        a: { type: "run", config: { command: "echo a" } },
        b: { type: "run", config: { command: "echo b" } },
      },
      edges: [{ from: "a", to: "b", on: "approve", max: 3, exhaust: "escalate" }],
    });
    expect(result.success).toBe(true);
  });

  it("rejects empty name", () => {
    const result = GraphDefinitionSchema.safeParse({
      name: "",
      nodes: {},
      edges: [],
    });
    expect(result.success).toBe(false);
  });
});
