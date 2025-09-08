use std::path::Path;
use std::process::Command;

fn change_extension(filename: &str, new_extension: &str) -> String {
    let path = Path::new(filename);
    let stem = path.file_stem().unwrap_or_default();
    format!("{}_x.{}", stem.to_string_lossy(), new_extension)
}

fn main() {
    let x_files = ["types.x", "scp.x", "scp_handle.x"];

    let x_dir = "x";
    let out_dir = "src";

    std::fs::create_dir_all(&out_dir).expect("failed to create output directory");

    for x_file in &x_files {
        let x_path = format!("{}/{}", x_dir, x_file);
        let rs_path = format!("{}/{}", out_dir, change_extension(x_file, "rs"));

        println!("cargo:rerun-if-changed={}", x_path);

        let output = Command::new("x23c")
            .args(&["--rust", "-o", rs_path.as_str(), x_path.as_str()])
            .output()
            .expect(&format!("failed to run x23c on {}", x_path));

        if !output.status.success() {
            panic!(
                "x23c failed for {}:\n{}",
                x_path,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
