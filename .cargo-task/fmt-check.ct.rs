/*
@ct-help@ Check cargo fmt lint/style. @@
*/

use cargo_task_util::*;

use std::process::Stdio;

// is cargo fmt (rustfmt) installed?
fn fmt_ok(env: &CTEnv) -> bool {
    let mut test = env.cargo();
    test
        .arg("help")
        .arg("fmt")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match test.status() {
        Ok(e) => e.success(),
        Err(_) => false,
    }
}

// attempt to add through rustup
fn install_fmt_rustup(env: &CTEnv) -> Result<(), ()> {
    let mut ru = std::process::Command::new("rustup");
    ru
        .arg("component")
        .arg("add")
        .arg("rustfmt");
    env.exec(ru).map_err(|_| ())?;
    Ok(())
}

// fall back to installing with cargo
fn install_fmt_cargo(env: &CTEnv) {
    let mut cargo = env.cargo();
    cargo
        .arg("install")
        .arg("rustfmt");
    ct_check_fatal!(env.exec(cargo));
}

fn main() {
    // get our cargo-task env
    let env = ct_env();

    // see if fmt is installed
    if !fmt_ok(&env) {
        if install_fmt_rustup(&env).is_err() {
            install_fmt_cargo(&env);
        }
    }

    // exec `cargo fmt -- --check`
    let mut cmd = env.cargo();
    cmd
        .arg("fmt")
        .arg("--")
        .arg("--check");
    ct_check_fatal!(env.exec(cmd));
}
