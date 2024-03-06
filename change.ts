import { err, ok, Result } from "./result.ts";

export type Change = {
  name: string;
  note: string;
  date: Date;
  planner: string;
};

export function parse(change: string): Result<Change> {
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

export function format(change: Change): string {
  const date = format_date(change.date);
  const note = change.note.replaceAll("\n", "\\n");
  return `${change.name} ${date} ${change.planner} # ${note}`;
}

export function format_date(date: Date): string {
  return date.toISOString().slice(0, 19) + "Z";
}

export const example: Change = {
  name: "change_name",
  note: "A description of the change",
  date: new Date("2024-03-07T03:19:34Z"),
  planner: "Ruslan Fadeev <github@kinrany.dev>",
};

export const example_string =
  "change_name 2024-03-07T03:19:34Z Ruslan Fadeev <github@kinrany.dev> # A description of the change";
