use std::cmp::Ordering::Equal;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use clap::Parser;
use java_properties::read;
use plist::Value;


/// Command line utility to find JVM versions on macOS
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
}

struct Jvm {
    version: String,
    name: String,
    architecture: String,
    path: String,
}

fn main() {
    let args = Args::parse();

    // Build and filter JVMs
    let jvms: Vec<Jvm> = collate_jvms()
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

    // If JVMS found, display
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

fn collate_jvms() -> Vec<Jvm> {
    let mut jvms = Vec::new();
    let paths = fs::read_dir("/Library/Java/JavaVirtualMachines").unwrap();
    for path in paths {
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
    jvms.sort_by(|a, b| b.version.partial_cmp(&a.version).unwrap_or(Equal));
    return jvms;
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
        let jvm = Jvm {
            version: "17.0.2".to_string(),
            name: "Eclipse Temurin 17".to_string(),
            architecture: "aarch64".to_string(),
            path: "/Library/Java/JavaVirtualMachines/temurin-17.jdk".to_string()
        };
        let same_name: Option<String> = Option::Some("Eclipse Temurin 17".to_string());
        let different_name: Option<String> = Option::Some("Eclipse Temurin 11".to_string());
        assert_eq!(filter_name(&same_name, &jvm), true);
        assert_eq!(filter_name(&different_name, &jvm), false);
    }

    #[test]
    fn test_filter_arch() {
        let jvm = Jvm {
            version: "17.0.2".to_string(),
            name: "Eclipse Temurin 17".to_string(),
            architecture: "aarch64".to_string(),
            path: "/Library/Java/JavaVirtualMachines/temurin-17.jdk".to_string()
        };
        let same_arch: Option<String> = Option::Some("aarch64".to_string());
        let different_arch: Option<String> = Option::Some("x86_64".to_string());
        assert_eq!(filter_arch(&same_arch, &jvm), true);
        assert_eq!(filter_arch(&different_arch, &jvm), false);
    }

    #[test]
    fn test_filter_version() {
        let jvm = Jvm {
            version: "17.0.2".to_string(),
            name: "Eclipse Temurin 17".to_string(),
            architecture: "aarch64".to_string(),
            path: "/Library/Java/JavaVirtualMachines/temurin-17.jdk".to_string()
        };
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
        let jvm = Jvm {
            version: "17.0.2".to_string(),
            name: "Eclipse Temurin 17".to_string(),
            architecture: "aarch64".to_string(),
            path: "/Library/Java/JavaVirtualMachines/temurin-17.jdk".to_string()
        };
        assert_eq!(get_compare_version(&jvm, &"17".to_string()), "17");
        assert_eq!(get_compare_version(&jvm, &"17.1".to_string()), "17.0");
        assert_eq!(get_compare_version(&jvm, &"17.0.1".to_string()), "17.0.2");
        assert_eq!(get_compare_version(&jvm, &"17.0.1.1".to_string()), "17.0.2");
    }

}
