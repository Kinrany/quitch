import * as Change from "./change.ts";
import * as Plan from "./plan.ts";
import { err, ok, Result, unwrap } from "./result.ts";

async function read_plan(plan_file_path: string): Promise<Plan.Plan> {
  const plan_string = await Deno.readTextFile(plan_file_path);
  return unwrap(Plan.parse(plan_string));
}

export function format_change(
  project: string,
  change: Change.Change,
  parent?: string,
): string {
  let str = "";
  str += `project ${project}\n`;
  str += `change ${change.name}\n`;
  if (parent) {
    str += `parent ${parent}\n`;
  }
  str += `planner ${change.planner}\n`;
  str += `date ${Change.format_date(change.date)}\n`;
  str += "\n";
  str += `${change.note}`;
  return str;
}

export async function change_digest(
  project: string,
  change: Change.Change,
  parent?: string,
): Promise<string> {
  const encode = (s: string) => new TextEncoder().encode(s);

  const change_str = format_change(project, change, parent);
  const bytes = encode(`change ${encode(change_str).length}\0${change_str}`);
  const hash_buffer = await crypto.subtle.digest("SHA-1", bytes);
  const hash_array = Array.from(new Uint8Array(hash_buffer));
  const hash_hex = hash_array
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hash_hex;
}

async function format_plan_change(
  plan: Plan.Plan,
  change_name: string,
): Promise<Result<string>> {
  const change_idx = plan.changes.findIndex((c) => c.name === change_name);
  if (change_idx === -1) {
    return err(new Error(`Unknown change "${change_name}"`));
  }

  const parent_digest = await plan.changes.slice(0, change_idx).reduce(
    async (parent_promise, change) => {
      const parent = await parent_promise;
      return await change_digest(plan.project, change, parent);
    },
    Promise.resolve(undefined as string | undefined),
  );

  return ok(format_change(
    plan.project,
    plan.changes[change_idx],
    parent_digest,
  ));
}

if (import.meta.main) {
  if (Deno.args[0] === "show") {
    if (Deno.args[1] === "change") {
      const change_name = Deno.args[2];
      if (!change_name) {
        console.error("Usage: quitch show change <change-name>");
        Deno.exit(1);
      }
      const plan_file_path = Deno.args[3] || "./sqitch.plan";
      const plan = await read_plan(plan_file_path);
      const result = await format_plan_change(plan, change_name);
      if (!result.ok) {
        console.error(result.error.message);
        Deno.exit(1);
      } else {
        console.log(result.value);
        Deno.exit(0);
      }
    } else if (Deno.args[1]) {
      console.error(`Unknown show type: ${Deno.args[1]}`);
      console.error(`Available show types: change`);
      Deno.exit(1);
    } else {
      console.error("Usage: quitch show <type> <object>");
      Deno.exit(1);
    }
  } else if (Deno.args[0]) {
    console.error(`Unknown command: ${Deno.args[0]}`);
    console.error(`Available commands: show`);
    Deno.exit(1);
  } else {
    console.error("Usage: quitch <command>");
    Deno.exit(1);
  }
}
