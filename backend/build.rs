// 在编译期记录 rustc 版本，供「关于」页展示后端技术栈。
fn main() {
    let version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "Rust".to_string());
    println!("cargo:rustc-env=RUSTC_VERSION={version}");
    println!("cargo:rerun-if-changed=build.rs");
}
