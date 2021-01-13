/*
@ct-help@ Run "cargo test". @@
*/

use cargo_task_util::*;

fn main() {
    // get our cargo-task env
    let env = ct_env();

    // smoke test a release build
    ct_info!("smoke test 'cargo build --release'");
    let mut cmd = env.cargo();
    cmd
        .arg("build")
        .arg("--release");
    ct_check_fatal!(env.exec(cmd));

    // exec `cargo test --all-features`
    ct_info!("run 'cargo test --all-features'");
    let mut cmd = env.cargo();
    cmd
        .arg("test")
        .arg("--all-features");
    ct_check_fatal!(env.exec(cmd));
}
