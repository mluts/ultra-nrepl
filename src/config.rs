use failure::Error as StdError;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, OptionalExtension, NO_PARAMS};
use std::collections::HashMap;
use std::convert::From;
use std::path::PathBuf;

lazy_static! {
    static ref MIGRATIONS: Vec<(&'static str, &'static str)> = vec![(
        "v1",
        "
CREATE TABLE IF NOT EXISTS sessions(
  id INTEGER PRIMARY KEY,
  addr TEXT,
  session_id TEXT,
  ops_list TEXT
)

         "
    )];
}

///! Configuration-related facilities

#[derive(Debug, failure::Fail)]
pub enum Error {
    #[fail(display = "failed to parse sessions: {}", error)]
    SessionsParseError { error: serde_json::Error },

    #[fail(display = "had problems with reading sessions file: {}", ioerr)]
    SessionsReadError { ioerr: std::io::Error },
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SessionsParseError { error }
    }
}

impl From<std::io::Error> for Error {
    fn from(ioerr: std::io::Error) -> Self {
        Self::SessionsReadError { ioerr }
    }
}

/// Returns path to cli config directory
pub fn config_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap();

    dir.push("unrepl");

    dir
}

/// Returns path to serialized sessions map file
pub fn sessions_path() -> PathBuf {
    let mut dir = config_path();

    dir.push("sessions.json");

    dir
}

/// Helper for creating directory tree for config
pub fn ensure_config_dir() -> Result<(), std::io::Error> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(config_path())?;

    Ok(())
}

/// Deserializes sessions map from file
pub fn parse_sessions(f: &mut std::fs::File) -> Result<HashMap<String, String>, Error> {
    if f.metadata()?.len() == 0 {
        Ok(HashMap::new())
    } else {
        Ok(serde_json::from_reader(f)?)
    }
}

fn db_path() -> PathBuf {
    let mut dir = config_path();
    dir.push("db.sqlite");
    dir
}

pub fn open() -> Result<Connection, StdError> {
    let conn = Connection::open(db_path())?; //.map_err(|e| e.into())
    ensure_migrations(&conn)?;

    Ok(conn)
}

fn ensure_migrations_table(conn: &Connection) -> Result<(), StdError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migrations(
            name TEXT PRIMARY KEY
)
        ",
        NO_PARAMS,
    )?;
    Ok(())
}

fn run_migrations(conn: &Connection, migration: Option<String>) -> Result<(), StdError> {
    let mut run = migration.is_none();

    for (mig_name, mig_sql) in MIGRATIONS.iter() {
        if run {
            conn.execute(mig_sql, params!())?;
            conn.execute("INSERT INTO migrations VALUES (?)", params![mig_name])?;
        }

        if migration.is_some() && migration.as_ref().unwrap() == mig_name {
            run = true;
        }
    }

    Ok(())
}

fn ensure_migrations(conn: &Connection) -> Result<(), StdError> {
    ensure_migrations_table(conn)?;

    let latest_migration: Option<String> = conn
        .query_row(
            "SELECT name FROM migrations ORDER BY name DESC LIMIT 1",
            params!(),
            |row| row.get(0),
        )
        .optional()?;

    run_migrations(conn, latest_migration)?;

    Ok(())
}
