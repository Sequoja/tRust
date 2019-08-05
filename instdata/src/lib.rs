use bincode;
use serde::{Deserialize, Serialize};
use std::process;
use std::thread;
use time;

/// Default data structure for dynamic data
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DynData {
    pub system_time: u64,
    pub counter: u128,
    pub pid: u32,
    pub thread_id: String,
    pub machine_id: String,
}

impl DynData {
    /// Constructs new DynData object
    pub fn new(machine_id: &str) -> DynData {
        DynData {
            system_time: time::precise_time_ns(),
            counter: 0,
            pid: process::id(),
            thread_id: format!("{:?}", thread::current().id()),
            machine_id: String::from(machine_id),
        }
    }

    /// Updates dynamic Data
    pub fn update(&mut self) {
        self.counter += 1;
        self.system_time = time::precise_time_ns();
    }
}

/// Default data structure for static data
#[derive(Debug, Deserialize, Serialize)]
pub struct StaticData {
    pub absolute_path: String,
    pub description: String,
    pub ast_depth: u128,
    pub source_file: String,
    pub lines_begin: u128,
    pub lines_end: u128,
}

impl StaticData {
    /// Constructs new StaticData object
    pub fn new(
        absolute_path: &str,
        description: &str,
        ast_depth: u128,
        source_file: &str,
        lines_begin: u128,
        lines_end: u128,
    ) -> StaticData {
        StaticData {
            absolute_path: String::from(absolute_path),
            description: String::from(description),
            ast_depth,
            source_file: String::from(source_file),
            lines_begin,
            lines_end,
        }
    }
}

/// Convert for bincode to Rust struct
pub fn to_structs(bytes: &[u8]) -> Option<(DynData, StaticData)> {
    match bincode::deserialize(bytes) {
        Ok(data) => Some(data),
        Err(err) => {
            eprintln!("Unable to deserialize SendData: {}", err);
            None
        }
    }
}

/// Convert from Rust struct to bincode
pub fn to_bincode(dynamic_data: &DynData, static_data: &StaticData) -> Option<Vec<u8>> {
    match bincode::serialize(&(dynamic_data, static_data)) {
        Ok(serialized) => Some(serialized),
        Err(err) => {
            eprintln!("Unable to serialize SendData: {}", err);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
