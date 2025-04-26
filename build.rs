use std::env;
use std::fs;
use std::path::Path;

fn main() {
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
    
    // The target directory already includes the profile (debug/release)
    // So we don't need to add it again
    
    // Copy the tools directory
    copy_directory("tools", &target_dir.join("tools"));
    
    // Print info for debugging
    println!("cargo:warning=Copied tools directory to {:?}", target_dir.join("tools"));
    
    // Tell Cargo to re-run this build script if the tools directory changes
    println!("cargo:rerun-if-changed=tools");
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
            fs::copy(&src_path, &dst_path)
                .expect(&format!("Failed to copy file from {:?} to {:?}", src_path, dst_path));
        }
    }
}
