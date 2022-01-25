use std::{
    ffi::OsStr,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use assert_cmd::Command;
use insta::assert_snapshot;
use tempfile::{tempdir, TempDir};

struct TestContext {
    command: Command,
    home_directory: TempDir,
    home_directory_display: String,
    working_directory: TempDir,
    working_directory_display: String,
}

impl TestContext {
    fn foreman() -> Self {
        let home_directory = tempdir().expect("unable to create temporary directory");
        let working_directory = tempdir().expect("unable to create temporary directory");

        let mut command = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        command.env("FOREMAN_HOME", home_directory.path().as_os_str());
        command.current_dir(working_directory.path());

        TestContext::new(command, home_directory, working_directory)
    }

    fn new(command: Command, home_directory: TempDir, working_directory: TempDir) -> Self {
        let home_directory_display = format!(
            "{}{}",
            home_directory.path().display(),
            std::path::MAIN_SEPARATOR
        );
        let working_directory_display = format!(
            "{}{}",
            working_directory.path().display(),
            std::path::MAIN_SEPARATOR
        );
        Self {
            command,
            home_directory,
            home_directory_display,
            working_directory,
            working_directory_display,
        }
    }

    fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.command.arg(arg);
        self
    }

    fn output(&mut self) -> String {
        let output = self.command.output().expect("unable to run command");
        let mut string = std::str::from_utf8(&output.stdout)
            .expect("unable to read output")
            .to_owned();
        if !string.is_empty() {
            string.push('\n');
        }
        string.push_str(std::str::from_utf8(&output.stderr).expect("unable to read output"));
        string
    }

    fn expect_success(mut self) -> Self {
        self.command.assert().success();
        self
    }

    fn path_from_home<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut root = self.home_directory.path().to_owned();
        root.push(path);
        root
    }

    fn path_from_working_directory<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut root = self.working_directory.path().to_owned();
        root.push(path);
        root
    }

    fn snapshot_command(&mut self, snapshot_name: &'static str) {
        let content = self.output();
        self.snapshot_string(snapshot_name, content);
    }

    fn snapshot_file<P: AsRef<Path>>(&self, snapshot_name: &'static str, file_path: P) {
        let content = read_to_string(file_path).expect("unable to read file");
        self.snapshot_string(snapshot_name, content);
    }

    fn snapshot_string(&self, snapshot_name: &'static str, content: String) {
        let content = content
            .replace(&self.home_directory_display, "{{FOREMAN_HOME}}")
            .replace(&self.working_directory_display, "{{CWD}}")
            .replace("foreman.exe", "foreman");
        insta::with_settings!({prepend_module_to_snapshot => false}, {
            assert_snapshot!(snapshot_name, content);
        });
    }
}

fn write_file(path: &Path, content: &str) {
    std::fs::write(path, content).expect("unable to write file");
}

#[test]
fn snapshot_help_command() {
    TestContext::foreman()
        .arg("help")
        .snapshot_command("help_command");
}

#[test]
fn snapshot_install_no_tools_found() {
    TestContext::foreman()
        .arg("install")
        .snapshot_command("install_no_tools");
}

#[test]
fn snapshot_install_default_foreman_toml() {
    let context = TestContext::foreman().arg("install").expect_success();
    context.snapshot_file(
        "default_foreman_toml",
        context.path_from_home("foreman.toml"),
    );
}

#[test]
fn snapshot_install_default_auth_toml() {
    let context = TestContext::foreman().arg("install").expect_success();
    context.snapshot_file("default_auth_toml", context.path_from_home("auth.toml"));
}

#[test]
fn snapshot_install_empty_configuration_file() {
    let mut context = TestContext::foreman().arg("install");
    let config_path = context.path_from_working_directory("foreman.toml");
    write_file(&config_path, "");
    context.snapshot_command("install_empty_config_file");
}

#[test]
fn snapshot_install_invalid_tool_configuration() {
    let mut context = TestContext::foreman().arg("install");
    let config_path = context.path_from_working_directory("foreman.toml");
    write_file(
        &config_path,
        r#"
[tools]
tool = { invalid = "roblox/tooling", version = "0.0.0" }
        "#,
    );
    context.snapshot_command("install_invalid_tool_configuration");
}

#[test]
fn snapshot_install_invalid_system_configuration_file() {
    let mut context = TestContext::foreman().arg("install");
    let config_path = context.path_from_home("foreman.toml");
    write_file(&config_path, "invalid");
    context.snapshot_command("install_invalid_system_configuration");
}

#[test]
fn snapshot_install_invalid_auth_toml() {
    let mut context = TestContext::foreman().arg("install");
    let auth_path = context.path_from_home("auth.toml");
    write_file(&auth_path, "invalid");
    let config_path = context.path_from_working_directory("foreman.toml");
    write_file(
        &config_path,
        r#"
[tools]
stylua = { github = "JohnnyMorganz/StyLua", version = "0.11.3" }
    "#,
    );
    context.snapshot_command("install_invalid_auth_configuration");
}
