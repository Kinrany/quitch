import { parseArgs } from "https://deno.land/std@0.218.2/cli/mod.ts";
import * as Path from "https://deno.land/std@0.218.2/path/mod.ts";
import {
  Client,
  ClientConfig,
  Connection,
} from "https://deno.land/x/mysql@v2.12.1/mod.ts";
import * as Change from "./change.ts";
import * as Plan from "./plan.ts";
import { ChangeRow } from "./registry.ts";
import { err, ok, Result, unwrap } from "./result.ts";
import schema from "./schema.ts";

async function load_plan(plan_file_path: string): Promise<Plan.Plan> {
  console.log(`Using plan file ${plan_file_path}`);
  const plan_string = await Deno.readTextFile(plan_file_path);
  const parsing_result = Plan.parse(plan_string);
  if (!parsing_result.ok) {
    throw parsing_result.error;
  }
  const plan = parsing_result.value;
  if (plan.changes.length === 0) {
    console.warn("Warning: the plan is empty");
  }
  return plan;
}

async function format_plan_change(
  plan: Plan.Plan,
  change_name: string,
): Promise<Result<string>> {
  for await (const change of Plan.full_changes(plan)) {
    if (change.name === change_name) {
      return ok(Change.format(plan.project, change, change.parent));
    }
  }

  return err(new Error(`Unknown change "${change_name}"`));
}

export function parse_connection_string(s: string): Result<ClientConfig> {
  let url: URL;
  try {
    url = new URL(s);
  } catch (e) {
    return err(new Error(`Invalid target: ${e}`));
  }
  if (url.protocol !== "mysql:") {
    return err(new Error(`Unsupported target protocol ${url.protocol}`));
  }

  let port: number;
  if (url.port) {
    try {
      port = parseInt(url.port, 10);
    } catch (_e) {
      return err(
        new Error(
          `Value "${url.port}" invalid for option db-port (number expected)`,
        ),
      );
    }
  } else {
    port = 3306;
  }

  return ok({
    hostname: url.hostname,
    port,
    username: url.username,
    password: url.password,
    db: url.pathname.slice(1),
  });
}

export function format_connection_string(opts: ClientConfig): string {
  return `mysql://${opts.username}:${opts.password}@${opts.hostname}:${opts.port}/${opts.db}`;
}

export type CommonArgs = {
  registry: string;
  plan_file: string;
  connection_options: ClientConfig;
};

export function parse_common_args(args: string[]): Result<CommonArgs> {
  const flags = parseArgs(args);

  const registry = flags["registry"] as string || "sqitch";

  const plan_file = flags["plan-file"] as string || "./sqitch.plan";

  const target = flags["target"];
  if (typeof target !== "string" || target === "") {
    return err(new Error("Missing required argument: --target <target>"));
  }
  const connection_options_result = parse_connection_string(target);
  let connection_options: ClientConfig;
  if (connection_options_result.ok) {
    connection_options = connection_options_result.value;
  } else {
    return connection_options_result;
  }

  return ok({
    registry,
    plan_file,
    connection_options,
  });
}

/**
 * Validate the state of the registry against the plan.
 * @param registry_client client for connecting to the change registry
 * @param plan plan to validate against
 * @returns the first undeployed change with change ID
 */
async function validate_against_plan(
  registry_client: Client,
  plan: Plan.Plan,
): Promise<Plan.FullChange | undefined> {
  let first_undeployed_change: Plan.FullChange | undefined;

  const change_rows: ChangeRow[] = await registry_client
    .query("select * from `changes`");
  const change_map = new Map(change_rows.map((c) => [c.change_id, c]));
  for await (const change of Plan.full_changes(plan)) {
    if (change_map.has(change.id)) {
      change_map.delete(change.id);
      continue;
    } else {
      first_undeployed_change = change;
      break;
    }
  }

  if (change_map.size > 0) {
    let error_string = "Found unknown changes";
    // TODO: make sure the order isn't random
    for (const [change_id, change] of change_map) {
      error_string += `${change_id} ${change.change}\n`;
    }
    throw new Error(error_string);
  }

  return first_undeployed_change;
}

async function connect_db(config: ClientConfig): Promise<Client> {
  const target = format_connection_string(config);
  console.debug(`Connecting to ${target}`);
  const db_client = await new Client().connect(config);
  await db_client.execute("SELECT 1");
  console.debug(`Connected to ${config.db}`);
  return db_client;
}

/**
 * Create a database schema with the given name if it does not exist
 * @param db_client the database client used to look up and create the schema
 * @param schema_name the name of the schema to create
 * @returns `true` if the schema had to be created
 */
async function create_schema_if_not_exists(
  db_client: Client,
  schema_name: string,
): Promise<boolean> {
  const rows = await db_client.query(
    "select schema_name from information_schema.schemata where schema_name = ?",
    [schema_name],
  );
  const registry_exists = rows.length > 0;
  if (!registry_exists) {
    console.log(`Creating schema ${schema_name}`);
    // Cannot use prepared statements because DDL
    await db_client.execute(`create schema ${schema_name}`);
  }
  return !registry_exists;
}

/**
 * The driver does not support multiple statements in a single query,
 * so we split the schema by the statement terminator and execute each
 * statement in sequence.
 * @param sql the SQL to execute
 * @returns a function that executes the SQL on a connection
 */
const execute_multiple_statements =
  (sql: string) => async (conn: Connection): Promise<void> => {
    const statements = sql
      .split(";")
      .map((s) => s.trim())
      .filter((s) => s.length !== 0);

    for (const statement of statements) {
      await conn.execute(statement);
    }
  };

/**
 * Connect to the main database and the registry
 */
async function connect(
  args: ClientConfig & { registry_name: string },
): Promise<{ db: Client; registry: Client }> {
  const db_client = await connect_db(args);

  // Create a schema for the registry if it doesn't exist
  const must_apply_registry_schema = await create_schema_if_not_exists(
    db_client,
    args.registry_name,
  );

  // Create the registry connection
  const registry_client = await connect_db({ ...args, db: args.registry_name });

  // Apply the schema if the registry is newly created
  if (must_apply_registry_schema) {
    console.log("Applying registry schema");
    await registry_client.useConnection(execute_multiple_statements(schema));
  }

  return { db: db_client, registry: registry_client };
}

async function log_registry_event(
  event: "deploy" | "revert" | "fail",
  registry: Client,
  change: Plan.FullChange,
  project_name: string,
) {
  await registry.execute(
    "insert into `events` (`event`, `change_id`, `change`, `project`, `note`, `requires`, `conflicts`, `tags`, `committed_at`, `committer_name`, `committer_email`, `planned_at`, `planner_name`, `planner_email`) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    [
      event,
      change.id,
      change.name,
      project_name,
      change.note,
      "",
      "",
      "",
      new Date(),
      "quitch",
      "quitch@quitch",
      change.date,
      // TODO: split the name from the email
      change.planner,
      change.planner,
    ],
  );
}

if (import.meta.main) {
  if (Deno.args[0] === "show") {
    if (Deno.args[1] === "change") {
      const args = parseArgs(Deno.args.slice(2));
      const change_name = args._.at(0)?.toString();
      if (!change_name) {
        console.error("Usage: quitch show change <change-name>");
        Deno.exit(1);
      }
      const plan_file_path = args["plan-file"] as string || "./sqitch.plan";
      const plan = await load_plan(plan_file_path);
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
  } else if (Deno.args[0] === "deploy") {
    /*
    // Parse arguments
    const args = parseArgs(Deno.args.slice(1));
    const common_args_result = parse_common_args(Deno.args.slice(1));
    let common_args: CommonArgs;
    if (common_args_result.ok) {
      common_args = common_args_result.value;
    } else {
      console.error(common_args_result.error);
      Deno.exit(1);
    }
    const deploy_to = args._.at(0)?.toString();

    const plan = await load_plan(common_args.plan_file);

    // Find the change that needs to be deployed
    let change_to_deploy_idx: number;
    if (deploy_to === undefined) {
      change_to_deploy_idx = plan.changes.length - 1;
    } else {
      const selected_change_idx = plan
        .changes
        .findIndex((c) => c.name === deploy_to);
      if (selected_change_idx !== -1) {
        change_to_deploy_idx = selected_change_idx;
      } else {
        console.error(`Change not in the plan: ${deploy_to}`);
        Deno.exit(1);
      }
    }
    const change_to_deploy = plan.changes[change_to_deploy_idx];

    // Connect to the main database and the registry
    let { db, registry } = await connect({
      registry_name: common_args.registry,
      ...common_args.connection_options,
    });

    // Retrieve applied changes and verify them against the plan
    const first_undeployed_change = await validate_against_plan(registry, plan);
    */

    console.error("Not implemented");
    Deno.exit(1);
  } else if (Deno.args[0] == "revert") {
    console.log("Reverting only the last change by default");

    // Initial setup
    const common_args = unwrap(parse_common_args(Deno.args.slice(1)));
    const plan = await load_plan(common_args.plan_file);
    const { db, registry } = await connect({
      registry_name: common_args.registry,
      ...common_args.connection_options,
    });

    // Make sure the registry is in a valid state
    const first_undeployed_change = await validate_against_plan(registry, plan);

    // Find the last deployed change
    const last_deployed_change_id = first_undeployed_change
      ? first_undeployed_change.parent
      : await (async () => {
        let last_deployed_change: Plan.FullChange | undefined;
        for await (const change of Plan.full_changes(plan)) {
          last_deployed_change = change;
        }
        // TODO: return full change so it doesn't need to be looked up again
        return last_deployed_change?.id;
      })();
    if (!last_deployed_change_id) {
      console.error("Nothing to revert");
      Deno.exit(1);
    }
    let last_deployed_change: Plan.FullChange | undefined;
    for await (const change of Plan.full_changes(plan)) {
      if (change.id === last_deployed_change_id) {
        last_deployed_change = change;
        break;
      }
    }
    if (!last_deployed_change) {
      if (first_undeployed_change) {
        const change_text = Change.format(
          plan.project,
          first_undeployed_change,
          first_undeployed_change.parent,
        );
        throw new Error(`Panic! Invalid parent ID on change:\n${change_text}`);
      } else {
        throw new Error("Panic! Last deployed change not found");
      }
    }

    // Get the script corresponding to reverting the last deployed change
    console.log(`Reverting ${last_deployed_change.name}`);
    const plan_dir = Path.dirname(common_args.plan_file);
    const revert_path = Path.join(
      plan_dir,
      "revert",
      `${last_deployed_change.name}.sql`,
    );
    const revert_sql = await Deno.readTextFile(revert_path);

    // Revert the change
    try {
      await db.useConnection(execute_multiple_statements(revert_sql));
      await registry.execute("delete from `changes` where change_id = ?", [
        last_deployed_change.id,
      ]);
      await log_registry_event(
        "revert",
        registry,
        last_deployed_change,
        plan.project,
      );
    } catch (e) {
      console.error("Failed to revert:", e);
      await log_registry_event(
        "fail",
        registry,
        last_deployed_change,
        plan.project,
      );
      Deno.exit(1);
    }
  } else if (Deno.args[0]) {
    console.error(`Unknown command: ${Deno.args[0]}`);
    console.error(`Available commands: revert, show`);
    Deno.exit(1);
  } else {
    console.error("Usage: quitch <command>");
    Deno.exit(1);
  }
}
