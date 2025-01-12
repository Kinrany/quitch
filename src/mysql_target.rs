use std::{fmt::Display, str::FromStr};

use anyhow::{anyhow, bail};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MysqlTarget {
    pub username: String,
    pub password: String,
    pub hostname: String,
    pub port: u16,
    pub db: String,
}

impl FromStr for MysqlTarget {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s)?;

        if url.scheme() != "mysql" {
            bail!("only mysql is supported");
        }

        Ok(MysqlTarget {
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
}

impl Display for MysqlTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let MysqlTarget {
            username,
            password,
            hostname,
            port,
            db,
        } = self;
        write!(f, "mysql://{username}:{password}@{hostname}:{port}/{db}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_connection_string() {
        assert_eq!(
            MysqlTarget::from_str("mysql://user:pass@localhost:3306/dbname").unwrap(),
            MysqlTarget {
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
            MysqlTarget {
                username: "user".into(),
                password: "pass".into(),
                hostname: "localhost".into(),
                port: 3306,
                db: "dbname".into(),
            }
            .to_string(),
            "mysql://user:pass@localhost:3306/dbname"
        );
    }
}
