import {
  assert,
  assertEquals,
  assertMatch,
} from "https://deno.land/std@0.218.2/assert/mod.ts";
import * as Change from "./change.ts";
import { ok } from "./result.ts";

Deno.test("Change.parse", () => {
  const result = Change.parse(Change.example_string);
  assertEquals(result, ok(Change.example));
});

Deno.test("Change.format and parse back", () => {
  const change_text = Change.format(Change.example);
  const result = Change.parse(change_text);
  assertEquals(result, ok(Change.example));
});

Deno.test("Change.parse converts escaped newlines to newlines", () => {
  const note = "a\\nb";
  const change = Change.parse(`name 2000-01-01T00:00:00Z author # ${note}`);
  assert(change.ok);
  assertEquals(change.value.note, "a\nb");
});

Deno.test("Change.format escapes newlines", () => {
  const note = "a\nb";
  const change_text = Change.format({
    ...Change.example,
    note,
  });
  assertMatch(change_text, /a\\nb/);
});
