use std::cmp::Ordering::Equal;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use clap::Parser;
use java_properties::read;
use plist::Value;


/// Command line utility to find JVMs
#[derive(Parser, Debug)]
#[clap(author, about, long_about = None)]
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
    release: f32,
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
            println!("{} ({}) \"{}\" - {} ({})",
                     jvm.version,
                     jvm.architecture,
                     jvm.name,
                     jvm.path,
                     jvm.release
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
            let release = process_version(&version).parse::<f32>().unwrap();

            // Build JVM Struct
            let tmp_jvm = Jvm {
                version,
                release,
                architecture,
                name,
                path: path.to_str().unwrap().to_string(),
            };
            jvms.push(tmp_jvm);
        }
    }
    jvms.sort_by(|a, b| b.release.partial_cmp(&a.release).unwrap_or(Equal));
    return jvms;
}

fn process_version(version: &String) -> String {
    let parts: Vec<String> = version.split(".").map(|s| s.to_string()).collect();
    let mut release_no: String = parts.get(0).unwrap().to_string();
    if parts.get(0).unwrap() == "1" {
        release_no.push_str(".");
        release_no.push_str(parts.get(1).unwrap());
    }
    release_no
}

fn filter_ver(ver: &Option<String>, jvm: &Jvm) -> bool {
    if !ver.is_none() {
        if ver.as_ref().unwrap().contains("+") {
            if jvm.release < ver.as_ref().unwrap().replace("+", "").parse::<f32>().unwrap() {
                return false;
            }
        } else {
            if jvm.release != ver.as_ref().unwrap().parse::<f32>().unwrap()  {
                return false;
            }
        }
    }
    return true;
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


