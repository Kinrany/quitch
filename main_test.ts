import { assertEquals } from "https://deno.land/std@0.218.2/assert/mod.ts";
import * as Change from "./change.ts";
import {
  change_digest,
  format_change,
  parse_connection_string,
} from "./main.ts";
import { ok } from "./result.ts";

Deno.test("format change for a digest", () => {
  const formatted_change = format_change("quitch", Change.example);
  assertEquals(
    formatted_change,
    `project quitch
change change_name
planner Ruslan Fadeev <github@kinrany.dev>
date 2024-03-07T03:19:34Z

A description of the change`,
  );
});

Deno.test("digest for change with no parent", async () => {
  assertEquals(
    await change_digest("quitch", Change.example, undefined),
    "da41a550b0cba5bd3dffbf645032a98ae1136da5",
  );
});

Deno.test("digest for change with parent", async () => {
  assertEquals(
    await change_digest(
      "quitch",
      Change.example,
      "da41a550b0cba5bd3dffbf645032a98ae1136da5",
    ),
    "7b6b9ba12694a34a5445e1d847d36d2344d61bcb",
  );
});

Deno.test("digest for change with unicode characters", async () => {
  const change = {
    ...Change.example,
    note: "🤦🏼‍♂️",
  };
  assertEquals(
    await change_digest("quitch", change, undefined),
    "fb29c4f840ce9cd266d983a2c90d7ddf745c1711",
  );
});

Deno.test("parse connection string", () => {
  assertEquals(
    parse_connection_string("mysql://user:password@localhost:3306/dbname"),
    ok({
      hostname: "localhost",
      port: 3306,
      username: "user",
      password: "password",
      db: "dbname",
    }),
  );
});
