use instcollect;
use std::env;
use std::process;

fn main() {
    // Collect arguments 
    let args: Vec<String> = env::args().collect();
    // Display usage info
    if args.len() >= 2 && args[1] == "--help" {
        println!(
            "Usage:\n\n instcollect <run_name> [--db <database_name>] [--table <table_name>]\n"
        );
        println!("Or to display this usage info:\n instcollect --help");

        process::exit(1);
    }

    // Parse arguments
    let config = instcollect::Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}\n", err);
        eprintln!("Use option '--help' to display usage info.");

        process::exit(1);
    });
    // Run collector with parsed arguments
    if let Err(e) = instcollect::run(config) {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
