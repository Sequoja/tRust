//! # Instrumentation
//!
//! `instrumentation` provides instrumentation functionalities
use std::cell::RefCell;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

#[macro_use]
extern crate serde;
use dirs;
use mio::net::UdpSocket as MioUdpSocket;
use state::{LocalStorage, Storage};

use configuration::LocalConfig;
use instdata::DynData;

// Reexporting
pub use configuration::read_conf_file;
pub use instdata::StaticData;

/// Global singleton
pub static INSTRUMENTATION: Storage<GlobalInstrumentation> = Storage::new();

/// Thread-local singleton
pub static LOCAL_INST: LocalStorage<RefCell<ThreadLocalInst>> = LocalStorage::new();

/// Initializes global instrumentation object
/// Inserted at beginning of main thread
pub fn global_init() {
    INSTRUMENTATION.set(GlobalInstrumentation::init());
}

/// Initializes thread-local instrumentation object
/// Inserted at beginning of thread closure
pub fn local_init() -> JoinHandle<()> {
    LOCAL_INST.set(|| RefCell::new(ThreadLocalInst::new_empty()));
    LOCAL_INST
        .get()
        .borrow_mut()
        .init(INSTRUMENTATION.get().local_config.clone())
}

/// Instrumentation call
/// Inserted before and after line os interest
pub fn instrument(static_data: StaticData) {
    LOCAL_INST.get().borrow().instrument(static_data);
}

/// Joins the helper thread just before spawned thread ends
/// Inserted at end of thread closure
pub fn clean_up(handle: JoinHandle<()>) {
    LOCAL_INST.get().borrow().signal_finish();
    if let Some(err) = handle.join().err() {
        eprintln!("Unable to join helper thread: {:?}", err);
    }
    println!("{:?} - {}", thread::current().id(), "Join is done");
}

const CONFIG_FILE: &str = ".rust_inst/instconfig.toml";

/// Global instrumentation object. Holds static and global dyn data
pub struct GlobalInstrumentation {
    local_config: LocalConfig,
}

impl GlobalInstrumentation {
    /// Reads config JSON and constructs new GlobalInstrumentation object
    pub fn init() -> GlobalInstrumentation {
        // Set up global object
        let global_inst = GlobalInstrumentation::set_up_from_config();

        println!(
            "{:?} - {}",
            thread::current().id(),
            "=====> YOU MADE IT!! <====="
        );
        // Return GlobalInstrumentation object
        global_inst
    }

    /// Reads the config JSON file
    fn set_up_from_config() -> GlobalInstrumentation {
        if let Some(mut config_path) = dirs::home_dir() {
            config_path.push(CONFIG_FILE);
            let config = read_conf_file(config_path);
            GlobalInstrumentation {
                local_config: LocalConfig::new(config),
            }
        } else {
            eprintln!("Unable to locate home dir");
            panic!();
        }
    }
}

/// This object is initialized for every thread. It hold connection to helper thread
/// and signals the helper thread when a intrumentation call occures
pub struct ThreadLocalInst {
    /// Sending end of the channel used to signal helper thread
    channel_sender: Option<SyncSender<Message<StaticData>>>,
}

impl ThreadLocalInst {
    /// Consructs empty ThreadLocalInst object
    fn new_empty() -> ThreadLocalInst {
        ThreadLocalInst {
            channel_sender: None,
        }
    }

    /// Spawns helper thread and creates channel
    /// Return JoinHandle for helper thread
    fn init(&mut self, local_config: LocalConfig) -> JoinHandle<()> {
        // Create Channel
        let (sender, receiver) = sync_channel::<Message<StaticData>>(1);
        self.channel_sender = Some(sender);
        let parent_thread = thread::current().id();

        // Spawn new helper_thread and return join-handle
        thread::spawn(move || {
            println!("{:?}  -->  {:?}", parent_thread, thread::current().id());
            // Create InstHelper struct
            let inst_helper = InstHelper::new(receiver, local_config);

            // Run
            inst_helper.run();
        })
    }

    /// Actual intrumentation call. This method is inserted bevore and after every line of interest.
    /// Signals helper thread
    pub fn instrument(&self, static_data: StaticData) {
        println!(
            "{:?} - {} with {:?}",
            thread::current().id(),
            "Instrumentation call",
            &static_data
        );
        if let Some(channel) = self.channel_sender.as_ref() {
            match channel.send(Message::Instrument(static_data)) {
                Ok(()) => {}
                Err(e) => eprintln!(
                    "{:?} Unable to send instrumentation data down channel: {}",
                    thread::current().id(),
                    e
                ),
            }
        } else {
            eprintln!("Inst call: Unable to retrieve channel, not initialized.");
        }
    }

    /// Signals the helper_thread to finish
    fn signal_finish(&self) {
        println!("{:?} - {}", thread::current().id(), "Signal finish");
        if let Some(channel) = self.channel_sender.as_ref() {
            match channel.send(Message::Finish) {
                Ok(()) => {}
                Err(e) => eprintln!(
                    "{:?} Unable to signal helper thread to finish: {}",
                    thread::current().id(),
                    e
                ),
            }
        } else {
            eprintln!("Signal finish: Unable to retrieve channel, not initialized.");
        }
    }
}

enum Message<S> {
    Instrument(S),
    Finish,
}

/// Data structure for the helper_thread
struct InstHelper {
    /// Receiving end of channel. For receiving signals
    channel_receiver: Receiver<Message<StaticData>>,
    /// Dynamic data
    dynamic_data: DynData,
    /// UdpSocket
    udp_socket: MioUdpSocket,
}

impl InstHelper {
    /// Constructs a new InstHelper
    fn new(recv: Receiver<Message<StaticData>>, local_config: LocalConfig) -> InstHelper {
        let socket =
            MioUdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).expect("Unable to bind socket.");
        socket
            .connect(local_config.collector_addr)
            .expect("connect function failed");
        InstHelper {
            channel_receiver: recv,
            dynamic_data: DynData::new(&local_config.machine_id),
            udp_socket: socket,
        }
    }

    /// Waits until signaled, then updates dynamic data and sends data to Collector
    fn run(mut self) {
        println!("{:?} - {}", thread::current().id(), "Helper is running...");
        // Blocks until something was received via channel
        while let Ok(Message::Instrument(static_data)) = self.channel_receiver.recv() {
            // Update dynamic data
            self.dynamic_data.update();

            // Send entire inst data to collector process
            self.send_inst(static_data);
        }
        println!("{:?} - {}", thread::current().id(), "Terminating.");
    }

    /// Sends data to Collector
    fn send_inst(&self, static_data: StaticData) {
        if let Some(bincode) = instdata::to_bincode(&self.dynamic_data, &static_data) {
            if let Err(err) = self.udp_socket.send(&bincode) {
                // eprintln!("Unable to send data: {}", err)
            }
        }
        println!(
            "{:?} - {} with {:?} {:?}",
            thread::current().id(),
            "Sending item",
            self.dynamic_data,
            &static_data
        );
    }
}

mod configuration {
    use std::collections::HashMap;
    use std::fs;
    use std::net::SocketAddr;
    use toml;

    /// Config struct is constructed when reading the config file
    #[derive(Deserialize)]
    pub struct Config {
        pub code_2_monitor: Vec<(String, String)>,
        pub special_behaviour: Vec<(String, String)>,
        pub collector_ip: String,
        pub collector_port: u16,
        pub machine_id: String,
    }

    /// Read the config file
    pub fn read_conf_file<P: AsRef<std::path::Path>>(path: P) -> Config {
        match fs::read_to_string(path) {
            Err(err) => {
                eprintln!("Unable to read config file: {}", err);
                panic!()
            }
            Ok(content) => deserialize_config(&content),
        }
    }

    /// Deserializes the config file
    fn deserialize_config(content: &str) -> Config {
        match toml::from_str::<Config>(content) {
            Err(err) => {
                eprintln!("Unable to deserialize config: {}", err);
                panic!()
            }
            Ok(config) => config,
        }
    }

    /// Local config for thread local usage
    #[derive(Clone)]
    pub struct LocalConfig {
        pub collector_addr: SocketAddr,
        pub machine_id: String,
        pub special_behaviour: HashMap<String, String>,
    }

    impl LocalConfig {
        /// Construction of the thread local config struct
        pub fn new(config: Config) -> LocalConfig {
            match config.collector_ip.parse() {
                Ok(ip_addr) => LocalConfig {
                    collector_addr: SocketAddr::new(ip_addr, config.collector_port),
                    machine_id: config.machine_id,
                    special_behaviour: config.special_behaviour.iter().cloned().collect(),
                },
                Err(err) => {
                    eprintln!("Unable to parse ip address: {}", err);
                    panic!()
                }
            }
        }
    }
}

// TODO
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check() {
        // TODO
        unimplemented!()
    }
}
