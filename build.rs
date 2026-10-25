// Build script: Cargo runs this before compiling the app (like a Gradle task).
// It embeds the icon, the Windows manifest and the version info into the exe.
fn main() {
    println!("cargo:rerun-if-changed=res");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("res/sound_hop.rc", embed_resource::NONE).manifest_required().unwrap();
    }
}
