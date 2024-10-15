<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_light.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_dark.svg">
  <img alt="OpenVAF" src="logo_dark.svg">
</picture>

<br>
<br>
<br>

## About this repository

This project is a fork of the ingenious work of **Pascal Kuthe [github](https://github.com/pascalkuthe/OpenVAF)**.

OpenVAF is a Verilog-A compiler that can compile Verilog-A files for use in circuit simulator.
The major aim of this Project is to provide a high-quality standard compliant compiler for Verilog-A.
Furthermore, the project aims to bring modern compiler construction algorithms/data structures to a field with a lack of such tooling.

Some highlights of OpenVAF include:

* **fast compile** times (usually below 1 second for most compact models)
* high-quality **user interface**
* **easy setup** (no runtime dependencies even for cross compilation)
* **fast simulations** surpassing existing solutions by 30%-60%, often matching handwritten models
* IDE aware design

Detailed documentation, examples and precompiled binaries of all release are **available on the [website](https://openvaf.semimod.de)**.

## Projects

The development of OpenVAF and related tools is tightly coupled and therefore happens in a single repository.
The work in this fork is focussed to following project:

### OpenVAF

OpenVAF is the main project of the repository and all other tools use OpenVAF as a library in some form.
OpenVAF can be build as a standalone CLI program that can compile Verilog-A files to shared objects that comply with the simulator independent OSDI interface.

OpenVAF has been tested with a NGSPICE prototype.
It can already support a large array of compact models.
However, due to the larger feature set additional testing and verification is still required.
Furthermore, some Verilog-A language features are currently not supported.

## Building OpenVAF with docker

The official docker image contains everything required for compiling OpenVAF. To build OpenVAF using the docker containers, simply run the following commands:

``` shell
git clone https://github.com/dwarning/OpenVAF.git && cd OpenVAF
# On REHL distros and fedora replace docker with podman
# on all commands below.
docker pull ghcr.io/pascalkuthe/ferris_ci_build_x86_64-unknown-linux-gnu:latest
# On Linux distros that enable SELinux linux RHEL based distros and fedora use $(pwd):/io:Z
docker run -ti -v $(pwd):/io ghcr.io/pascalkuthe/ferris_ci_build_x86_64-unknown-linux-gnu:latest

# Now you are inside the docker container
cd /io
cargo build --release
# OpenVAF will be build this can take a while
# afterwards the binary is available in target/release/openvaf
# inside the repository
```

## Building OpenVAF without docker

### Prerequisite under linux

OpenVAF **requires rust/cargo 1.64 or newer** (best installed with [rustup](https://rustup.rs/)). Furthermore, the **LLVM-15** development libraries and **clang-15** are required. Newer version also work but older versions of LLVM/clang are not supported. Note that its imperative that **you clang version matches your LLVM version**.
Cargo (rustc is included) should installed by the normal apt command.

On Debian and Ubuntu the [LLVM Project provided packages](https://apt.llvm.org/) can be used:

``` shell
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh <version number> all
```
Please use version number <17, e.g. 15.0.7 or 16.0.6. Higher gives trouble with some libs used in openVAF. Pascals docker image uses 16.

On fedora (37+) you can simply install LLVM from the default repositories:

``` shell
sudo dnf install clang llvm-devel
```

Environment variable (in case there are different LLVM versions installed): `LLVM_CONFIG=/usr/lib/llvm-16/bin/llvm-config`
PATH should include: `usr/lib/llvm-16/bin`

### Prerequisite under Windows

Download [rustup-init](https://win.rustup.rs), run it to install Cargo/Rust.
During installation select "Customize installation" and set profile to "complete".

Install Visual Studio Community Edition (tested with version 2019 and 2022). There are alternative ways only to install MSVC compiler, linker & other tools, also headers/libraries from Windows SDK by using this [download and install tool](https://github.com/Data-Oriented-House/PortableBuildTools).
Make sure you install CMake Tools that come with Visual Studio or by alternative windows installation.

Build LLVM and Clang, download [LLVM 16.0.6](https://github.com/llvm/llvm-project/releases/tag/llvmorg-16.0.6) sources (get the .zip file)

Unpack the sources. This creates directory `llvm-project-llvmorg-16.0.6`. Create a directory named `build` in parallel.

Start Visual Studio x64 native command prompt.
Run CMake, using nmake as build system (default). Alternativ Ninja build system is possible by given switch `-G ninja`.
Replace `c:\llvm` with the path where you want your LLVM and Clang binaries and libraries to be installed.
```
cmake -S llvm-project-llvmorg-16.0.6\llvm -B build -DCMAKE_INSTALL_PREFIX=C:\LLVM -DCMAKE_BUILD_TYPE=Release -DLLVM_TARGETS_TO_BUILD="X86;ARM;AArch64" -DLLVM_ENABLE_PROJECTS="llvm;clang"
```
Build and install:
```
cd build
nmake
nmake install
```
The LLVM build needs some time! 

Environment variable (in case there are different LLVM versions installed): `LLVM_CONFIG=C:\LLVM\bin\llvm-config.exe`
PATH should include: `C:\LLVM\bin`

### Build OpenVAF

To build OpenVAF you can run:

``` shell
cargo build --release --bin openvaf
```
Place the binary from `openvaf/target/release` in one location of your PATH.


By default, OpenVAF will link against the static LLVM libraries, to avoid runtime dependencies. This is great for creating portable binaries but sometimes building with shared libraries is preferable. Simply set the `LLVM_LINK_SHARED` environment variable during linking to use the shared system libraries. If multiple LLVM versions are installed (often the case on debian) the `LLVM_CONFIG` environment variable can be used to specify the path of the correct `llvm-config` binary.
An example build invocation using shared libraries on debian is shown below:

``` shell
LLVM_LINK_SHARED=1 LLVM_CONFIG="llvm-config-16" cargo build --release
```

OpenVAF includes many integration and unit tests inside its source code.
For development [cargo-nexttest](https://nexte.st/) is recommended to run these tests as it significantly reduces the test runtime.
However, the built-in cargo test runner (requires no extra installation) can also be used.
To run the testsuite simply call:

``` shell
cargo test # default test runner, requires no additional installation
cargo nextest run # using cargo-nextest, much faster but must be installed first
```

By default, the test suite will skip slow integration tests that compile entire compact models.
These can be enabled by setting the `RUN_SLOW_TESTS` environment variable:

``` shell
RUN_SLOW_TESTS=1 cargo nextest run
```

During development, you likely don't want to run full release builds as these
can take a while to build. Debug builds are much faster:
``` shell
cargo build # debug build
cargo run --bin openvaf test.va # create a debug build and run it
cargo clippy # check the sourcecode for errors/warnings without building (even faster)
```

## Download binaries and usage OpenVAF

You can download binaries for Linux and Windows [here](https://github.com/dwarning/OpenVAF/releases/tag/v1.0).
OpenVAF needs a linker. On Linux at most linker is part of the operating system - on windows you need the MS linker `link.exe` - see section `Prerequisite under Windows`.

``` shell
openvaf your_verilog-a-model.va
```

## Acknowledgement

Geoffrey Coram and Arpad Buermen are authors of several bugfixes included in this fork.

## Copyright

This work is free software and licensed under the GPL-3.0 license.
It contains code that is derived from [rustc](https://github.com/rust-lang/rust/) and [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer). These projects are both licensed under the MIT license. As required a copy of the license and disclaimer can be found in `copyright/LICENSE_MIT`.

Many models int integration tests folder are not licensed under a GPL compatible license. All of those models contain explicit license information. They do not endup in the openvaf binary in any way and therefore do not affect the license of the entire project. Integration tests without explicit model information (either in the model files or in a dedicated LICENSE file) fall under GPLv3.0 like the rest of the repo.
