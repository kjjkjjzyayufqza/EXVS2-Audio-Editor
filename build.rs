use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Embed Windows EXE icon (Explorer / zip / shortcut).
    // Taskbar while running uses ViewportBuilder::with_icon in main.rs separately.
    // Use CARGO_CFG_TARGET_OS so this tracks the *target*, not only the host.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let icon_path = Path::new(&manifest_dir).join("assets").join("favicon.ico");
        if !icon_path.is_file() {
            panic!(
                "Windows icon file not found: {} (required for EXE icon)",
                icon_path.display()
            );
        }

        // winres/rc.exe is most reliable with classic BMP-based .ico (not PNG-compressed ICO).
        let mut res = winres::WindowsResource::new();
        // Prefer forward slashes in generated .rc paths
        let icon_str = icon_path.to_string_lossy().replace('\\', "/");
        res.set_icon(&icon_str);
        res.compile()
            .unwrap_or_else(|e| panic!("Failed to embed Windows icon resource: {e}"));
        println!("cargo:warning=Embedded Windows EXE icon from {icon_str}");
    }

    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let _profile = env::var("PROFILE").unwrap();

    // Construct the target directory path
    // Note: OUT_DIR usually contains something like target/debug/build/project-hash/out
    // We need to go up a few levels to get the actual target directory
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();

    // Copy the tools directory
    copy_directory("tools", &target_dir.join("tools"));

    // Print info for debugging
    println!(
        "cargo:warning=Copied tools directory to {:?}",
        target_dir.join("tools")
    );

    // Tell Cargo to re-run this build script if the tools directory changes
    println!("cargo:rerun-if-changed=tools");
    // Tell Cargo to re-run this build script if icon assets change
    println!("cargo:rerun-if-changed=assets/favicon.ico");
    println!("cargo:rerun-if-changed=assets/icon-256.png");
}

// Function to recursively copy a directory
fn copy_directory<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) {
    let from = from.as_ref();
    let to = to.as_ref();

    // Create the target directory if it doesn't exist
    fs::create_dir_all(to).expect("Failed to create target directory");

    // Read the source directory
    let entries = fs::read_dir(from).expect("Failed to read source directory");

    // Copy each entry
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let file_type = entry.file_type().expect("Failed to get file type");
        let src_path = entry.path();
        let dst_path = to.join(entry.file_name());

        if file_type.is_dir() {
            // Recursively copy subdirectories
            copy_directory(&src_path, &dst_path);
        } else {
            // Copy files
            fs::copy(&src_path, &dst_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to copy file from {:?} to {:?}: {e}",
                    src_path, dst_path
                )
            });
        }
    }
}
