use indexmap::IndexMap;
use itertools::Itertools;

use crate::change::Change;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    project: String,
    changes: Vec<Change>,
}

impl Plan {
    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn parse(plan_string: &str) -> anyhow::Result<Self> {
        let lines = plan_string.lines();
        if lines.clone().next() != Some("%syntax-version=1.0.0") {
            anyhow::bail!("Unsupported sqitch plan syntax");
        }

        // There are three types of lines:
        // - Meta lines that start with %
        // - Change lines
        // - Empty lines

        // Parse meta lines
        let meta_entries: IndexMap<&str, &str> = lines
            .clone()
            .filter_map(|line| line.strip_prefix('%'))
            .map(|line| {
                let mut parts = line.splitn(2, '=');
                let key = parts
                    .next()
                    .expect("splitn always returns at least one element");
                let value = parts.next().unwrap_or("");
                (key, value)
            })
            .collect();
        let project = meta_entries
            .get("project")
            .map_or_else(String::new, |s| s.to_string());

        // Change lines are lines that aren't meta lines or empty
        let changes: Vec<Change> = lines
            .filter(|line| !line.is_empty() && !line.starts_with('%'))
            .map(Change::parse_line)
            .try_collect()?;

        Ok(Plan { project, changes })
    }

    #[cfg(test)]
    pub fn format(&self) -> String {
        use std::iter::once;

        let meta_lines = vec![
            "%syntax-version=1.0.0".to_string(),
            format!("%project={}", self.project),
        ];
        let change_lines = self.changes.iter().map(Change::format_line);
        meta_lines
            .into_iter()
            .chain(once(String::new()))
            .chain(change_lines)
            .chain(once(String::new()))
            .join("\n")
    }

    pub fn full_changes(&self) -> impl Iterator<Item = FullChange> + '_ {
        let mut parent_id = None;
        self.changes.iter().map(move |change| {
            let change_id = change.id(&self.project, parent_id.clone());
            FullChange {
                change: change.clone(),
                id: change_id.clone(),
                parent: parent_id.replace(change_id),
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullChange {
    pub change: Change,
    pub id: String,
    pub parent: Option<String>,
}
impl FullChange {
    pub fn name(&self) -> &str {
        &self.change.name
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::DateTime;

    use crate::change::tests::example as example_change;

    use super::*;

    pub fn example() -> Plan {
        Plan {
            project: "quitch".into(),
            changes: vec![
                example_change(),
                Change {
                    date: DateTime::from_str("2024-03-10T00:04:24Z").unwrap(),
                    name: "change_num2".into(),
                    note: "Second change".into(),
                    planner: "Ruslan Fadeev <github@kinrany.dev>".into(),
                },
            ],
        }
    }

    pub static EXAMPLE_STRING: &str = "\
        %syntax-version=1.0.0\n\
        %project=quitch\n\
        \n\
        change_name 2024-03-07T03:19:34Z Ruslan Fadeev <github@kinrany.dev> # A description of the change\n\
        change_num2 2024-03-10T00:04:24Z Ruslan Fadeev <github@kinrany.dev> # Second change\n";

    #[test]
    fn test_parse() {
        let plan = Plan::parse(EXAMPLE_STRING).unwrap();
        assert_eq!(plan, example());
    }

    #[test]
    fn test_format_plus_parse() {
        let plan_string = example().format();
        let plan = Plan::parse(&plan_string).unwrap();
        assert_eq!(plan, example());
    }

    #[test]
    fn test_full_changes() {
        let plan = example();
        let full_changes: Vec<_> = plan.full_changes().collect();
        assert_eq!(
            full_changes,
            vec![
                FullChange {
                    change: example_change(),
                    id: "da41a550b0cba5bd3dffbf645032a98ae1136da5".into(),
                    parent: None,
                },
                FullChange {
                    change: Change {
                        date: DateTime::from_str("2024-03-10T00:04:24Z").unwrap(),
                        name: "change_num2".into(),
                        note: "Second change".into(),
                        planner: "Ruslan Fadeev <github@kinrany.dev>".into(),
                    },
                    id: "2959791f9fb4db4c322a9fdf121215d5e8a6a601".into(),
                    parent: Some("da41a550b0cba5bd3dffbf645032a98ae1136da5".into())
                }
            ]
        );
    }
}
