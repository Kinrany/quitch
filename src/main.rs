mod change;
mod mysql_target;
mod plan;
mod registry;

use std::{future::ready, path::Path};

use anyhow::bail;
use clap::Parser;
use futures::StreamExt;
use mysql_target::MysqlTarget;
use sqlx::{Executor, MySqlPool};

use self::{
    plan::{FullChange, Plan},
    registry::{Event, Registry},
};

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

#[derive(Clone, Debug, PartialEq, Eq, clap::Parser)]
struct CommonArgs {
    #[clap(long, default_value = "sqitch")]
    registry: String,
    #[clap(long, default_value = "sqitch.plan")]
    plan_file: String,
    #[clap(long)]
    target: MysqlTarget,
}
impl CommonArgs {
    fn top_dir(&self) -> &Path {
        Path::new(&self.plan_file).parent().expect("plan_dir")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, clap::Parser)]
#[clap(rename_all = "kebab-case")]
struct Cli {
    #[clap(flatten)]
    common_args: CommonArgs,

    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clone, Debug, PartialEq, Eq, clap::Parser)]
enum Command {
    Revert,
    Deploy,
}

async fn connect_db(target: &MysqlTarget) -> anyhow::Result<MySqlPool> {
    eprintln!("Connecting to {target}");
    let pool = MySqlPool::connect(&target.to_string()).await?;
    pool.execute("select 1").await?;
    eprintln!("Connected to {}", target.db);
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
    args: MysqlTarget,
    registry_name: String,
) -> anyhow::Result<(MySqlPool, Registry)> {
    let db_client = connect_db(&args).await?;

    // Create a schema for the registry if it doesn't exist
    let must_apply_registry_schema =
        create_schema_if_not_exists(&db_client, &registry_name).await?;

    // Create the registry connection
    let registry_args = MysqlTarget {
        db: registry_name,
        ..args
    };
    let registry_client = connect_db(&registry_args).await?;

    let registry = Registry::new(registry_client);

    // Apply the schema if the registry is newly created
    if must_apply_registry_schema {
        registry.apply_schema().await?;
    }

    Ok((db_client, registry))
}

async fn revert(
    db: &MySqlPool,
    registry: &Registry,
    plan: Plan,
    top_dir: &Path,
) -> anyhow::Result<()> {
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
    let revert_path = top_dir
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("Reverting only the last change by default");

    // Initial setup
    let cli = Cli::parse();
    let top_dir = cli.common_args.top_dir().to_path_buf();
    let plan = load_plan(&cli.common_args.plan_file).await?;
    let (db, registry) = connect(cli.common_args.target, cli.common_args.registry).await?;

    match cli.cmd {
        Command::Revert => {
            revert(&db, &registry, plan, &top_dir).await?;
        }
        Command::Deploy => unimplemented!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_common_args() {
        assert_eq!(
            Cli::parse_from([
                "quitch",
                "--target",
                "mysql://user:pass@localhost:3306/dbname",
                "--registry",
                "quitch",
                "--plan-file",
                "./quitch.plan",
                "revert",
            ]),
            Cli {
                common_args: CommonArgs {
                    registry: "quitch".to_string(),
                    plan_file: "./quitch.plan".to_string(),
                    target: MysqlTarget {
                        username: "user".to_string(),
                        password: "pass".to_string(),
                        hostname: "localhost".to_string(),
                        port: 3306,
                        db: "dbname".to_string(),
                    },
                },
                cmd: Command::Revert
            }
        );
    }
}
