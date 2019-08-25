# tRust

The tRust framework allows observing the execution of parallel Rust applications. This framework provides a modified Rust compiler for the automated insertion of probes into the observed program and its dependencies. A run-time library enables the transmission of observation data to a central collector application for persistent storage. This centrally collected data allows for extensive analysis of the run-time behavior of the program.


## Setting up the ```rustup``` toolchain

For Cargo to use the drop-in compiler provided by tRust it is necessary to register a cus- tom toolchain with rustup. The following describes how to set up a custom toolchain on Ubuntu. It is important that this is run after the drop-in compiler was built.

1. Create a new directory which will later contain the custom toolchain.
```bash
$ mkdir  ̃/.rust_custom_toolchains
```

1. Copy the entire toolchain used to build the drop-in compiler to the newly created directory.
```bash
$ cp -R  ̃/.rustup/toolchains/nightly-2019-02-07-x86_64-unknown-linux-gnu  ̃/.rust_custom_toolchains/rustinst
```

1. Copy the executable binary of the drop-in compiler inside the bin directory of the new toolchain.
```bash
$ cp path/to/where/built/rustc-dropin/is/rustc  ̃/.rust_custom_toolchains/rustinst/bin/
```

1. Use ```rustup``` to register the new rustinst toolchain.
```bash
$ rustup toolchain link rustinst  ̃/.rust_custom_toolchains/rustinst
```

1. Finally, override the default toolchain for the working directory containing the user program such that Cargo will automatically use the new rustinst toolchain.
```bash
$ rustup override set rustinst
```

## Description of the Configuration File

The configuration file containing the functions and methods of interest as well as the ad- dress (IP and port) of the machine running the collector application has to be stored in ``` ̃/.rust inst/instconfig.toml```. As the file extension indicates the file is formatted as TOML (Tom’s Obvious, Minimal Language), a common file format for configuration files in the Rust ecosystem. The various options for configuring tRust are explained in the following:

```toml
machine_id = "192.168.86.76"
collector_ip = "192.168.86.71"
collector_port = 8080
code_2_monitor = [
    ["", "ExternCrateItem"],
    ["main", "GlobalScope"],
    ["std::thread::spawn", "LocalScope"],
    ["par_iter_mut", "LocalScope"],
    ["rayon::join", "LocalScope"],
    ["join_context", "LocalScope"],
    ["crossbeam_channel::bounded", "InstCallForFunction"],
    ["send", "InstCallForMethod"],
    ["recv", "InstCallForMethod"],
    ["timely::execute_from_args", "LocalScope"],
    ["receive", "InstCallForMethod"],
]
```

- ```machine_id``` specifies the IP address of the current system. This address is sent as part of the static data to the collector application.
- ```collector_ip``` specifies the IP address of the machine running the collector application.
- ```collector_port``` specifies the port on which the collector application is listening.
- ```code_2_monitor``` specifies all the functions and methods which should receive instrumentation. Each function or method is   specified by its absolute name and the kind of instrumentation it should receive.
```["absoult func or method name", "instrumentation kind"]```
    In the following the different instrumentation kinds are explained:
    - ```ExternCrateItem``` defines the import statement. This has to be present in the config file at all times for tRust to work correctly.
    - ```GlobalScope``` defines which function in the application should be used for global initialization and finalization.
    - ```LocalScope``` defines functions and methods which introduces a new thread-local scope.
    - ```InstCallForFunction``` defines functions which should receive measurement instrumentation calls.
    - ```InstCallForMethod``` defines methods which should receive measurement instrumentation calls.
