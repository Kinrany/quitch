import * as Change from "./change.ts";
import { err, ok, Result } from "./result.ts";

export type Plan = {
  project: string;
  changes: Change.Change[];
};

export function parse(plan_text: string): Result<Plan> {
  const lines = plan_text.split("\n");

  // First line must be a recognized syntax format
  if (lines[0] !== "%syntax-version=1.0.0") {
    return err(new Error("Unsupported sqitch plan syntax"));
  }

  // There are three types of lines:
  // - Meta lines that start with %
  // - Change lines
  // - Empty lines

  // Parse meta lines
  const meta_entries: [string, string][] = lines
    .filter((line) => line.startsWith("%"))
    .map((line) => {
      line = line.slice(1).trim();
      const idx = line.indexOf("=");
      if (idx === -1) {
        return [line, ""];
      } else {
        return [line.slice(0, idx), line.slice(idx + 1)];
      }
    });
  new Map();
  const metadata = new Map(meta_entries);
  const project = metadata.get("project") || "";

  // Change lines are lines that aren't meta lines or empty
  const change_lines = lines
    .filter((line) => !line.startsWith("%"))
    .filter((line) => line.trim() !== "");

  // Parse each change line or exit early if any fails
  const changes = [];
  for (const line of change_lines) {
    const result = Change.parse_line(line);
    if (!result.ok) {
      console.debug(`Failed to parse line: ${line}`);
      return result;
    }
    changes.push(result.value);
  }

  return ok({
    project,
    changes,
  });
}

export function format(plan: Plan): string {
  const meta_lines = [
    "%syntax-version=1.0.0",
    `%project=${plan.project}`,
  ];
  const change_lines = plan.changes.map((change) => Change.format_line(change));
  return [...meta_lines, "", ...change_lines].join("\n") + "\n";
}

export const example: Plan = {
  project: "quitch",
  changes: [
    Change.example,
    {
      date: new Date("2024-03-10T00:04:24Z"),
      name: "change_num2",
      note: "Second change",
      planner: "Ruslan Fadeev <github@kinrany.dev>",
    },
  ],
};

export const example_string = `%syntax-version=1.0.0
%project=quitch

change_name 2024-03-07T03:19:34Z Ruslan Fadeev <github@kinrany.dev> # A description of the change
change_num2 2024-03-10T00:04:24Z Ruslan Fadeev <github@kinrany.dev> # Second change
`;

export type FullChange = Change.Change & {
  id: string;
  parent?: string;
};

/**
 * Generate a list of changes with their ids and parent ids
 * @param plan The plan to generate changes for
 */
export async function* full_changes(
  plan: Plan,
): AsyncGenerator<FullChange, void> {
  let parent_id: string | undefined;
  for (const change of plan.changes) {
    const change_id = await Change.id(plan.project, change, parent_id);
    yield { ...change, id: change_id, parent: parent_id };
    parent_id = change_id;
  }
}
