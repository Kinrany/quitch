import {
  assert,
  assertEquals,
} from "https://deno.land/std@0.218.2/assert/mod.ts";
import * as Plan from "./plan.ts";
import * as Change from "./change.ts";
import { ok } from "./result.ts";

Deno.test("Plan.parse", () => {
  const result = Plan.parse(Plan.example_string);
  assertEquals(result, ok(Plan.example));
});

Deno.test("Plan.format + Plan.parse", () => {
  const plan_string = Plan.format(Plan.example);
  const result = Plan.parse(plan_string);
  assertEquals(result, ok(Plan.example));
});

Deno.test("Plan.full_changes", async () => {
  const results = [];
  for await (const result of Plan.full_changes(Plan.example)) {
    results.push(result);
  }
  assertEquals(results, [
    {
      ...Change.example,
      parent: undefined,
      id: "da41a550b0cba5bd3dffbf645032a98ae1136da5",
    },
    {
      date: new Date("2024-03-10T00:04:24.000Z"),
      name: "change_num2",
      note: "Second change",
      planner: "Ruslan Fadeev <github@kinrany.dev>",
      parent: "da41a550b0cba5bd3dffbf645032a98ae1136da5",
      id: "2959791f9fb4db4c322a9fdf121215d5e8a6a601",
    },
  ]);
});

Deno.test("Plan.full_changes - empty", async () => {
  const plan: Plan.Plan = { project: "quitch", changes: [] };
  const result = await Plan.full_changes(plan).next();
  assert(result.done);
  assertEquals(result.value, undefined);
});
