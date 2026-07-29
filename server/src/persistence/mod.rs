//! DB operations and schema migrations

// Touch this comment if changes only include .sql files and no .rs so that
// migration happens.
// nya~

pub(in crate::persistence) mod character;
pub mod character_loader;
pub mod character_updater;
mod diesel_to_rusqlite;
pub mod error;
mod json_models;
mod models;

use crate::persistence::character_updater::PetPersistenceData;
use common::comp;
use refinery::Report;
use rusqlite::{
    Connection, OpenFlags,
    trace::{TraceEvent, TraceEventCodes},
};
use std::{
    fs,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tracing::info;

// re-export waypoint parser for use to look up location names in character list
pub(crate) use character::parse_waypoint;

/// A struct of the components that are persisted to the DB for each character
#[derive(Debug)]
pub struct PersistedComponents {
    pub body: comp::Body,
    pub hardcore: Option<comp::Hardcore>,
    pub stats: comp::Stats,
    pub skill_set: comp::SkillSet,
    pub inventory: comp::Inventory,
    pub waypoint: Option<comp::Waypoint>,
    pub pets: Vec<PetPersistenceData>,
    pub active_abilities: comp::ActiveAbilities,
    pub map_marker: Option<comp::MapMarker>,
}

pub type EditableComponents = (comp::Body,);

// See: https://docs.rs/refinery/0.5.0/refinery/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `run_migrations` call below.
// `pub(crate)`, not private: `APEX-T4.5-FIXTURES` needs the real embedded
// migration set to build byte-real historical-schema fixtures (a partial
// `Target::Version` run), not a hand-approximated one.
pub(crate) mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./src/migrations");
}

/// A database connection blessed by Veloren.
pub(crate) struct VelorenConnection {
    connection: Connection,
    sql_log_mode: SqlLogMode,
}

impl VelorenConnection {
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            sql_log_mode: SqlLogMode::Disabled,
        }
    }

    /// Updates the SQLite log mode if DatabaseSetting.sql_log_mode has changed
    pub fn update_log_mode(&mut self, database_settings: &Arc<RwLock<DatabaseSettings>>) {
        let settings = database_settings
            .read()
            .expect("DatabaseSettings RwLock was poisoned");
        if self.sql_log_mode == settings.sql_log_mode {
            return;
        }

        set_log_mode(&mut self.connection, settings.sql_log_mode);
        self.sql_log_mode = settings.sql_log_mode;

        info!(
            "SQL log mode for connection changed to {:?}",
            settings.sql_log_mode
        );
    }
}

impl Deref for VelorenConnection {
    type Target = Connection;

    fn deref(&self) -> &Connection { &self.connection }
}

fn set_log_mode(connection: &mut Connection, sql_log_mode: SqlLogMode) {
    match sql_log_mode {
        SqlLogMode::Trace => {
            connection.trace_v2(
                TraceEventCodes::SQLITE_TRACE_STMT,
                Some(rusqlite_trace_callback),
            );
        },
        SqlLogMode::Profile => {
            connection.trace_v2(
                TraceEventCodes::SQLITE_TRACE_PROFILE,
                Some(rusqlite_trace_callback),
            );
        },
        SqlLogMode::Disabled => {
            connection.trace_v2(TraceEventCodes::empty(), None);
        },
    };
}

#[derive(Clone)]
pub struct DatabaseSettings {
    pub db_dir: PathBuf,
    pub sql_log_mode: SqlLogMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SqlLogMode {
    /// Logging is disabled
    #[default]
    Disabled,
    /// Records timings for each SQL statement
    Profile,
    /// Prints all executed SQL statements
    Trace,
}

impl SqlLogMode {
    pub fn variants() -> [&'static str; 3] { ["disabled", "profile", "trace"] }
}

impl core::str::FromStr for SqlLogMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "profile" => Ok(Self::Profile),
            "trace" => Ok(Self::Trace),
            _ => Err("Could not parse SqlLogMode"),
        }
    }
}

#[expect(clippy::to_string_trait_impl)]
impl ToString for SqlLogMode {
    fn to_string(&self) -> String {
        match self {
            SqlLogMode::Disabled => "disabled",
            SqlLogMode::Profile => "profile",
            SqlLogMode::Trace => "trace",
        }
        .into()
    }
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(settings: &DatabaseSettings) {
    let mut conn = establish_connection(settings, ConnectionMode::ReadWrite);

    diesel_to_rusqlite::migrate_from_diesel(&mut conn)
        .expect("One-time migration from Diesel to Refinery failed");

    // If migrations fail to run, the server cannot start since the database will
    // not be in the required state.
    // DET-MIG-001 (v12 save-migration, Critical): abort on a divergent
    // migration (name/checksum mismatch against the recorded history) rather
    // than silently accepting it — refinery's own default, loosened here at
    // some point without justification. A changed already-applied migration
    // means the database's transformation history no longer uniquely
    // identifies the code that produced it, i.e. ambiguous save provenance;
    // the determinism law requires that to fail closed. Corrections must be
    // new migration IDs, never edits to applied ones.
    //
    // NOTE (ship-policy, escalated to Ben — same shape as BLD-031(b)): this is
    // a live all-player-DB startup gate, so on a real database that carries a
    // divergent migration this now hard-panics at the `.expect` below. Whether
    // the SHIPPED behaviour should stay a hard failure or soften to
    // warn+record is Ben's call; this change makes the cert/determinism lane
    // correct now regardless.
    let report: Report = embedded::migrations::runner()
        .set_abort_divergent(true)
        .run(&mut conn.connection)
        .expect("Database migrations failed, server startup aborted");

    // DET-MIG-002 (v12 save-migration, High): record the identity of every
    // applied migration -- its ordered version/name and checksum -- not just a
    // count. The database's transformation history is authoritative save
    // provenance; logging only "Applied N migrations" means a later audit
    // cannot reconstruct which exact migrations ran from Bastion's own durable
    // evidence. This is pure diagnostic logging: no schema or save-format
    // change, and it pairs with DET-MIG-001's fail-closed divergence check.
    let applied = report.applied_migrations();
    if applied.is_empty() {
        info!("No database migrations were pending");
    } else {
        info!("Applied {} database migration(s):", applied.len());
        for migration in applied {
            // `migration` Displays as `<prefix><version>__<name>` (e.g.
            // `V12__add_foo`); checksum uniquely identifies the applied SQL.
            info!("  {} (checksum {})", migration, migration.checksum());
        }
    }
}

/// Runs after the migrations. In some cases, it can reclaim a significant
/// amount of space (reported 30%)
pub fn vacuum_database(settings: &DatabaseSettings) {
    let conn = establish_connection(settings, ConnectionMode::ReadWrite);

    conn.execute("VACUUM main", [])
        .expect("Database vacuuming failed, server startup aborted");

    info!("Database vacuumed");
}

// This callback uses info logging because it is never enabled by default,
// only when explicitly turned on via CLI arguments or interactive CLI commands.
// Setting it to anything other than info would remove the ability to get SQL
// logging from a running server that wasn't started at higher than info.
fn rusqlite_trace_callback(event: TraceEvent<'_>) {
    match event {
        TraceEvent::Stmt(_, msg) => info!("{}", msg),
        TraceEvent::Profile(stmt, dur) => info!("{} Duration: {:?}", stmt.sql(), dur),
        _ => (),
    }
}

pub(crate) fn establish_connection(
    settings: &DatabaseSettings,
    connection_mode: ConnectionMode,
) -> VelorenConnection {
    fs::create_dir_all(&settings.db_dir)
        .unwrap_or_else(|_| panic!("Failed to create saves directory: {:?}", settings.db_dir));

    let open_flags = OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | match connection_mode {
            ConnectionMode::ReadWrite => {
                OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE
            },
            ConnectionMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        };

    let connection = Connection::open_with_flags(settings.db_dir.join("db.sqlite"), open_flags)
        .unwrap_or_else(|err| {
            panic!(
                "Error connecting to {}, Error: {:?}",
                settings.db_dir.join("db.sqlite").display(),
                err
            )
        });

    let mut veloren_connection = VelorenConnection::new(connection);

    let connection = &mut veloren_connection.connection;

    set_log_mode(connection, settings.sql_log_mode);
    veloren_connection.sql_log_mode = settings.sql_log_mode;

    rusqlite::vtab::array::load_module(connection).expect("Failed to load sqlite array module");

    connection.set_prepared_statement_cache_capacity(100);

    // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
    // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to set foreign_keys PRAGMA");
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .expect("Failed to set journal_mode PRAGMA");
    connection
        .pragma_update(None, "busy_timeout", "250")
        .expect("Failed to set busy_timeout PRAGMA");

    veloren_connection
}
