use std::process::Command;
use std::path::Path;
use std::fs;
use std::collections::HashMap;

fn run_configure(icu_src: &Path, build_dir: &Path, target: &str, cross_build: Option<&Path>) {
    let icu_configure = icu_src.join("runConfigureICU");
    if !icu_configure.exists() {
        panic!("Cannot find runConfigureICU at {}", icu_configure.display());
    }

    let mut configure = Command::new("sh");
    configure.arg(&icu_configure)
             .current_dir(build_dir);

    match target {
        // macOS builds
        "host" | "x86_64-macos" | "aarch64-macos" => {
            configure.arg("MacOSX");
            if target != "host" {
                let arch = if target == "x86_64-macos" { "x86_64" } else { "arm64" };
                configure.env("CFLAGS", format!("-arch {} -O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1", arch))
                         .env("CXXFLAGS", format!("-arch {} -O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1", arch))
                         .env("LDFLAGS", format!("-arch {}", arch));
            }
        }

        // OHOS cross-build
        "aarch64-ohos" => {
            let ohos_sdk = std::env::var("OHOS_SDK")
                .unwrap_or_else(|_| "/Applications/DevEco-Studio.app/Contents/sdk".to_string());
            let llvm_bin = format!("{}/default/openharmony/native/llvm/bin", ohos_sdk);
            let sysroot = format!("{}/default/openharmony/native/sysroot", ohos_sdk);

            let cc = format!("{}/clang --target=aarch64-linux-ohos --sysroot={}", llvm_bin, sysroot);
            let cxx = format!("{}/clang++ --target=aarch64-linux-ohos --sysroot={}", llvm_bin, sysroot);

            configure.arg("Linux")
                     .arg("--host=aarch64-linux-ohos")
                     .arg(format!("--with-cross-build={}", cross_build.unwrap().display()))
                     .env("CC", cc)
                     .env("CXX", cxx)
                     .env("CFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("CXXFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("LDFLAGS", "-fPIC");
        }

        // Android cross-builds
        "x86_64-android" | "aarch64-android" | "armv7-android" | "x86-android" => {
            let ndk = std::env::var("ANDROID_NDK_HOME")
                .expect("ANDROID_NDK_HOME must be set to a valid Android NDK path");
            let llvm_bin = format!("{}/toolchains/llvm/prebuilt/darwin-x86_64/bin", ndk);

            let (triple, api) = match target {
                "x86_64-android" => ("x86_64-linux-android", "21"),
                "aarch64-android" => ("aarch64-linux-android", "21"),
                "armv7-android"   => ("armv7a-linux-androideabi", "21"),
                "x86-android"     => ("i686-linux-android", "21"),
                _ => unreachable!(),
            };

            let cc = format!("{}/{}{}-clang", llvm_bin, triple, api);
            let cxx = format!("{}/{}{}-clang++", llvm_bin, triple, api);

            configure.arg("Linux")
                     .arg(format!("--host={}", triple))
                     .arg(format!("--with-cross-build={}", cross_build.unwrap().display()))
                     .arg("--with-data-packaging=archive")
                     .env("CC", cc)
                     .env("CXX", cxx)
                     .env("CFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("CXXFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("LDFLAGS", "-fPIC");
        }

        // Linux cross-builds (zig)
        _ => {
            let host_triple_map: HashMap<_, _> = [
                ("x86_64-linux", "x86_64-linux-gnu"),
                ("amd64-linux",  "x86_64-linux-gnu"),
                ("aarch64-linux", "aarch64-linux-gnu"),
            ].into_iter().collect();

            let zig_cc_map: HashMap<_, _> = [
                ("x86_64-linux", "zig cc -target x86_64-linux-gnu"),
                ("amd64-linux",  "zig cc -target x86_64-linux-gnu"),
                ("aarch64-linux", "zig cc -target aarch64-linux-gnu"),
            ].into_iter().collect();

            let zig_cxx_map: HashMap<_, _> = [
                ("x86_64-linux", "zig c++ -target x86_64-linux-gnu"),
                ("amd64-linux",  "zig c++ -target x86_64-linux-gnu"),
                ("aarch64-linux", "zig c++ -target aarch64-linux-gnu"),
            ].into_iter().collect();

            if !host_triple_map.contains_key(target) {
                panic!("Unsupported target: {}", target);
            }

            configure.arg("Linux")
                     .arg(format!("--host={}", host_triple_map[target]))
                     .arg(format!("--with-cross-build={}", cross_build.unwrap().display()))
                     .env("CC", zig_cc_map[target])
                     .env("CXX", zig_cxx_map[target])
                     .env("CFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("CXXFLAGS", "-O2 -fPIC -DUCONFIG_ONLY_COLLATION=1 -DUCONFIG_NO_LEGACY_CONVERSION=1")
                     .env("LDFLAGS", "-fPIC");
        }
    }

    configure.arg("--enable-static")
             .arg("--disable-shared");

    let status = configure.status().expect("Failed to run configure");
    assert!(status.success(), "Configure failed");
}

fn run_make(build_dir: &Path) {
    let status = Command::new("make")
        .current_dir(build_dir)
        .arg("-j8")
        .status()
        .expect("Failed to run make");
    assert!(status.success(), "Make failed");
}

fn copy_libs(build_dir: &Path, target_out: &Path) {
    fs::create_dir_all(target_out).unwrap();
    let lib_dir = build_dir.join("lib");
    for lib in &["libicuuc.a", "libicui18n.a", "libicudata.a"] {
        fs::copy(lib_dir.join(lib), target_out.join(lib)).unwrap();
    }
}


fn extract_and_prepare(crate_dir: &Path) {
    let download_dir = crate_dir.join("download");
    let icu_src_tar = download_dir.join("icu4c-77_1-src.tgz");
    let icu_dir = crate_dir.join("icu");

    // remove old icu directory if it exists
    if icu_dir.exists() {
        println!("Removing old ICU source tree...");
        fs::remove_dir_all(&icu_dir).unwrap();
    }
    fs::create_dir_all(&icu_dir).unwrap(); // ✅ make sure it exists

    println!("Extracting ICU source...");
    let status = Command::new("tar")
        .arg("-xzvf")
        .arg(&icu_src_tar)
        .arg("--strip-components=1")
        .arg("-C")
        .arg(&icu_dir)
        .status()
        .expect("Failed to extract ICU tarball");
    assert!(status.success(), "ICU extraction failed");

    // copy icudt77l.dat into icu/source/data/in
    let data_file = download_dir.join("icudt77l.dat");
    let data_in_dir = icu_dir.join("source/data/in");
    fs::create_dir_all(&data_in_dir).unwrap();
    fs::copy(&data_file, data_in_dir.join("icudt77l.dat"))
        .expect("Failed to copy icudt77l.dat");
    println!("Copied icudt77l.dat to {}", data_in_dir.display());
}


fn main() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    extract_and_prepare(&crate_dir);

    let icu_src = crate_dir.join("icu/source");

    // 1️⃣ Build host ICU (current macOS arch)
    let build_host = icu_src.join("build-host");
    fs::create_dir_all(&build_host).unwrap();
    println!("Building host ICU (macOS current arch)...");
    run_configure(&icu_src, &build_host, "host", None);
    run_make(&build_host);

    // 2️⃣ Cross-build macOS, Linux, OHOS, and Android targets
    let targets = [
        "x86_64-macos",
        "aarch64-macos",
        "x86_64-linux",
        "amd64-linux",
        "aarch64-linux",
        "aarch64-ohos",
        "armv7-android",
        "aarch64-android",
        "x86-android",
        "x86_64-android",
    ];

    for target in &targets {
        let build_target = icu_src.join(format!("build-{}", target));
        fs::create_dir_all(&build_target).unwrap();

        println!("Building ICU for {}...", target);
        run_configure(&icu_src, &build_target, target, Some(&build_host));
        run_make(&build_target);

        let out_dir = if target.ends_with("macos") {
            let arch = if target.starts_with("x86_64") { "x86_64" } else { "aarch64" };
            crate_dir.join("libs/osx").join(arch)
        } else if target.ends_with("ohos") {
            crate_dir.join("libs/ohos").join("aarch64")
        } else if target.ends_with("android") {
            // Copy libicudata to lib (no data build on android)
            let lib_dir = build_target.join("lib");
            let data_dir = build_target.join("stubdata");
            let data_file = "libicudata.a";
            fs::copy(data_dir.join(data_file), lib_dir.join(data_file)).unwrap();

            let abi = match *target {
                "armv7-android"   => "armeabi-v7a",
                "aarch64-android" => "arm64-v8a",
                "x86-android"     => "x86",
                "x86_64-android"  => "x86_64",
                _ => panic!("Unknown Android target {}", target),
            };
            crate_dir.join("libs/android").join(abi)
        } else { // Linux targets
            let arch = target.strip_suffix("-linux").unwrap_or(target);
            crate_dir.join("libs/linux").join(arch)
        };

        copy_libs(&build_target, &out_dir);
        println!("Copied ICU libraries to {}", out_dir.display());
    }
}
