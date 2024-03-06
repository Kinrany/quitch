import { assertEquals } from "https://deno.land/std@0.218.2/assert/mod.ts";
import * as Plan from "./plan.ts";
import { ok } from "./result.ts";

Deno.test("parse example plan", () => {
  const result = Plan.parse(Plan.example_string);
  assertEquals(result, ok(Plan.example));
});

Deno.test("print and parse example plan", () => {
  const plan_string = Plan.format(Plan.example);
  const result = Plan.parse(plan_string);
  assertEquals(result, ok(Plan.example));
});
