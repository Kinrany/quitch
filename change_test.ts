import {
  assert,
  assertEquals,
  assertMatch,
} from "https://deno.land/std@0.218.2/assert/mod.ts";
import * as Change from "./change.ts";
import { ok } from "./result.ts";

Deno.test("Change.format", () => {
  const formatted_change = Change.format("quitch", Change.example);
  assertEquals(
    formatted_change,
    Change.example_string,
  );
});

Deno.test("Change.id - no parent", async () => {
  assertEquals(
    await Change.id("quitch", Change.example, undefined),
    "da41a550b0cba5bd3dffbf645032a98ae1136da5",
  );
});

Deno.test("Change.id - with parent", async () => {
  assertEquals(
    await Change.id(
      "quitch",
      Change.example,
      "da41a550b0cba5bd3dffbf645032a98ae1136da5",
    ),
    "7b6b9ba12694a34a5445e1d847d36d2344d61bcb",
  );
});

Deno.test("Change.id - unicode", async () => {
  const change = {
    ...Change.example,
    note: "🤦🏼‍♂️",
  };
  assertEquals(
    await Change.id("quitch", change, undefined),
    "fb29c4f840ce9cd266d983a2c90d7ddf745c1711",
  );
});

Deno.test("Change.parse_line", () => {
  const result = Change.parse_line(Change.example_line);
  assertEquals(result, ok(Change.example));
});

Deno.test("Change.format_line + Change.parse_line", () => {
  const change_text = Change.format_line(Change.example);
  const result = Change.parse_line(change_text);
  assertEquals(result, ok(Change.example));
});

Deno.test("Change.parse_line - converts escaped newlines to newlines", () => {
  const note = "a\\nb";
  const change = Change.parse_line(
    `name 2000-01-01T00:00:00Z author # ${note}`,
  );
  assert(change.ok);
  assertEquals(change.value.note, "a\nb");
});

Deno.test("Change.format_line - escapes newlines", () => {
  const note = "a\nb";
  const change_text = Change.format_line({
    ...Change.example,
    note,
  });
  assertMatch(change_text, /a\\nb/);
});
