# javalocate
[![license](https://img.shields.io/github/license/dameikle/javalocate.svg?maxAge=2592000)](https://github.com/dameikle/javalocate/blob/main/LICENSE)
[![build](https://github.com/dameikle/javalocate/actions/workflows/rust.yml/badge.svg)](https://github.com/dameikle/javalocate/actions)

Command line utility to find JVM versions on macOS - useful for setting _JAVA_HOME_, particularly on machines with different JVM versions. 

I'm thinking of you, Java Devs with Apple Silicon hardware 🐱‍💻

## Usage

The utility is designed to be used in a similar fashion to the _/usr/libexec/java_home_ by providing 
a number of flags that can be passed to control the selection.

These are shown below:

```
OPTIONS:
    -a, --arch <ARCH>          Architecture to filter on (e.g. x86_64, aarch64, amd64)
    -d, --detailed             Print out full details
    -f, --fail                 Return error code if no JVM found
    -h, --help                 Print help information
    -n, --name <NAME>          JVM Name to filter on
    -v, --version <VERSION>    Version to filter on (e.g. 1.8, 11, 17, etc)
```

### Outputs
By default, the utility ouputs a single path location to the "top" JVM found, ordered by decending version (i.e. Java 17 > Java 8).

Passing the detailed flag (_--detailed_ or _-d_) prints the full details of all JVMs found.

This flag can also be used in conjunction with filters to display full details for the filtered set.

### Filtering

The filtering options of _name_, _version_ and _arch_ can be used in isolation or together to fine tune the selection.

For example, to get the path to Java 17
```bash
javalocate -v 17
```

Or to get the path to the x86_64 JVM for Java 11 
```bash
javalocate -v 11 -a x86_64
```

Or to get the path to latest aarch64 JVM available
```bash
javalocate -a aarch64
```

You can also specify a minimum version by appending a _+_ to the version:
```bash
javalocate -v 1.8+
```

### Exit Code

By default, the utility returns an OK (0) exit code whether a JVM is found or not.

Setting the fail flag (_--fail or _-f_) changes this behaviour, returning a CONFIG ERROR (78) exit code.

This can be useful if you want to use the utility in a shell script.

For example, the below would return an error code if Java 11 or above could not be found when trying to set the _JAVA_HOME_ environment variable:
```bash
export JAVA_HOME=$(javalocate -v 11+ -f)
```

## Building

The utility is developed in Rust and can be build from source using:

```
cargo build
```

Or for a release version
```
cargo build --profile release
```

## Licence
Copyright 2021 David Meikle

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
