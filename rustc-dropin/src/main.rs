#![feature(rustc_private)]

extern crate rustc_driver;

// use rustc_driver;
use rustc_dropin::Instrumentator;

/// Starting point of this program
fn main() {
    rustc_driver::run(|| {
        // Grab the command line arguments.
        let args: Vec<_> = std::env::args().collect();
        // Run the compiler driver
        rustc_driver::run_compiler(&args, Box::new(Instrumentator::new()), None, None)
    });
}
