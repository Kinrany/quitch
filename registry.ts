export type ChangeRow = {
  change_id: string;
  script_hash?: string;
  /** Name of the change */
  change: string;
  project: string;
  note: string;
  committed_at: Date;
  committer_name: string;
  committer_email: string;
  planned_at: Date;
  planner_name: string;
  planner_email: string;
};
