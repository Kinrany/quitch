use std::{fmt::Display, str::FromStr};

use anyhow::bail;
use chrono::{DateTime, Utc};
use sha1::{Digest, Sha1};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub name: String,
    pub note: String,
    pub date: DateTime<Utc>,
    pub planner: String,
}

impl Change {
    pub fn format(&self, project: &str, parent: Option<String>) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;

        let mut s = String::new();
        writeln!(&mut s, "project {}", project)?;
        writeln!(&mut s, "change {}", self.name)?;
        if let Some(parent) = parent {
            writeln!(&mut s, "parent {}", parent)?;
        }
        writeln!(&mut s, "planner {}", self.planner)?;
        writeln!(&mut s, "date {}", format_line_date(self.date))?;
        writeln!(&mut s)?;
        write!(&mut s, "{}", self.note)?;
        Ok(s)
    }

    pub fn id(&self, project: &str, parent_id: Option<String>) -> String {
        let change_str = self.format(project, parent_id).expect("always succeeds");
        let bytes = format!("change {}\0{change_str}", change_str.len());
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        let hash = hasher.finalize();
        base16ct::lower::encode_string(&hash)
    }

    pub fn parse_line(mut change: &str) -> anyhow::Result<Self> {
        fn index_of(s: &str, ch: char) -> Option<usize> {
            s.char_indices()
                .find_map(|(idx, ch2)| (ch2 == ch).then_some(idx))
        }

        let Some(name_end_idx) = index_of(change, ' ') else {
            bail!("missing space after name");
        };
        let name = change[..name_end_idx].to_string();
        change = change[name_end_idx..].trim_start();

        let Some(date_end_idx) = index_of(change, ' ') else {
            bail!("missing space after date");
        };
        let date = DateTime::from_str(&change[..date_end_idx])?;
        change = change[date_end_idx..].trim_start();

        let (planner, note) = match index_of(change, '#') {
            Some(planner_end_idx) => (
                change[..planner_end_idx].trim().to_string(),
                change[planner_end_idx + 1..].trim().replace("\\n", "\n"),
            ),
            None => (change.trim().to_string(), String::new()),
        };

        Ok(Self {
            name,
            note,
            date,
            planner,
        })
    }

    #[cfg(test)]
    pub fn format_line(&self) -> String {
        format!(
            "{} {} {} # {}",
            self.name,
            format_line_date(self.date),
            self.planner,
            self.note.replace('\n', "\\n"),
        )
    }
}

pub fn format_line_date(date: DateTime<Utc>) -> impl Display {
    date.format("%FT%TZ")
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn example() -> Change {
        Change {
            date: DateTime::from_str("2024-03-07T03:19:34Z").unwrap(),
            name: "change_name".into(),
            note: "A description of the change".into(),
            planner: "Ruslan Fadeev <github@kinrany.dev>".into(),
        }
    }

    pub static EXAMPLE_STRING: &str = "project quitch\n\
        change change_name\n\
        planner Ruslan Fadeev <github@kinrany.dev>\n\
        date 2024-03-07T03:19:34Z\n\
        \n\
        A description of the change";

    pub static EXAMPLE_LINE: &str = "change_name \
        2024-03-07T03:19:34Z \
        Ruslan Fadeev <github@kinrany.dev> \
        # A description of the change";

    #[test]
    fn test_format() {
        let formatted_change = example().format("quitch", None).unwrap();
        assert_eq!(formatted_change, EXAMPLE_STRING);
    }

    #[test]
    fn test_id_without_parent() {
        assert_eq!(
            example().id("quitch", None),
            "da41a550b0cba5bd3dffbf645032a98ae1136da5",
        );
    }

    #[test]
    fn test_id_with_parent() {
        assert_eq!(
            example().id(
                "quitch",
                Some("da41a550b0cba5bd3dffbf645032a98ae1136da5".to_string())
            ),
            "7b6b9ba12694a34a5445e1d847d36d2344d61bcb"
        );
    }

    #[test]
    fn test_id_with_unicode_note() {
        let mut change = example();
        change.note = "🤦🏼‍♂️".into();
        assert_eq!(
            change.id("quitch", None),
            "fb29c4f840ce9cd266d983a2c90d7ddf745c1711"
        );
    }

    #[test]
    fn test_parse_line() {
        let change = Change::parse_line(EXAMPLE_LINE).unwrap();
        assert_eq!(change, example());
    }

    #[test]
    fn test_format_plus_parse_line() {
        let change_text = example().format_line();
        let change = Change::parse_line(&change_text).unwrap();
        assert_eq!(change, example());
    }

    #[test]
    fn test_parse_line_with_newlines() {
        let note = "a\\nb";
        let change =
            Change::parse_line(&format!("name 2000-01-01T00:00:00Z author # {note}")).unwrap();
        assert_eq!(change.note, "a\nb");
    }

    #[test]
    fn test_format_line_with_newlines() {
        let note = "a\nb".to_string();
        let change = Change { note, ..example() };
        let change_text = change.format_line();
        assert!(change_text.contains("a\\nb"));
    }
}
