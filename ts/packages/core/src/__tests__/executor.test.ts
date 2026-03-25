import { describe, it, expect } from "vitest";
import { execute, findEntryNode } from "../executor.js";
import { parseGraphYaml, buildGraph } from "../parser.js";
import type {
  ExecutorRegistry,
  GraphNode,
  ExecutionContext,
  NodeResult,
} from "../types.js";

/** Stub executor that records calls and returns configurable results. */
function makeStubExecutors(
  results: Record<string, Partial<NodeResult>> = {}
): ExecutorRegistry {
  const makeExecutor = () => ({
    execute: async (node: GraphNode, _ctx: ExecutionContext): Promise<NodeResult> => {
      const override = results[node.id] ?? {};
      return {
        status: "success",
        output: override.output ?? `${node.id}-output`,
        duration_ms: 1,
        ...override,
      };
    },
  });
  return {
    agent: makeExecutor(),
    run: makeExecutor(),
    decision: makeExecutor(),
    fan: makeExecutor(),
  };
}

describe("findEntryNode", () => {
  it("finds the node with no incoming edges", () => {
    const yaml = `
name: test
nodes:
  a:
    type: run
    config:
      command: "echo a"
  b:
    type: run
    config:
      command: "echo b"
  c:
    type: run
    config:
      command: "echo c"
edges:
  - chain: [a, b, c]
`;
    const graph = buildGraph(parseGraphYaml(yaml));
    const entry = findEntryNode(graph);
    expect(entry?.id).toBe("a");
  });
});

describe("execute", () => {
  it("runs a linear graph to completion", async () => {
    const yaml = `
name: linear
nodes:
  a:
    type: run
    config:
      command: "echo a"
  b:
    type: run
    config:
      command: "echo b"
  c:
    type: run
    config:
      command: "echo c"
edges:
  - chain: [a, b, c]
`;
    const graph = buildGraph(parseGraphYaml(yaml));
    const result = await execute(graph, makeStubExecutors());

    expect(result.status).toBe("completed");
    expect(result.context.results.size).toBe(3);
    expect(result.context.results.get("a")?.status).toBe("success");
    expect(result.context.results.get("b")?.status).toBe("success");
    expect(result.context.results.get("c")?.status).toBe("success");
  });

  it("follows decision edges based on verdict", async () => {
    const yaml = `
name: branching
nodes:
  inspect:
    type: run
    config:
      command: "echo inspect"
  route:
    type: decision
    config:
      input: steps.inspect.output
      cases:
        approve: done
        rework: retry
  done:
    type: run
    config:
      command: "echo done"
  retry:
    type: run
    config:
      command: "echo retry"
edges:
  - chain: [inspect, route]
  - from: route
    to: done
    on: approve
  - from: route
    to: retry
    on: rework
`;
    const graph = buildGraph(parseGraphYaml(yaml));

    // Decision executor returns "approve" verdict
    const executors = makeStubExecutors({
      route: {
        output: { verdict: "approve", matched: "approve", target: "done" },
      },
    });

    const result = await execute(graph, executors);
    expect(result.status).toBe("completed");
    expect(result.context.results.has("done")).toBe(true);
    expect(result.context.results.has("retry")).toBe(false);
  });

  it("respects loop max and exhausts", async () => {
    const yaml = `
name: loop
nodes:
  work:
    type: run
    config:
      command: "echo work"
  check:
    type: decision
    config:
      input: steps.work.output
      cases:
        rework: work
edges:
  - chain: [work, check]
  - from: check
    to: work
    on: rework
    max: 2
    exhaust: escalate
`;
    const graph = buildGraph(parseGraphYaml(yaml));

    // Decision always says rework
    const executors = makeStubExecutors({
      check: {
        output: { verdict: "rework", matched: "rework", target: "work" },
      },
    });

    const result = await execute(graph, executors);
    expect(result.status).toBe("exhausted");
  });

  it("handles on_fail with rework routing", async () => {
    const yaml = `
name: onfail
nodes:
  build:
    type: run
    config:
      command: "echo build"
  gate:
    type: run
    config:
      command: "npm test"
    middleware:
      on_fail: rework(build)
  done:
    type: run
    config:
      command: "echo done"
edges:
  - chain: [build, gate, done]
`;
    const graph = buildGraph(parseGraphYaml(yaml));

    let gateCallCount = 0;
    const executors = makeStubExecutors();
    executors.run = {
      execute: async (node: GraphNode) => {
        if (node.id === "gate") {
          gateCallCount++;
          // Fail first time, succeed second
          if (gateCallCount === 1) {
            return { status: "failure", output: "tests failed", duration_ms: 1 };
          }
        }
        return { status: "success", output: `${node.id}-ok`, duration_ms: 1 };
      },
    };

    const result = await execute(graph, executors);
    expect(result.status).toBe("completed");
    expect(gateCallCount).toBe(2);
  });

  it("skips nodes with false condition", async () => {
    const yaml = `
name: conditional
nodes:
  a:
    type: run
    config:
      command: "echo a"
  b:
    type: run
    config:
      command: "echo b"
    middleware:
      condition: steps.a.output.needs_ai
  c:
    type: run
    config:
      command: "echo c"
edges:
  - chain: [a, b, c]
`;
    const graph = buildGraph(parseGraphYaml(yaml));

    const executors = makeStubExecutors({
      a: { output: { needs_ai: false } },
    });

    const result = await execute(graph, executors);
    expect(result.status).toBe("completed");
    expect(result.context.results.get("b")?.status).toBe("skipped");
    expect(result.context.results.get("c")?.status).toBe("success");
  });
});
