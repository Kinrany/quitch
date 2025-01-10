use chrono::{DateTime, Utc};

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(dead_code)]
pub struct ChangeRow {
    pub change_id: String,
    pub script_hash: Option<String>,
    /// Name of the change
    pub change: String,
    pub project: String,
    pub note: String,
    pub committed_at: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub planned_at: DateTime<Utc>,
    pub planner_name: String,
    pub planner_email: String,
}
