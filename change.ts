import { err, ok, Result } from "./result.ts";

export type Change = {
  name: string;
  note: string;
  date: Date;
  planner: string;
};

export function format(
  project: string,
  change: Change,
  parent?: string,
): string {
  let str = "";
  str += `project ${project}\n`;
  str += `change ${change.name}\n`;
  if (parent) {
    str += `parent ${parent}\n`;
  }
  str += `planner ${change.planner}\n`;
  str += `date ${format_line_date(change.date)}\n`;
  str += "\n";
  str += `${change.note}`;
  return str;
}

export async function id(
  project: string,
  change: Change,
  parent?: string,
): Promise<string> {
  const encode = (s: string) => new TextEncoder().encode(s);

  const change_str = format(project, change, parent);
  const bytes = encode(`change ${encode(change_str).length}\0${change_str}`);
  const hash_buffer = await crypto.subtle.digest("SHA-1", bytes);
  const hash_array = Array.from(new Uint8Array(hash_buffer));
  const hash_hex = hash_array
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hash_hex;
}

export function parse_line(change: string): Result<Change> {
  const error = (m: string) => err(new Error(`Invalid change format: ${m}`));

  const name_end_idx = change.indexOf(" ");
  if (name_end_idx === -1) return error("missing space after name");
  const name = change.slice(0, name_end_idx);
  change = change.slice(name_end_idx).trimStart();

  const date_end_idx = change.indexOf(" ");
  if (date_end_idx === -1) return error("missing space after date");
  let date;
  try {
    date = new Date(change.slice(0, date_end_idx));
  } catch (_e) {
    return error("invalid date format, expected ISO 8601 date string");
  }
  change = change.slice(date_end_idx).trimStart();

  const author_end_idx = change.indexOf("#");
  let planner;
  let note;
  if (author_end_idx === -1) {
    planner = change;
    note = "";
  } else {
    planner = change.slice(0, author_end_idx).trim();
    note = change.slice(author_end_idx + 1).trim().replaceAll("\\n", "\n");
  }

  return ok({
    name,
    note,
    date,
    planner,
  });
}

export function format_line(change: Change): string {
  const date = format_line_date(change.date);
  const note = change.note.replaceAll("\n", "\\n");
  return `${change.name} ${date} ${change.planner} # ${note}`;
}

export function format_line_date(date: Date): string {
  return date.toISOString().slice(0, 19) + "Z";
}

export const example: Change = {
  date: new Date("2024-03-07T03:19:34Z"),
  name: "change_name",
  note: "A description of the change",
  planner: "Ruslan Fadeev <github@kinrany.dev>",
};

export const example_string = `project quitch
change change_name
planner Ruslan Fadeev <github@kinrany.dev>
date 2024-03-07T03:19:34Z

A description of the change`;

export const example_line =
  "change_name 2024-03-07T03:19:34Z Ruslan Fadeev <github@kinrany.dev> # A description of the change";
