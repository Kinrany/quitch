mod change;
mod plan;
mod registry;

use std::{collections::HashMap, future::ready, path::Path};

use anyhow::{anyhow, bail};
use clap::Parser;
use futures::StreamExt;
use sqlx::{Executor, MySqlPool};
use url::Url;

use self::{
    plan::{FullChange, Plan},
    registry::ChangeRow,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ClientConfig {
    username: String,
    password: String,
    hostname: String,
    port: u16,
    db: String,
}

async fn load_plan(plan_file_path: &str) -> anyhow::Result<Plan> {
    eprintln!("Using plan file {plan_file_path}");
    let plan_string = tokio::fs::read_to_string(plan_file_path).await?;
    let plan = Plan::parse(&plan_string)?;
    if plan.is_empty() {
        eprintln!("Warning: the plan is empty");
    }
    Ok(plan)
}

// Will be used in `quitch show change`
#[allow(unused)]
fn format_plan_change(plan: &Plan, change_name: &str) -> anyhow::Result<String> {
    if let Some(change) = plan.full_changes().find(|c| c.name() == change_name) {
        Ok(change
            .change
            .format(plan.project(), change.parent)
            .expect("always succeeds"))
    } else {
        bail!("change not found in plan");
    }
}

fn parse_connection_string(s: &str) -> anyhow::Result<ClientConfig> {
    let url = Url::parse(s)?;

    if url.scheme() != "mysql" {
        bail!("only mysql is supported");
    }

    Ok(ClientConfig {
        hostname: url
            .host()
            .ok_or_else(|| anyhow!("missing hostname"))?
            .to_string(),
        port: url.port().unwrap_or(3306),
        username: url.username().to_string(),
        password: url
            .password()
            .ok_or_else(|| anyhow!("missing password"))?
            .to_string(),
        db: url.path().trim_start_matches('/').to_string(),
    })
}

fn format_connection_string(opts: &ClientConfig) -> String {
    let ClientConfig {
        username,
        password,
        hostname,
        port,
        db,
    } = opts;
    format!("mysql://{username}:{password}@{hostname}:{port}/{db}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommonArgs {
    registry: String,
    plan_file: String,
    connection_options: ClientConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, clap::Parser)]
enum Cli {
    #[clap(rename_all = "kebab-case")]
    Revert {
        #[clap(long, default_value = "sqitch")]
        registry: String,
        #[clap(long, default_value = "sqitch.plan")]
        plan_file: String,
        #[clap(long)]
        target: String,
    },
}
impl Cli {
    fn parse_common_args(self) -> anyhow::Result<CommonArgs> {
        match self {
            Self::Revert {
                registry,
                plan_file,
                target,
            } => Ok(CommonArgs {
                registry,
                plan_file,
                connection_options: parse_connection_string(&target)?,
            }),
        }
    }
}

async fn connect_db(config: &ClientConfig) -> anyhow::Result<MySqlPool> {
    let target = format_connection_string(config);
    eprintln!("Connecting to {target}");
    let pool = MySqlPool::connect(&target).await?;
    pool.execute("select 1").await?;
    eprintln!("Connected to {}", config.db);
    Ok(pool)
}

async fn create_schema_if_not_exists(pool: &MySqlPool, schema_name: &str) -> anyhow::Result<bool> {
    let rows = sqlx::query(
        "
        select schema_name
        from information_schema.schemata
        where schema_name = ?",
    )
    .bind(schema_name)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        eprintln!("Creating schema {schema_name}");
        // TODO: replace this hack
        if schema_name.contains('`') {
            unimplemented!("schema names with ` in them not supported");
        }
        pool.execute(format!("create schema `{schema_name}`").as_str())
            .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Connect to the main database and the registry
async fn connect(
    args: ClientConfig,
    registry_name: String,
) -> anyhow::Result<(MySqlPool, Registry)> {
    let db_client = connect_db(&args).await?;

    // Create a schema for the registry if it doesn't exist
    let must_apply_registry_schema =
        create_schema_if_not_exists(&db_client, &registry_name).await?;

    // Create the registry connection
    let registry_args = ClientConfig {
        db: registry_name,
        ..args
    };
    let registry_client = connect_db(&registry_args).await?;

    let registry = Registry {
        pool: registry_client,
    };

    // Apply the schema if the registry is newly created
    if must_apply_registry_schema {
        registry.apply_schema().await?;
    }

    Ok((db_client, registry))
}

#[derive(Debug, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
enum Event {
    Deploy,
    Fail,
    Merge,
    Revert,
}

struct Registry {
    pool: MySqlPool,
}
impl Registry {
    async fn apply_schema(&self) -> anyhow::Result<()> {
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
    async fn validate_against_plan(&self, plan: &Plan) -> anyhow::Result<Option<FullChange>> {
        let change_rows: Vec<ChangeRow> = sqlx::query_as("select * from `changes`")
            .fetch_all(&self.pool)
            .await?;
        let mut change_map: HashMap<_, _> = change_rows
            .into_iter()
            .map(|c| (c.change_id.clone(), c))
            .collect();
        for change in plan.full_changes() {
            let stored = change_map.remove(&change.id);
            if stored.is_none() {
                if !change_map.is_empty() {
                    eprintln!("Warning: found unknown changes");
                    for (change_id, change) in change_map {
                        eprintln!("{change_id} {}", change.change);
                    }
                }
                return Ok(Some(change));
            }
        }

        Ok(None)
    }

    async fn add_event(
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

    async fn delete_change(&self, change_id: &str) -> anyhow::Result<()> {
        sqlx::query("delete from `changes` where change_id = ?")
            .bind(change_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("Reverting only the last change by default");

    // Initial setup
    let common_args = Cli::parse().parse_common_args()?;
    let plan = load_plan(&common_args.plan_file).await?;
    let (db, registry) = connect(common_args.connection_options, common_args.registry).await?;

    // Make sure the registry is in a valid state
    let first_undeployed_change = registry.validate_against_plan(&plan).await?;

    // Find the last deployed change
    let last_deployed_change_id = if let Some(change) = first_undeployed_change {
        change.parent
    } else {
        plan.full_changes().last().map(|c| c.id)
    };
    let Some(last_deployed_change_id) = last_deployed_change_id else {
        eprint!("Nothing to revert");
        if plan.is_empty() {
            eprintln!(" (the plan is empty)");
        } else {
            eprintln!();
        }
        return Ok(());
    };
    let last_deployed_change = plan
        .full_changes()
        .find(|c| c.id == last_deployed_change_id)
        .expect("last_deployed_change_id is not in the plan");

    // Get the script corresponding to reverting the last deployed change
    eprintln!("Reverting {}", last_deployed_change.change.name);
    let plan_dir = Path::new(&common_args.plan_file)
        .parent()
        .expect("plan_dir");
    let revert_path = plan_dir
        .join("revert")
        .join(format!("{}.sql", last_deployed_change.name()));
    let revert_sql = tokio::fs::read_to_string(&revert_path).await?;

    // Revert the change
    let revert_the_change = async {
        let change = last_deployed_change.clone();
        db.execute_many(revert_sql.as_str())
            .take_while(|r| ready(r.is_ok()))
            .for_each(|_| ready(()))
            .await;
        registry.delete_change(&change.id).await?;
        registry
            .add_event(Event::Revert, &change, plan.project())
            .await?;
        anyhow::Ok(())
    };
    if let Err(error) = revert_the_change.await {
        eprintln!("Failed to revert");
        registry
            .add_event(Event::Revert, &last_deployed_change, plan.project())
            .await?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_connection_string() {
        assert_eq!(
            parse_connection_string("mysql://user:pass@localhost:3306/dbname").unwrap(),
            ClientConfig {
                username: "user".to_string(),
                password: "pass".to_string(),
                hostname: "localhost".to_string(),
                port: 3306,
                db: "dbname".to_string(),
            }
        );
    }

    #[test]
    fn test_format_connection_string() {
        assert_eq!(
            format_connection_string(&ClientConfig {
                username: "user".into(),
                password: "pass".into(),
                hostname: "localhost".into(),
                port: 3306,
                db: "dbname".into(),
            }),
            "mysql://user:pass@localhost:3306/dbname"
        );
    }

    #[test]
    fn test_parse_common_args() {
        assert_eq!(
            Cli::parse_from([
                "quitch",
                "revert",
                "--target",
                "mysql://user:pass@localhost:3306/dbname",
                "--registry",
                "quitch",
                "--plan-file",
                "./quitch.plan",
            ])
            .parse_common_args()
            .unwrap(),
            CommonArgs {
                registry: "quitch".to_string(),
                plan_file: "./quitch.plan".to_string(),
                connection_options: ClientConfig {
                    username: "user".to_string(),
                    password: "pass".to_string(),
                    hostname: "localhost".to_string(),
                    port: 3306,
                    db: "dbname".to_string(),
                },
            }
        );
    }
}
