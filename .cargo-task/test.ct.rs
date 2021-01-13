/*
@ct-help@ Run "cargo test". @@
*/

use cargo_task_util::*;

fn main() {
    // get our cargo-task env
    let env = ct_env();

    // exec `cargo test --all-features`
    ct_info!("run 'cargo test --all-features'");
    let mut cmd = env.cargo();
    cmd
        .arg("test")
        .arg("--all-features");
    ct_check_fatal!(env.exec(cmd));
}
