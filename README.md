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
