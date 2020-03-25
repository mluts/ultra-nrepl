use failure::Error as StdError;
use lazy_static::lazy_static;
use rusqlite::{params, Connection, OptionalExtension, NO_PARAMS};
use std::cell::RefCell;
use std::convert::From;
use std::path::PathBuf;

lazy_static! {
    static ref MIGRATIONS: Vec<(&'static str, &'static str)> = vec![(
        "v1",
        "
CREATE TABLE IF NOT EXISTS sessions(
  addr TEXT PRIMARY KEY,
  session_id TEXT,
  ops_list TEXT
)

         "
    )];
}

thread_local! {
    static DB: RefCell<Connection> = RefCell::new(open_db_connection().unwrap());
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

/// Helper for creating directory tree for config
pub fn ensure_config_dir() -> Result<(), std::io::Error> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(config_path())?;

    Ok(())
}

fn db_path() -> PathBuf {
    let mut dir = config_path();
    dir.push("db.sqlite");
    dir
}

pub fn open_db_connection() -> Result<Connection, StdError> {
    Connection::open(db_path()).map_err(|e| e.into())
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

pub fn ensure_migrations() -> Result<(), StdError> {
    DB.with(|conn| {
        let conn = conn.borrow();
        ensure_migrations_table(&conn)?;

        let latest_migration: Option<String> = conn
            .query_row(
                "SELECT name FROM migrations ORDER BY name DESC LIMIT 1",
                params!(),
                |row| row.get(0),
            )
            .optional()?;

        run_migrations(&conn, latest_migration)?;

        Ok(())
    })
}

pub fn save_session(session: Session) -> Result<(), StdError> {
    DB.with(|conn| {
        let conn = conn.borrow();

        conn.execute(
            "INSERT OR REPLACE
            INTO sessions (addr, session_id, ops_list)
            VALUES (?1, ?2, ?3)",
            params![session.addr, session.session, session.ops.join(",")],
        )?;

        Ok(())
    })
}

pub fn load_session(addr: String) -> Result<Option<Session>, StdError> {
    DB.with(|conn| {
        let conn = conn.borrow();

        conn.query_row(
            "SELECT addr, session_id, ops_list
            FROM sessions
            WHERE addr = ?",
            params![addr],
            |row| {
                Ok(Session::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<usize, String>(2)?
                        .split(",")
                        .map(|s| s.to_string())
                        .collect(),
                ))
            },
        )
        .optional()
        .map_err(|e| e.into())
    })
}

pub struct Session {
    addr: String,
    session: String,
    ops: Vec<String>,
}

impl Session {
    pub fn new(addr: String, session: String, ops: Vec<String>) -> Self {
        Self { addr, session, ops }
    }
    pub fn session(&self) -> String {
        self.session.to_string()
    }
}
