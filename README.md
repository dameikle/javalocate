# javalocate
[![license](https://img.shields.io/github/license/dameikle/javalocate.svg?maxAge=2592000)](https://github.com/dameikle/javalocate/blob/main/LICENSE)
[![Crates.io](https://img.shields.io/crates/v/javalocate)](https://crates.io/crates/javalocate)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/dameikle/javalocate)](https://github.com/dameikle/javalocate/releases)
[![ubuntu build](https://github.com/dameikle/javalocate/actions/workflows/ubuntu.yml/badge.svg)](https://github.com/dameikle/javalocate/actions)
[![windows build](https://github.com/dameikle/javalocate/actions/workflows/windows.yml/badge.svg)](https://github.com/dameikle/javalocate/actions)
[![macOS build](https://github.com/dameikle/javalocate/actions/workflows/macos.yml/badge.svg)](https://github.com/dameikle/javalocate/actions)

Command line utility to find JVM versions on macOS, Linux (Debian, Ubuntu, RHEL/CentOS & Fedora) and Windows - useful 
for setting _JAVA_HOME_, particularly on machines with different JVM versions and architectures. 

I'm thinking of you, Java Devs with Apple Silicon hardware 🐱‍💻

## Install

The utility can be installed using Homebrew via the [homebrew-javalocate](https://github.com/dameikle/homebrew-javalocate) tap:
```bash
brew tap dameikle/javalocate
brew install javalocate
```
Or using Cargo via the [javalocate](https://crates.io/crates/javalocate) crate on crates.io:
```bash
cargo install javalocate
```

## Usage

The utility is designed to be used in a similar fashion to the _/usr/libexec/java_home_ by providing 
a number of flags that can be passed to control the selection.

These are shown below:

```
OPTIONS:
    -a, --arch <ARCH>                   Architecture to filter on (e.g. x86_64, aarch64, amd64)
    -d, --detailed                      Print out full details
    -f, --fail                          Return error code if no JVM found
    -h, --help                          Print help information
    -n, --name <NAME>                   JVM Name to filter on
    -v, --version <VERSION>             Version to filter on (e.g. 1.8, 11, 17, etc)
    -r, --register-location <LOCATION>  Registers a custom JVM location directory to search in
    -x, --remove-location <LOCATION>    Removes a registered custom JVM location directory
    -l, --display-locations             Displays all the custom JVM location directories that are registered
```

### Outputs
By default, the utility outputs a single path location to the "top" JVM found, ordered by descending version (i.e. Java 17 > Java 8), 
prioritising the system architecture (i.e. aarch64 > x86_64 on a Apple Silicon Mac).

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

## Default Locations

The utility looks in the default JVM installation locations for the following operating systems:

| Operating System | Location                               |
|------------------|----------------------------------------|
| macOS            | /Library/Java/JavaVirtualMachines      |
| Ubuntu           | /usr/lib/jvm                           |
| Debian           | /usr/lib/jvm                           |
| RHEL             | /usr/lib/jvm                           |
| CentOS           | /usr/lib/jvm                           |
| Fedora           | /usr/lib/jvm                           |
| Windows          | Registry - HKEY_LOCAL_MACHINE\Software |

It assumes that the _release_ file is included in the JVM package on Linux and Windows, and the _release_ file and
_Info.plist_ file is packaged on macOS.

Experimental support has been added to build information from path file name where _release_ file is not available. This
can occur on older JVMs.

## Custom Locations

You can add your own locations to search in using the Custom JVM Location options. This can be useful
if you maintain your own manually installed JVM collections.

For example, if you manually install JVMs into the the _/opt/jvms_ directory you can configure it to 
be searched using the _--register-location_ (-r) command:
```bash
javalocate -r /opt/jvms
javalocate --register-location /opt/jvms
```

If you want to then remove that location, you can use the _--remove-location_ (-x) command:
```bash
javalocate -x /opt/jvms
javalocate --remove-location /opt/jvms
```

You can list the currently registered location using the _--display-locations_ (-l) command:
```bash
javalocate -l
javalocate --display-locations
```

## Tips and Tricks

### Bash Alias

Adding the following to your _~/.bashrc_ (or _~/.bash_aliases_) file:

```bash
setjava() {
    export JAVA_HOME=`javalocate -v $1`
}
```

Allows you to quickly flip between versions:
```bash
setjava 17
echo $JAVA_HOME
setjava 8
echo $JAVA_HOME
setjava 11
echo $JAVA_HOME
```

### Powershell
You can set the version required in Powershell using the following syntax:
```powershell
$env:JAVA_HOME=$(javalocate.exe -v 11)
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
Copyright 2022 David Meikle

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
