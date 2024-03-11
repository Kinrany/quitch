import { assertEquals } from "https://deno.land/std@0.218.2/assert/mod.ts";
import { format_connection_string, parse_connection_string } from "./main.ts";
import { ok } from "./result.ts";
import { parse_common_args } from "./main.ts";

Deno.test("parse connection string", () => {
  assertEquals(
    parse_connection_string("mysql://user:pass@localhost:3306/dbname"),
    ok({
      username: "user",
      password: "pass",
      hostname: "localhost",
      port: 3306,
      db: "dbname",
    }),
  );
});

Deno.test("format connection string", () => {
  assertEquals(
    format_connection_string({
      username: "user",
      password: "pass",
      hostname: "localhost",
      port: 3306,
      db: "dbname",
    }),
    "mysql://user:pass@localhost:3306/dbname",
  );
});

Deno.test("parse common args", () => {
  const args = [
    "--target",
    "mysql://user:pass@localhost:3306/dbname",
    "--registry",
    "quitch",
    "--plan-file",
    "./quitch.plan",
  ];
  assertEquals(
    parse_common_args(args),
    ok({
      connection_options: {
        username: "user",
        password: "pass",
        hostname: "localhost",
        port: 3306,
        db: "dbname",
      },
      registry: "quitch",
      plan_file: "./quitch.plan",
    }),
  );
});
