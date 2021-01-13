/*
@ct-help@ Publish to crates.io and tag git release. @@

@ct-cargo-deps@
toml = "0.5"
@@
*/

use cargo_task_util::*;

fn main() {
    // get our cargo-task env
    let env = ct_env();

    // make sure there are no local changes
    let mut cmd = std::process::Command::new("git");
    cmd
        .arg("diff")
        .arg("--exit-code");
    ct_check_fatal!(env.exec(cmd));

    // fetch version out of Cargo.toml
    let toml: toml::Value = toml::from_str(&std::fs::read_to_string("Cargo.toml").unwrap()).unwrap();
    let version = toml
        .as_table()
        .unwrap()
        .get("package")
        .unwrap()
        .as_table()
        .unwrap()
        .get("version")
        .unwrap()
        .as_str()
        .unwrap();

    // run 'cargo publish'
    let mut cmd = env.cargo();
    cmd
        .arg("publish");
    ct_check_fatal!(env.exec(cmd));

    // add git tag
    let mut cmd = std::process::Command::new("git");
    cmd
        .arg("tag")
        .arg("-a")
        .arg(version)
        .arg("-m")
        .arg(version);
    ct_check_fatal!(env.exec(cmd));

    // push git tag
    let mut cmd = std::process::Command::new("git");
    cmd
        .arg("push")
        .arg("--tags");
    ct_check_fatal!(env.exec(cmd));
}
