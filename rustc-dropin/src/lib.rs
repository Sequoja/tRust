// Uses the compiler interface described in https://github.com/nrc/stupid-stats

#![feature(rustc_private)]

extern crate getopts;
extern crate rustc;
extern crate rustc_codegen_utils;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_metadata;
extern crate rustc_plugin;
extern crate syntax;

use instrument::read_conf_file;
mod insertfuncs;
mod instfinder;
use instfinder::InstFinder;
mod pathresolver;
use dirs;
use pathresolver::PathResolver;
use rustc::session::config::{self, ErrorOutputType, Input};
use rustc::session::Session;
use rustc_codegen_utils::codegen_backend::CodegenBackend;
use rustc_driver::driver::{CompileController, CompileState};
use rustc_driver::{Compilation, CompilerCalls, RustcDefaultCalls};
use rustc_metadata::cstore::CStore;
use std::path::PathBuf;
use syntax::{ast, errors};

const CONFIG_FILE: &str = ".rust_inst/instconfig.toml";

/// Struct implements CompilerCalls Trait
pub struct Instrumentator {
    default_calls: RustcDefaultCalls,
}

impl Instrumentator {
    pub fn new() -> Instrumentator {
        Instrumentator {
            default_calls: RustcDefaultCalls,
        }
    }
}

impl Default for Instrumentator {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> CompilerCalls<'a> for Instrumentator {
    fn early_callback(
        &mut self,
        _: &getopts::Matches,
        _: &config::Options,
        _: &ast::CrateConfig,
        _: &errors::registry::Registry,
        _: ErrorOutputType,
    ) -> Compilation {
        Compilation::Continue
    }

    fn late_callback(
        &mut self,
        code_backend: &CodegenBackend,
        matches: &getopts::Matches,
        session: &Session,
        cstore: &CStore,
        input: &Input,
        odir: &Option<PathBuf>,
        ofile: &Option<PathBuf>,
    ) -> Compilation {
        self.default_calls.late_callback(
            code_backend,
            matches,
            session,
            cstore,
            input,
            odir,
            ofile,
        );
        Compilation::Continue
    }

    fn some_input(
        &mut self,
        input: Input,
        input_path: Option<PathBuf>,
    ) -> (Input, Option<PathBuf>) {
        (input, input_path)
    }

    fn no_input(
        &mut self,
        m: &getopts::Matches,
        o: &config::Options,
        cc: &ast::CrateConfig,
        odir: &Option<PathBuf>,
        ofile: &Option<PathBuf>,
        r: &errors::registry::Registry,
    ) -> Option<(Input, Option<PathBuf>)> {
        self.default_calls.no_input(m, o, cc, odir, ofile, r);
        // This is not optimal error handling.
        panic!("No input supplied to stupid-stats");
    }

    /// Customize compiler driver
    fn build_controller(
        self: Box<Self>,
        _sess: &Session,
        _opts: &getopts::Matches,
    ) -> CompileController<'a> {
        // Default behavior
        let mut controller = CompileController::basic();
        
        // Define callback for hook after_parse
        controller.after_parse.callback = Box::new(|state: &mut CompileState| {
            // Check if parsing was successful
            if state.krate.is_some() {

                // Read config file
                let config = if let Some(mut config_path) = dirs::home_dir() {
                    config_path.push(CONFIG_FILE);
                    read_conf_file(config_path)
                } else {
                    panic!("Unable to locate home directory!")
                };

                // Prepare name resolution
                let resolv_paths = {
                    let mut path_resolver = PathResolver::new();
                    path_resolver.find_resolv_paths(state.krate.as_ref().unwrap())
                };

                // Insert instrumentation calls at relevant positions
                let mut inst_finder = InstFinder::new(
                    resolv_paths,
                    config.code_2_monitor,
                    &state.session.source_map(),
                );
                // Construct list of InstPoints
                inst_finder.find_inst_points(state.krate.as_ref().unwrap());
                // Insert appropirate instrumentation for each InstPoint
                inst_finder.insert_instrumentations();
            } else {
                panic!("Crate could not be parsed!");
            }
        });

        controller
    }
}

// TODO
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_test() {
        unimplemented!()
    }
}
