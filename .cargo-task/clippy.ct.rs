/*
@ct-help@ Run "cargo clippy" to check for lint. @@
*/

use cargo_task_util::*;
use std::process::Stdio;

// is clippy installed?
fn clippy_ok(env: &CTEnv) -> bool {
    let mut test = env.cargo();
    test
        .arg("help")
        .arg("clippy")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match test.status() {
        Ok(e) => e.success(),
        Err(_) => false,
    }
}

// attepmt to add through rustup
fn install_clippy_rustup(env: &CTEnv) -> Result<(), ()> {
    let mut ru = std::process::Command::new("rustup");
    ru
        .arg("component")
        .arg("add")
        .arg("clippy");
    env.exec(ru).map_err(|_| ())?;
    Ok(())
}

// fall back to installing with cargo
fn install_clippy_cargo(env: &CTEnv) {
    let mut cargo = env.cargo();
    cargo
        .arg("install")
        .arg("clippy");
    ct_check_fatal!(env.exec(cargo));
}

fn main() {
    // get our cargo-task env
    let env = ct_env();

    // see if clippy is installed
    if !clippy_ok(&env) {
        if install_clippy_rustup(&env).is_err() {
            install_clippy_cargo(&env);
        }
    }

    // exec `cargo clippy`
    let mut cmd = env.cargo();
    cmd.arg("clippy");
    ct_check_fatal!(env.exec(cmd));
}
