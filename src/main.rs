use std::cmp::Ordering;
#[cfg(target_os = "linux")]
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::{Command, Stdio};
use serde::{Serialize, Deserialize};
use clap::Parser;
use java_properties::read;
#[cfg(target_os = "macos")]
use plist::Value;

#[cfg(target_os = "windows")]
extern crate winreg;
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::HKEY_LOCAL_MACHINE;


/// Command line utility to find JVM versions on macOS, Linux and Windows
#[derive(Parser, Debug)]
#[clap(author, about, version, long_about = None)]
struct Args {
    /// JVM Name to filter on
    #[clap(short, long)]
    name: Option<String>,

    /// Architecture to filter on (e.g. x86_64, aarch64, amd64)
    #[clap(short, long)]
    arch: Option<String>,

    /// Version to filter on (e.g. 1.8, 11, 17, etc)
    #[clap(short, long)]
    version: Option<String>,

    /// Print out full details
    #[clap(short, long)]
    detailed: bool,

    /// Return error code if no JVM found
    #[clap(short, long)]
    fail: bool,

    /// Add location
    #[clap(short = 'r', long)]
    register_location: Option<String>,

    /// Remove location
    #[clap(short = 'x', long)]
    remove_location: Option<String>,

    /// Display locations
    #[clap(short = 'l', long)]
    display_locations: bool
}

#[derive(Clone)]
struct Jvm {
    version: String,
    name: String,
    architecture: String,
    path: String
}

#[derive(Clone)]
struct OperatingSystem {
    name: String,
    architecture: String
}

#[derive(Serialize, Deserialize)]
struct Config {
    paths: Vec<String>
}

impl Default for Config {
    fn default() -> Self {
        Config {
            paths: vec![]
        }
    }
}

fn main() {
    let args = Args::parse();
    let mut cfg: Config = confy::load("javalocate").unwrap();

    if !args.register_location.is_none() {
        let location = args.register_location.as_ref().unwrap().as_str().to_string();
        if !cfg.paths.contains(&location) {
            cfg.paths.push(location);
            confy::store("javalocate", &cfg).unwrap();
        }
        std::process::exit(exitcode::OK);
    }

    if !args.remove_location.is_none() {
        let location = args.remove_location.as_ref().unwrap().as_str().to_string();
        if let Some(pos) = cfg.paths.iter().position(|x| *x == location) {
            cfg.paths.remove(pos);
        }
        confy::store("javalocate", &cfg).unwrap();
        std::process::exit(exitcode::OK);
    }

    if args.display_locations {
        if cfg.paths.is_empty() {
            println!("No custom JVM locations registered");
        } else {
            println!("Custom JVM locations registered:");
            for tmp in cfg.paths {
                println!("{}", tmp);
            }
        }
        std::process::exit(exitcode::OK);
    }

    // Fetch default java architecture based on kernel
    let operating_system = get_operating_system();

    // Build and filter JVMs
    let jvms: Vec<Jvm> = collate_jvms(&operating_system, &cfg)
        .into_iter()
        .filter(|tmp| filter_arch(&args.arch, tmp))
        .filter(|tmp| filter_ver(&args.version, tmp))
        .filter(|tmp| filter_name(&args.name, tmp))
        .collect();

    // If empty decide on response based on fail param
    if jvms.is_empty() {
        if args.fail {
            eprintln!("Couldn't find a JVM to use.");
            std::process::exit(exitcode::CONFIG);
        } else {
            std::process::exit(exitcode::OK);
        }
    }

    // If JVMs found, display
    if args.detailed {
        for jvm in &jvms {
            println!("{} ({}) \"{}\" - {}",
                     jvm.version,
                     jvm.architecture,
                     jvm.name,
                     jvm.path
            );
        }
    }
    else {
        println!("{}", jvms.first().unwrap().path);
    }
}


#[cfg(any(target_os = "linux", target_os = "macos"))]
fn get_operating_system() -> OperatingSystem {
    let output = Command::new("uname")
        .arg("-ps")
        .stdout(Stdio::piped())
        .output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parts: Vec<String> =
        stdout.split(" ").map(|s| s.to_string()).collect();

    let os = trim_string(parts.get(0).unwrap().as_str());
    let arch = trim_string(parts.get(1).unwrap().as_str());

    let default_architecture =
        if os.eq_ignore_ascii_case("Darwin") {
            if arch.eq_ignore_ascii_case("arm") {
                "aarch64".to_string()
            } else {
                "x86_64".to_string()
            }
        } else if os.eq_ignore_ascii_case("Linux") {
            if arch.eq_ignore_ascii_case("x86_64") {
                "x86_64".to_string()
            } else if arch.eq_ignore_ascii_case("i386") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("i586") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("i686") {
                "x86".to_string()
            } else if arch.eq_ignore_ascii_case("aarch64") {
                "aarch64".to_string()
            } else if arch.eq_ignore_ascii_case("arm64") {
                "arm64".to_string()
            } else {
                eprintln!("{} architecture is unknown on Linux", arch);
                std::process::exit(exitcode::UNAVAILABLE);
            }
        } else {
            eprintln!("Running on non-supported operation system");
            std::process::exit(exitcode::UNAVAILABLE);
        };

    let mut name = String::new();
    if os.eq_ignore_ascii_case("Linux") {
        // Attempt to load the Release file into HashMap
        let release_file = File::open("/etc/os-release");
        let release_file = match release_file {
            Ok(release_file) => release_file,
            Err(_error) => std::process::exit(exitcode::UNAVAILABLE),
        };
        let properties = read(BufReader::new(release_file)).unwrap();
        name.push_str(properties.get("ID").unwrap_or(&"".to_string()).replace("\"", "").as_str());
    } else if os.eq_ignore_ascii_case("Darwin") {
        name.push_str("macOS");
    }

    return OperatingSystem {
        name,
        architecture: default_architecture
    }
}

#[cfg(target_os = "windows")]
fn get_operating_system() -> OperatingSystem {
    let current_version = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion").unwrap();
    let name: String = current_version.get_value("ProductName").unwrap();

    let environment = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment").unwrap();
    let arch: String = environment.get_value("PROCESSOR_ARCHITECTURE").unwrap();
    let default_architecture =
        if arch.eq_ignore_ascii_case("amd64") {
            "x86_64".to_string()
        } else if arch.eq_ignore_ascii_case("x86") {
            "x86".to_string()
        } else if arch.eq_ignore_ascii_case("arm64") {
            "arm64".to_string()
        } else {
            eprintln!("Unknown processor architecture");
            std::process::exit(exitcode::UNAVAILABLE);
        };

    return OperatingSystem {
        name,
        architecture: default_architecture
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn trim_string(value: &str) -> &str {
    value.strip_suffix("\r\n")
        .or(value.strip_suffix("\n"))
        .unwrap_or(value)
}

#[cfg(target_os = "linux")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> Vec<Jvm> {
    let mut jvms = Vec::new();
    let dir_lookup = HashMap::from(
        [("ubuntu".to_string(), "/usr/lib/jvm".to_string()),
            ("debian".to_string(), "/usr/lib/jvm".to_string()),
            ("rhel".to_string(), "/usr/lib/jvm".to_string()),
            ("centos".to_string(), "/usr/lib/jvm".to_string()),
            ("fedora".to_string(), "/usr/lib/jvm".to_string())]);

    let path = dir_lookup.get(os.name.as_str());
    if path.is_none() && cfg.paths.is_empty() {
        eprintln!("Default JVM path is unknown on {} Linux", os.name);
        std::process::exit(exitcode::UNAVAILABLE);
    }
    let mut paths = cfg.paths.to_vec();
    paths.push(path.unwrap().to_string());

    for path in paths {
        for path in fs::read_dir(path).unwrap() {
            let path = path.unwrap().path();
            let metadata = fs::metadata(&path).unwrap();
            let link = fs::read_link(&path);

            if metadata.is_dir() && link.is_err() {
                // Attempt to use release file, if not, attempt to build from folder name
                let release_file = File::open(path.join("release"));
                if release_file.is_ok() {
                    // Collate required information
                    let properties = read(BufReader::new(release_file.unwrap())).unwrap();
                    let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
                    let architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();

                    // Build JVM Struct
                    let tmp_jvm = Jvm {
                        version,
                        architecture,
                        name,
                        path: path.to_str().unwrap().to_string(),
                    };
                    jvms.push(tmp_jvm);
                } else {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    let parts: Vec<String> = file_name.split("-").map(|s| s.to_string()).collect();
                    // Assuming four part or more form - e.g. "java-8-openjdk-amd64"
                    if parts.len() < 3 || !parts.get(1).unwrap().to_string().eq("java") {
                        continue;
                    }

                    let version = parts.get(1).unwrap().to_string();
                    let mut architecture = parts.get(3).unwrap().to_string();
                    architecture = architecture.replace("amd64", "x86_64");
                    architecture = architecture.replace("i386", "x86");
                    let name = file_name.to_string();

                    // Build JVM Struct
                    let tmp_jvm = Jvm {
                        version,
                        architecture,
                        name,
                        path: path.to_str().unwrap().to_string(),
                    };
                    jvms.push(tmp_jvm);
                }
            }
        }
    }
    jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return jvms;
}

#[cfg(target_os = "macos")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> Vec<Jvm> {
    assert!(os.name.contains("macOS"));
    let mut jvms = Vec::new();
    let mut paths = cfg.paths.to_vec();
    paths.push("/Library/Java/JavaVirtualMachines".to_string());
    for path in paths {
        for path in fs::read_dir(path).unwrap() {
            let path = path.unwrap().path();
            let metadata = fs::metadata(&path).unwrap();

            if metadata.is_dir() {
                // Attempt to load the Info PList
                let info =
                    Value::from_file(path.join("Contents/Info.plist"));

                let info = match info {
                    Ok(info) => info,
                    Err(_error) => continue,
                };
                let name = info
                    .as_dictionary()
                    .and_then(|dict| dict.get("CFBundleName"))
                    .and_then(|info_string| info_string.as_string());
                let name = name.unwrap_or(&"".to_string()).replace("\"", "");

                // Attempt to load the Release file into HashMap
                let release_file = File::open(path.join("Contents/Home/release"));
                let release_file = match release_file {
                    Ok(release_file) => release_file,
                    Err(_error) => continue,
                };

                // Collate required information
                let properties = read(BufReader::new(release_file)).unwrap();
                let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
                let architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");

                // Build JVM Struct
                let tmp_jvm = Jvm {
                    version,
                    architecture,
                    name,
                    path: path.join("Contents/Home").to_str().unwrap().to_string(),
                };
                jvms.push(tmp_jvm);
            }
        }
    }
    jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return jvms;
}

#[cfg(target_os = "windows")]
fn collate_jvms(os: &OperatingSystem, cfg: &Config) -> Vec<Jvm> {
    assert!(os.name.contains("Windows"));
    let mut jvms = Vec::new();

    // Loop round software keys in the registry
    let system = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE").unwrap();
    for name in system.enum_keys().map(|x| x.unwrap()) {
        let software: String = name.clone();
        // Find software with JDK key
        for jdk in system.open_subkey(name).unwrap().enum_keys()
                            .map(|x| x.unwrap())
                            .filter(|x| x.starts_with("JDK")) {
            // Next key should be JVM
            for jvm in system.open_subkey(format!("{}\\{}", software, jdk)).unwrap().enum_keys().map(|x| x.unwrap()) {
                let mut jvm_path = String::new();
                // Old style JavaSoftware entry
                let java_home: Result<String, _> = system.open_subkey(format!("{}\\{}\\{}", software, jdk, jvm)).unwrap().get_value("JavaHome");
                if java_home.is_ok() {
                    jvm_path = java_home.unwrap();
                }
                // Per JVM Entry - check for Hotspot or OpenJ9 entry
                let hotspot_path: Result<RegKey, _> = system.open_subkey(format!("{}\\{}\\{}\\hotspot\\MSI", software, jdk, jvm));
                if hotspot_path.is_ok() {
                    jvm_path = hotspot_path.unwrap().get_value("Path").unwrap();
                }
                let openj9_path: Result<RegKey, _> = system.open_subkey(format!("{}\\{}\\{}\\openj9\\MSI", software, jdk, jvm));
                if openj9_path.is_ok() {
                    jvm_path = openj9_path.unwrap().get_value("Path").unwrap();
                }
                jvm_path = jvm_path.strip_suffix("\\").unwrap_or(jvm_path.as_str()).to_string();

                let path = Path::new(jvm_path.as_str()).join("release");
                let release_file = File::open(path);
                if release_file.is_ok() {
                    jvms.push(process_release_file(&jvm_path, release_file.unwrap()));
                }
            }
        }
    }
    // Read from Custom JVM Location Paths
    if !cfg.paths.is_empty() {
        for path in &cfg.paths {
            for path in fs::read_dir(path).unwrap() {
                let jvm_path = path.unwrap().path();
                let metadata = fs::metadata(&jvm_path).unwrap();

                if metadata.is_dir() {
                    let path = Path::new(jvm_path.to_str().unwrap()).join("release");
                    let release_file = File::open(&path);
                    if release_file.is_ok() {
                        jvms.push(process_release_file(&path.to_str().unwrap().to_string(), release_file.unwrap()));
                    }
                }

            }
        }
    }
    jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return jvms;
}

#[cfg(target_os = "windows")]
fn process_release_file(jvm_path: &String, release_file: File) -> Jvm {
    // Collate required information
    let properties = read(BufReader::new(release_file)).unwrap();
    let version = properties.get("JAVA_VERSION").unwrap_or(&"".to_string()).replace("\"", "");
    let mut architecture = properties.get("OS_ARCH").unwrap_or(&"".to_string()).replace("\"", "");
    architecture = architecture.replace("amd64", "x86_64");
    architecture = architecture.replace("i386", "x86");
    let implementor = properties.get("IMPLEMENTOR").unwrap_or(&"".to_string()).replace("\"", "");
    let name = format!("{} - {}", implementor, version);

    // Build JVM Struct
    let tmp_jvm = Jvm {
        version,
        architecture,
        name,
        path: jvm_path.to_string(),
    };
    tmp_jvm
}

fn compare_boosting_architecture(a: &Jvm, b: &Jvm, default_arch: &String) -> Ordering {
    let version_test = compare_version_values(&b.version, &a.version);
    if version_test == Ordering::Equal {
        if b.architecture != default_arch.as_str() && a.architecture == default_arch.as_str() {
            return Ordering::Less;
        }
        if b.architecture == default_arch.as_str() && a.architecture != default_arch.as_str() {
            return Ordering::Greater;
        }
    }
    return version_test;
}

fn filter_ver(ver: &Option<String>, jvm: &Jvm) -> bool {
    if !ver.is_none() {
        let version = ver.as_ref().unwrap();
        if version.contains("+") {
            let sanitised_version = version.replace("+", "");
            let compare_jvm_version = get_compare_version(jvm, &sanitised_version);
            let compare = compare_version_values(&compare_jvm_version, &sanitised_version);
            if compare.is_lt() {
                return false;
            }
        } else {
            let compare_jvm_version = get_compare_version(jvm, version);
            let compare = compare_version_values(&version, &compare_jvm_version);
            if compare.is_ne() {
                return false;
            }
        }
    }
    return true;
}

fn compare_version_values(version1: &String, version2: &String) -> Ordering {
    // Normalise old style versions - e.g. 1.8 -> 8, 1.9 -> 9
    let mut normalised1= version1.strip_prefix("1.")
        .unwrap_or(version1.as_str()).to_string();
    let mut normalised2= version2.strip_prefix("1.")
        .unwrap_or(version2.as_str()).to_string();
    // Normalise old sub versions e.g. 1.8.0_292 -> 1.8.0.292
    normalised1 = normalised1.replace("_", ".");
    normalised2 = normalised2.replace("_", ".");

    let count_version1: Vec<String> =
        normalised1.split(".").map(|s| s.to_string()).collect();
    let count_version2: Vec<String> =
        normalised2.split(".").map(|s| s.to_string()).collect();

    let compare = Ordering::Equal;
    for i in 0..count_version1.len() {
        let version1_int = count_version1.get(i).unwrap().parse::<i32>().unwrap();
        let version2_int = count_version2.get(i).unwrap().parse::<i32>().unwrap();
        if version1_int > version2_int {
            return Ordering::Greater
        } else if version1_int < version2_int {
            return Ordering::Less;
        } else {
            continue;
        }
    }
    return compare;
}

fn get_compare_version(jvm: &Jvm, version: &String) -> String {
    let version_count = version.matches('.').count();
    let mut  jvm_version = jvm.version.clone();

    // Normalise single digit compares for old style versions
    if jvm.version.starts_with("1.") && version.matches('.').count() == 0 {
        if !version.starts_with("1.") {
            jvm_version = jvm_version.strip_prefix("1.")
                .unwrap_or(jvm_version.as_str()).to_string();
        }
    }

    let tmp_version: Vec<String> =
        jvm_version.split_inclusive(".").map(|s| s.to_string()).collect();
    let mut compare_version: String = String::new();
    for i in 0..version_count + 1 {
        compare_version.push_str(tmp_version.get(i).unwrap_or(&"".to_string()));
    }
    compare_version = compare_version.strip_suffix(".")
        .unwrap_or(compare_version.as_str()).to_string();
    compare_version
}

fn filter_arch(arch: &Option<String>, jvm: &Jvm) -> bool {
    if !arch.is_none() {
        if jvm.architecture != arch.as_ref().unwrap().to_string() {
            return false;
        }
    }
    return true;
}

fn filter_name(name: &Option<String>, jvm: &Jvm) -> bool {
    if !name.is_none() {
        if jvm.name != name.as_ref().unwrap().to_string() {
            return false;
        }
    }
    return true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_name() {
        let jvm = create_jvm("17.0.2",
                             "Eclipse Temurin 17",
                             "aarch64",
                             "/Library/Java/JavaVirtualMachines/temurin-17.jdk");
        let same_name: Option<String> = Option::Some("Eclipse Temurin 17".to_string());
        let different_name: Option<String> = Option::Some("Eclipse Temurin 11".to_string());
        assert_eq!(filter_name(&same_name, &jvm), true);
        assert_eq!(filter_name(&different_name, &jvm), false);
    }

    #[test]
    fn test_filter_arch() {
        let jvm = create_jvm("17.0.2",
                             "Eclipse Temurin 17",
                             "aarch64",
                             "/Library/Java/JavaVirtualMachines/temurin-17.jdk");
        let same_arch: Option<String> = Option::Some("aarch64".to_string());
        let different_arch: Option<String> = Option::Some("x86_64".to_string());
        assert_eq!(filter_arch(&same_arch, &jvm), true);
        assert_eq!(filter_arch(&different_arch, &jvm), false);
    }

    #[test]
    fn test_filter_version() {
        let jvm = create_jvm("17.0.2",
                             "Eclipse Temurin 17",
                             "aarch64",
                             "/Library/Java/JavaVirtualMachines/temurin-17.jdk");
        let same_ver: Option<String> = Option::Some("17".to_string());
        let different_ver_same_format: Option<String> = Option::Some("11".to_string());
        let different_ver_diff_format: Option<String> = Option::Some("11.0.2".to_string());
        let different_ver_diff_format2: Option<String> = Option::Some("11.0.2.1".to_string());
        assert_eq!(filter_ver(&same_ver, &jvm), true);
        assert_eq!(filter_ver(&different_ver_same_format, &jvm), false);
        assert_eq!(filter_ver(&different_ver_diff_format, &jvm), false);
        assert_eq!(filter_ver(&different_ver_diff_format2, &jvm), false);
    }

    #[test]
    fn test_compare_version() {
        let jvm = create_jvm("17.0.2",
                             "Eclipse Temurin 17",
                             "aarch64",
                             "/Library/Java/JavaVirtualMachines/temurin-17.jdk");
        assert_eq!(get_compare_version(&jvm, &"8+".to_string()), "17");
        assert_eq!(get_compare_version(&jvm, &"17".to_string()), "17");
        assert_eq!(get_compare_version(&jvm, &"17.1".to_string()), "17.0");
        assert_eq!(get_compare_version(&jvm, &"17.0.1".to_string()), "17.0.2");
        assert_eq!(get_compare_version(&jvm, &"17.0.1.1".to_string()), "17.0.2");
        assert_eq!(get_compare_version(&jvm, &"17.0.1_bau".to_string()), "17.0.2");
        let jvm2 = create_jvm("1.8.0",
                             "AdoptOpenJDK 8",
                             "aarch64",
                             "/Library/Java/JavaVirtualMachines/adoptopenjdk-1.8.0.jdk");
        assert_eq!(get_compare_version(&jvm2, &"8".to_string()), "8");

    }

    #[test]
    fn test_compare_version_values(){
        assert_eq!(compare_version_values(&"17.0.1".to_string(), &"17.0.1".to_string()), Ordering::Equal);
        assert_eq!(compare_version_values(&"8.0.1".to_string(), &"17.0.1".to_string()), Ordering::Less);
        assert_eq!(compare_version_values(&"8.1.1".to_string(), &"8.0.1".to_string()), Ordering::Greater);
        assert_eq!(compare_version_values(&"17".to_string(), &"17".to_string()), Ordering::Equal);
        assert_eq!(compare_version_values(&"17".to_string(), &"11".to_string()), Ordering::Greater);
        assert_eq!(compare_version_values(&"1.8".to_string(), &"8".to_string()), Ordering::Equal);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_trim_string(){
        assert_eq!(trim_string("Arm\n"), "Arm");
        assert_eq!(trim_string("Arm\r\n"), "Arm");
        assert_eq!(trim_string("Arm"), "Arm");
    }

    #[test]
    fn test_compare_version_architecture(){
        let jvm1: Jvm = create_jvm("11.0.2",
                                   "Eclipse Temurin 11",
                                   "aarch64",
                                   "/Library/Java/JavaVirtualMachines/temurin-11-aarch64.jdk");

        let jvms: Vec<Jvm> = vec![jvm1.clone()];
        check_version(jvms.clone(), "11+", 1);
        check_version(jvms.clone(), "11.0+", 1);
        check_version(jvms.clone(), "11.0.1+", 1);
        check_version(jvms.clone(), "11.1+", 0);
        check_version(jvms.clone(), "11.0.3+", 0);
        check_version(jvms.clone(), "17+", 0);
    }

    fn check_version(jvms: Vec<Jvm>, version: &str, number: usize) {
        let result: &Vec<Jvm> = &jvms.into_iter()
            .filter(|tmp| filter_ver(&Option::Some(version.to_string()), tmp))
            .collect();
        assert_eq!(result.len(), number);
    }

    #[test]
    fn test_compare_boosting_architecture(){
        let jvm1: Jvm = create_jvm("11.0.2",
                                   "Eclipse Temurin 11",
                                   "aarch64",
                                   "/Library/Java/JavaVirtualMachines/temurin-11-aarch64.jdk");
        let jvm2: Jvm = create_jvm("11.0.2",
                                   "Eclipse Temurin 11",
                                   "x86_64",
                                   "/Library/Java/JavaVirtualMachines/temurin-11-x86_64.jdk");
        let jvm3: Jvm = create_jvm("17.0.1",
                                   "Eclipse Temurin 17",
                                   "x86_64",
                                   "/Library/Java/JavaVirtualMachines/temurin-17-x86_64.jdk");
        let jvm4: Jvm = create_jvm("8",
                                   "Adopt OpenJDK 8",
                                   "x86_64",
                                   "/Library/Java/JavaVirtualMachines/java-8-openjdk-amd64");

        let gold_ordered_aarch64 :Vec<Jvm> = vec![jvm3.clone(), jvm1.clone(), jvm2.clone(), jvm4.clone()];
        let gold_ordered_x86_64 :Vec<Jvm> = vec![jvm3.clone(), jvm2.clone(), jvm1.clone(), jvm4.clone()];
        let mut jvms :Vec<Jvm> = vec![jvm1.clone(), jvm2.clone(), jvm3.clone(), jvm4.clone()];

        jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &"aarch64".to_string()));
        assert_eq!(jvm_vec_compare(gold_ordered_aarch64, &jvms), true);
        jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &"x86_64".to_string()));
        assert_eq!(jvm_vec_compare(gold_ordered_x86_64, &jvms), true);
    }

    fn create_jvm(version: &str, name: &str, architecture: &str, path: &str) -> Jvm {
        return Jvm {
            version: version.to_string(),
            name: name.to_string(),
            architecture: architecture.to_string(),
            path: path.to_string()
        };
    }

    fn jvm_vec_compare(va: Vec<Jvm>, vb: &Vec<Jvm>) -> bool {
        (va.len() == vb.len()) &&
            va.iter()
                .zip(vb)
                .all(|(a,b)| a.architecture == b.architecture
                    && a.version == b.version
                    && a.name == b.name
                    && a.path == b.path)
    }

}
