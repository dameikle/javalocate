use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::process::{Command, Stdio};
use clap::Parser;
use java_properties::read;
use plist::Value;


/// Command line utility to find JVM versions on macOS and Linux
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
    fail: bool
}

#[derive(Clone, Debug)]
struct Jvm {
    version: String,
    name: String,
    architecture: String,
    path: String
}

#[derive(Clone, Debug)]
struct OperatingSystem {
    name: String,
    family: String,
    architecture: String
}

fn main() {
    let args = Args::parse();

    // Fetch default java architecture based on kernel
    let operating_system = get_operating_system();

    // Build and filter JVMs
    let jvms: Vec<Jvm> = collate_jvms(&operating_system)
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
        family: os.to_string(),
        architecture: default_architecture
    }
}

fn trim_string(value: &str) -> &str {
    value.strip_suffix("\r\n")
        .or(value.strip_suffix("\n"))
        .unwrap_or(value)
}

fn collate_jvms(os: &OperatingSystem) -> Vec<Jvm> {
    return if os.family.eq_ignore_ascii_case("Darwin") {
        collate_jvms_mac(os)
    } else {
        collate_jvms_linux(os)
    }
}

fn collate_jvms_linux(os: &OperatingSystem) -> Vec<Jvm> {
    let mut jvms = Vec::new();
    let dir_lookup = HashMap::from(
        [("ubuntu".to_string(), "/usr/lib/jvm".to_string()),
            ("debian".to_string(), "/usr/lib/jvm".to_string()),
            ("rhel".to_string(), "/usr/lib/jvm".to_string()),
            ("centos".to_string(), "/usr/lib/jvm".to_string()),
            ("fedora".to_string(), "/usr/lib/jvm".to_string())]);

    let path = dir_lookup.get(os.name.as_str());
    if path.is_none() {
        eprintln!("Default JVM path is unknown on {} Linux", os.name);
        std::process::exit(exitcode::UNAVAILABLE);
    }

    for path in fs::read_dir(path.unwrap()).unwrap() {
        let path = path.unwrap().path();
        let metadata = fs::metadata(&path).unwrap();
        let link = fs::read_link(&path);

        if metadata.is_dir() && link.is_err() {
            // Attempt to load the Release file into HashMap
            let release_file = File::open(path.join("release"));
            let release_file = match release_file {
                Ok(release_file) => release_file,
                Err(_error) => continue,
            };

            // Collate required information
            let properties = read(BufReader::new(release_file)).unwrap();
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
        }
    }
    jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return jvms;
}

fn collate_jvms_mac(os: &OperatingSystem) -> Vec<Jvm> {
    let mut jvms = Vec::new();
    for path in fs::read_dir("/Library/Java/JavaVirtualMachines").unwrap() {
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
                path: path.to_str().unwrap().to_string(),
            };
            jvms.push(tmp_jvm);
        }
    }
    jvms.sort_by(|a, b| compare_boosting_architecture(a, b, &os.architecture));
    return jvms;
}

fn compare_boosting_architecture(a: &Jvm, b: &Jvm, default_arch: &String) -> Ordering {
    let version_test = b.version.partial_cmp(&a.version).unwrap_or(Ordering::Equal);
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
            if jvm.version < version.replace("+", "") {
                return false;
            }
        } else {
            let compare_version = get_compare_version(jvm, version);
            // Handle single unit comparison against older version numbers
            if compare_version == "1" {
                return false;
            }
            // Perform comparison
            if version != compare_version.as_str() {
                return false;
            }
        }
    }
    return true;
}

fn get_compare_version(jvm: &Jvm, version: &String) -> String {
    let version_count = version.matches('.').count();
    let tmp_version: Vec<String> =
        jvm.version.split_inclusive(".").map(|s| s.to_string()).collect();
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
        assert_eq!(get_compare_version(&jvm, &"17".to_string()), "17");
        assert_eq!(get_compare_version(&jvm, &"17.1".to_string()), "17.0");
        assert_eq!(get_compare_version(&jvm, &"17.0.1".to_string()), "17.0.2");
        assert_eq!(get_compare_version(&jvm, &"17.0.1.1".to_string()), "17.0.2");
    }

    #[test]
    fn test_trim_string(){
        assert_eq!(trim_string("Arm\n"), "Arm");
        assert_eq!(trim_string("Arm\r\n"), "Arm");
        assert_eq!(trim_string("Arm"), "Arm");
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

        let gold_ordered_aarch64 :Vec<Jvm> = vec![jvm3.clone(), jvm1.clone(), jvm2.clone()];
        let gold_ordered_x86_64 :Vec<Jvm> = vec![jvm3.clone(), jvm2.clone(), jvm1.clone()];
        let mut jvms :Vec<Jvm> = vec![jvm1.clone(), jvm2.clone(), jvm3.clone()];

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
