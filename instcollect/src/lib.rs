use instdata::{to_structs, DynData, StaticData};
use r2d2::{self, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OpenFlags, NO_PARAMS, types::ToSql};
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::udp::UdpSocket;
use tokio::prelude::future::lazy;
use tokio::prelude::*;

const SOCKET_BINDING: &str = "0.0.0.0:8080";
const DB_FILE_NAME: &str = "instrumentation.db";
const DB_TABLE_NAME: &str = "instrumentation";

/// Configuration struct
#[derive(Clone)]
pub struct Config {
    db_name: String,
    table_name: String,
    run_name: String,
}

impl Config {
    /// Constructs new config struct
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() > 6 {
            return Err("too many arguments");
        } else if args.len() < 2 {
            return Err("not enough arguments");
        } else if [3, 5].contains(&args.len()) {
            return Err("either 'database_name' or 'table_name' was not provided");
        }
        let run_name = format!("run_{}", args[1].clone());
        let db_name = if args.len() >= 4 && args[2] == "--db" {
            args[3].clone()
        } else if args.len() >= 6 && args[4] == "--db" {
            args[5].clone()
        } else {
            // Default name
            String::from(DB_FILE_NAME)
        };
        let table_name = if args.len() >= 4 && args[2] == "--table" {
            args[3].clone()
        } else if args.len() >= 6 && args[4] == "--table" {
            args[5].clone()
        } else {
            // Default name
            String::from(DB_TABLE_NAME)
        };

        Ok(Config {
            db_name,
            table_name,
            run_name,
        })
    }
}

/// Runs the collector
pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    println!("\n---- Instrumentation Collector ----");
    println!("-----------------------------------\n");
    println!("Connecting to database '{}'...", config.db_name);

    let manager = SqliteConnectionManager::file(&config.db_name).with_flags(
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    );
    let pool = r2d2::Pool::new(manager)?;

    create_if_not_exist(pool.get().unwrap(), &config)?;

    println!("Inserting in table '{}'...", config.table_name);
    println!("Name of the instrumentation run: {}\n", config.run_name);

    println!("Waiting on port '8080' for instrumentation data...");
    println!("Control + C to exit");

    let incoming = SocketReader {
        socket: UdpSocket::bind(&SOCKET_BINDING.parse().unwrap())?,
    };
    let server = incoming.for_each(move |(message, sender_addr)| {
        let pool_handle = pool.clone();
        let config = config.clone();
        tokio::spawn(lazy(move || {
            if let Some((dyn_data, static_data)) = to_structs(&message) {
                println!("{:?}  -  {:?} ,  {:?}", sender_addr, dyn_data, static_data);
                // insert into sqlite db
                if let Ok(conn) = pool_handle.get() {
                    insert(conn, &config, dyn_data, static_data);
                    println!("Inserted in db");
                }
            }
            Ok(())
        }));
        Ok(())
    });

    tokio::run(server);
    Ok(())
}

/// SocketReader struct responsible for retrieve packages from the network interface
struct SocketReader {
    socket: UdpSocket,
}

impl Stream for SocketReader {
    type Item = (Vec<u8>, SocketAddr);
    type Error = ();

    /// Polls the feature
    fn poll(&mut self) -> Poll<Option<(Vec<u8>, SocketAddr)>, ()> {
        let mut buffer: Vec<u8> = vec![0; 1024];
        match self.socket.poll_recv_from(&mut buffer) {
            Ok(Async::Ready((_num_bytes, sender_addr))) => {
                Ok(Async::Ready(Some((buffer, sender_addr))))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => Err(()),
        }
    }
}

/// Creates new db table if does not exist
fn create_if_not_exist(
    conn: PooledConnection<SqliteConnectionManager>,
    config: &Config,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        format!(
            "CREATE TABLE IF NOT EXISTS {} {}",
            &config.table_name, DB_SCHEMA
        )
        .as_str(),
        NO_PARAMS,
    )?;
    Ok(())
}

/// Inserts incomming event data into the db
fn insert(
    conn: PooledConnection<SqliteConnectionManager>,
    config: &Config,
    dyn_data: DynData,
    static_data: StaticData,
) {
    let dyn_d_store = dyn_data.prepare_store();
    let params = dyn_d_store
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<_>>();
    if let Err(err) = conn.execute(
        format!("INSERT INTO {} {}", config.table_name, DB_INSERT).as_ref(),
        &static_data
            .prepare_store()
            .iter()
            .fold(params, |mut acc, x| {
                acc.push(x.as_ref());
                acc
            }),
    ) {
        eprintln!("Unable to insert instrumentation data: {}", err);
    }
}

/// Trait for storage preparation
trait PrepareStorage {
    fn prepare_store(self) -> Vec<Box<dyn ToSql>>;
}

impl PrepareStorage for StaticData {
    /// Prepares static data for database storage
    fn prepare_store(self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.absolute_path),
            Box::new(self.description),
            Box::new(self.ast_depth as f64),
            Box::new(self.source_file),
            Box::new(self.lines_begin as f64),
            Box::new(self.lines_end as f64),
        ]
    }
}

impl PrepareStorage for DynData {
    /// Prepares dynamic data for database storage
    fn prepare_store(self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.system_time as f64),
            Box::new(self.counter as f64),
            Box::new(self.pid),
            Box::new(self.thread_id),
            Box::new(self.machine_id),
        ]
    }
}

/// SQL statement
const DB_SCHEMA: &str = "(
    time_stamp      REAL    PRIMARY KEY,
    counter         REAL,
    pid             INTEGER,
    thread_id       TEXT,
    machine_id      TEXT,
    absolute_path   TEXT,
    description     TEXT,
    ast_depth       REAL,
    source_file     TEXT,
    lines_begin     REAL,
    lines_end       REAL)";

/// SQL statement
const DB_INSERT: &str = "(
    time_stamp,
    counter,
    pid,
    thread_id,
    machine_id,
    absolute_path,
    description,
    ast_depth,
    source_file,
    lines_begin,
    lines_end
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";