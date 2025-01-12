use std::{collections::HashMap, future::ready};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use sqlx::{Executor, MySqlPool};

use crate::{FullChange, Plan};

#[derive(Clone, Copy, Debug, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Event {
    Deploy,
    Fail,
    Merge,
    Revert,
}

pub struct Registry {
    pool: MySqlPool,
}
impl Registry {
    pub fn new(pool: MySqlPool) -> Self {
        Registry { pool }
    }

    pub async fn apply_schema(&self) -> anyhow::Result<()> {
        eprintln!("Applying registry schema");
        static SCHEMA: &str = include_str!("./registry_schema.sql");
        self.pool
            .execute_many(SCHEMA)
            .take_while(|r| ready(r.is_ok()))
            .for_each(|_| ready(()))
            .await;
        Ok(())
    }

    /// Validate the contents of the registry against a plan.
    ///
    /// Return the first undeployed change in the plan, if any.
    pub async fn validate_against_plan(&self, plan: &Plan) -> anyhow::Result<Option<FullChange>> {
        let change_rows: Vec<ChangeRow> = sqlx::query_as("select * from `changes`")
            .fetch_all(&self.pool)
            .await?;
        let mut change_map: HashMap<_, _> = change_rows
            .into_iter()
            .map(|c| (c.change.change_id.clone(), c))
            .collect();
        for change in plan.full_changes() {
            let stored = change_map.remove(&change.id);
            if stored.is_none() {
                if !change_map.is_empty() {
                    eprintln!("Warning: found unknown changes");
                    for (change_id, change) in change_map {
                        eprintln!("{change_id} {}", change.change.name);
                    }
                }
                return Ok(Some(change));
            }
        }

        Ok(None)
    }

    pub async fn add_event(
        &self,
        event: Event,
        change: &FullChange,
        project: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "insert into `events` (
                `event`, `change_id`, `change`, `project`, `note`,
                `requires`, `conflicts`, `tags`,
                `committed_at`, `committer_name`, `committer_email`,
                `planned_at`, `planner_name`, `planner_email`
            ) values (
                ?, ?, ?, ?, ?,
                '', '', '',
                ?, ?, ?,
                ?, ?, ?
            )",
        )
        // Change
        .bind(event)
        .bind(&change.id)
        .bind(&change.change.name)
        .bind(project)
        .bind(&change.change.note)
        // Committer
        .bind(chrono::Utc::now())
        .bind("quitch")
        .bind("quitch@quitch")
        // Planner
        .bind(change.change.date)
        .bind(&change.change.planner)
        .bind(&change.change.planner)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_change(&self, change_id: &str) -> anyhow::Result<()> {
        sqlx::query("delete from `changes` where change_id = ?")
            .bind(change_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(dead_code)]
struct EventRow {
    event: Event,
    #[sqlx(flatten)]
    change: Change,

    requires: String,
    conflicts: String,
    tags: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(dead_code)]
struct ChangeRow {
    #[sqlx(flatten)]
    change: Change,
    script_hash: Option<String>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
#[expect(dead_code)]
struct Change {
    change_id: String,
    /// Name of the change
    #[sqlx(rename = "change")]
    name: String,
    project: String,
    note: String,
    committed_at: DateTime<Utc>,
    committer_name: String,
    committer_email: String,
    planned_at: DateTime<Utc>,
    planner_name: String,
    planner_email: String,
}
