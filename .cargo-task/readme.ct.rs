/*
@ct-help@ Generate a README.md from our rust lib.rs docs. @@
*/

use cargo_task_util::*;
use std::process::Stdio;

fn readme_ok(env: &CTEnv) -> bool {
    let mut test = env.cargo();
    test
        .arg("help")
        .arg("readme")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match test.status() {
        Ok(e) => e.success(),
        Err(_) => false,
    }
}

fn install_readme_cargo(env: &CTEnv) {
    let mut cargo = env.cargo();
    cargo
        .arg("install")
        .arg("cargo-readme");
    ct_check_fatal!(env.exec(cargo));
}

fn main() {
    let env = ct_env();

    // see if clippy is installed
    if !readme_ok(&env) {
        install_readme_cargo(&env);
    }

    let mut cmd = env.cargo();
    cmd
        .arg("readme")
        .arg("--output")
        .arg("README.md");
    ct_check_fatal!(env.exec(cmd));

    if std::env::var_os("CI").is_some() {
        let mut cmd = std::process::Command::new("git");
        cmd
            .arg("diff")
            .arg("--exit-code");
        ct_check_fatal!(env.exec(cmd));
    }
}
