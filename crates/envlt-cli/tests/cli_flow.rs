use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cli(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("envlt").expect("binary exists");
    cmd.env("ENVLT_HOME", home.path());
    cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    cmd.env("ENVLT_BUNDLE_PASSPHRASE", "bundle-passphrase");
    cmd
}

fn cli_with_example(home: &TempDir, example_pairs: &[(&str, &str)]) -> Command {
    let mut cmd = cli(home);
    for (key, value) in example_pairs {
        cmd.env(format!("ENVLT_EXAMPLE_{key}"), value);
    }
    cmd
}

#[test]
fn init_add_list_set_use_flow_works() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");
    let output_path = project_dir.path().join(".generated.env");

    fs::write(&env_path, "APP_ENV=dev\nPORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();

    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "api-payments"])
        .assert()
        .success();

    let link = fs::read_to_string(project_dir.path().join(".envlt-link")).expect("project link");
    assert!(link.contains("api-payments"));

    cli(&home)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("api-payments"));

    cli(&home)
        .args(["set", "--project", "api-payments", "PORT=4000"])
        .assert()
        .success();

    cli(&home)
        .args([
            "use",
            "--project",
            "api-payments",
            "--out",
            output_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let generated = fs::read_to_string(output_path).expect("generated env");
    assert!(generated.contains("APP_ENV=dev"));
    assert!(generated.contains("PORT=4000"));
}

#[test]
fn run_injects_variables_into_child_process() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "HELLO=world\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "api-auth"])
        .assert()
        .success();

    cli(&home)
        .current_dir(project_dir.path())
        .args(["run", "sh", "-c", "printf %s \"$HELLO\""])
        .assert()
        .success()
        .stdout("world");
}

#[test]
fn wrong_passphrase_returns_error() {
    let home = TempDir::new().expect("tempdir");

    cli(&home).arg("init").assert().success();

    let mut cmd = Command::cargo_bin("envlt").expect("binary exists");
    cmd.env("ENVLT_HOME", home.path());
    cmd.env("ENVLT_PASSPHRASE", "wrong-passphrase");
    cmd.arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("vault passphrase is invalid"));
}

#[test]
fn set_and_use_can_resolve_project_from_link() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");
    let output_path = project_dir.path().join(".env.local");

    fs::write(&env_path, "MODE=dev\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "linked-project"])
        .assert()
        .success();

    cli(&home)
        .current_dir(project_dir.path())
        .args(["set", "MODE=prod"])
        .assert()
        .success();

    cli(&home)
        .current_dir(project_dir.path())
        .args(["use", "--out", output_path.to_str().expect("utf8 path")])
        .assert()
        .success();

    let generated = fs::read_to_string(output_path).expect("generated env");
    assert!(generated.contains("MODE=prod"));
}

#[test]
fn save_creates_backup_file() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "KEY=one\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "backup-project"])
        .assert()
        .success();

    cli(&home)
        .args(["set", "--project", "backup-project", "KEY=two"])
        .assert()
        .success();

    assert!(home.path().join("vault.age.bak").exists());
}

#[test]
fn export_and_import_roundtrip_bundle() {
    let home_a = TempDir::new().expect("tempdir");
    let home_b = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let bundle_path = project_dir.path().join("bundle.evlt");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "API_URL=https://example.com\nTOKEN=abc123\n").expect("write env");

    cli(&home_a).arg("init").assert().success();
    cli(&home_a)
        .current_dir(project_dir.path())
        .args(["add", "shared-project"])
        .assert()
        .success();

    cli(&home_a)
        .args([
            "export",
            "shared-project",
            "--out",
            bundle_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let mut import_cmd = Command::cargo_bin("envlt").expect("binary exists");
    import_cmd.env("ENVLT_HOME", home_b.path());
    import_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    import_cmd.env("ENVLT_BUNDLE_PASSPHRASE", "bundle-passphrase");

    import_cmd.arg("init").assert().success();

    let mut import_bundle_cmd = Command::cargo_bin("envlt").expect("binary exists");
    import_bundle_cmd.env("ENVLT_HOME", home_b.path());
    import_bundle_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    import_bundle_cmd.env("ENVLT_BUNDLE_PASSPHRASE", "bundle-passphrase");
    import_bundle_cmd
        .args(["import", bundle_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("shared-project"));

    let mut list_cmd = Command::cargo_bin("envlt").expect("binary exists");
    list_cmd.env("ENVLT_HOME", home_b.path());
    list_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    list_cmd
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("shared-project"));
}

#[test]
fn import_fails_with_wrong_bundle_passphrase() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let bundle_path = project_dir.path().join("bundle.evlt");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "TOKEN=abc123\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "secure-project"])
        .assert()
        .success();
    cli(&home)
        .args([
            "export",
            "secure-project",
            "--out",
            bundle_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let import_home = TempDir::new().expect("tempdir");
    let mut init_cmd = Command::cargo_bin("envlt").expect("binary exists");
    init_cmd.env("ENVLT_HOME", import_home.path());
    init_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    init_cmd.env("ENVLT_BUNDLE_PASSPHRASE", "wrong-bundle-passphrase");
    init_cmd.arg("init").assert().success();

    let mut import_cmd = Command::cargo_bin("envlt").expect("binary exists");
    import_cmd.env("ENVLT_HOME", import_home.path());
    import_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    import_cmd.env("ENVLT_BUNDLE_PASSPHRASE", "wrong-bundle-passphrase");
    import_cmd
        .args(["import", bundle_path.to_str().expect("utf8 path")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bundle payload decryption failed"));
}

#[test]
fn import_fails_when_project_already_exists_without_overwrite() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let bundle_path = project_dir.path().join("bundle.evlt");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "TOKEN=abc123\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "existing-project"])
        .assert()
        .success();
    cli(&home)
        .args([
            "export",
            "existing-project",
            "--out",
            bundle_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    cli(&home)
        .args(["import", bundle_path.to_str().expect("utf8 path")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn import_with_overwrite_replaces_existing_project_snapshot() {
    let home_source = TempDir::new().expect("tempdir");
    let home_target = TempDir::new().expect("tempdir");
    let source_project_dir = TempDir::new().expect("tempdir");
    let target_project_dir = TempDir::new().expect("tempdir");
    let source_env_path = source_project_dir.path().join(".env");
    let target_env_path = target_project_dir.path().join(".env");
    let bundle_path = source_project_dir.path().join("bundle.evlt");
    let output_path = target_project_dir.path().join(".env.generated");

    fs::write(&source_env_path, "MODE=prod\nTOKEN=from-bundle\n").expect("write source env");
    fs::write(&target_env_path, "MODE=dev\nTOKEN=local\n").expect("write target env");

    cli(&home_source).arg("init").assert().success();
    cli(&home_source)
        .current_dir(source_project_dir.path())
        .args(["add", "shared-project"])
        .assert()
        .success();
    cli(&home_source)
        .args([
            "export",
            "shared-project",
            "--out",
            bundle_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    cli(&home_target).arg("init").assert().success();
    cli(&home_target)
        .current_dir(target_project_dir.path())
        .args(["add", "shared-project"])
        .assert()
        .success();

    let mut import_cmd = Command::cargo_bin("envlt").expect("binary exists");
    import_cmd.env("ENVLT_HOME", home_target.path());
    import_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    import_cmd.env("ENVLT_BUNDLE_PASSPHRASE", "bundle-passphrase");
    import_cmd
        .args([
            "import",
            bundle_path.to_str().expect("utf8 path"),
            "--overwrite",
        ])
        .assert()
        .success();

    let mut use_cmd = Command::cargo_bin("envlt").expect("binary exists");
    use_cmd.env("ENVLT_HOME", home_target.path());
    use_cmd.env("ENVLT_PASSPHRASE", "test-passphrase");
    use_cmd
        .args([
            "use",
            "--project",
            "shared-project",
            "--out",
            output_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let generated = fs::read_to_string(output_path).expect("generated env");
    assert!(generated.contains("MODE=prod"));
    assert!(generated.contains("TOKEN=from-bundle"));
    assert!(!generated.contains("TOKEN=local"));
}

#[test]
fn add_from_example_uses_defaults_and_prompts_missing_values() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let example_path = project_dir.path().join(".env.example");
    let output_path = project_dir.path().join(".env.generated");

    fs::write(&example_path, "APP_ENV=dev\nAPI_KEY=\n").expect("write example");

    cli(&home).arg("init").assert().success();
    cli_with_example(&home, &[("API_KEY", "from-example-secret")])
        .current_dir(project_dir.path())
        .args([
            "add",
            "example-project",
            "--from-example",
            example_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    cli(&home)
        .args([
            "use",
            "--project",
            "example-project",
            "--out",
            output_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success();

    let generated = fs::read_to_string(output_path).expect("generated env");
    assert!(generated.contains("APP_ENV=dev"));
    assert!(generated.contains("API_KEY=from-example-secret"));
}

#[test]
fn vars_shows_types_and_masks_secret_values() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "API_KEY=abc123\nPORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "typed-project"])
        .assert()
        .success();

    cli(&home)
        .args(["vars", "--project", "typed-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("API_KEY\tsecret\tab***"))
        .stdout(predicate::str::contains("PORT\tconfig\t3000"));
}

#[test]
fn vars_can_resolve_project_from_link() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "JWT_SECRET=supersecret\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "linked-vars-project"])
        .assert()
        .success();

    cli(&home)
        .current_dir(project_dir.path())
        .arg("vars")
        .assert()
        .success()
        .stdout(predicate::str::contains("JWT_SECRET\tsecret\tsu***"));
}

#[test]
fn set_can_override_variable_type_explicitly() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "PORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "typed-set-project"])
        .assert()
        .success();

    cli(&home)
        .args([
            "set",
            "--project",
            "typed-set-project",
            "--plain",
            "PORT=4000",
        ])
        .assert()
        .success();

    cli(&home)
        .args(["vars", "--project", "typed-set-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PORT\tplain\t4000"));
}

#[test]
fn diff_example_reports_shared_missing_and_extra_keys() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");
    let example_path = project_dir.path().join(".env.example");

    fs::write(&env_path, "PORT=3000\nAPI_KEY=abc123\nLOCAL_ONLY=1\n").expect("write env");
    fs::write(&example_path, "PORT=\nAPI_KEY=\nREQUIRED_KEY=\n").expect("write example");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "diff-project"])
        .assert()
        .success();

    cli(&home)
        .args([
            "diff",
            "--project",
            "diff-project",
            "--example",
            example_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("shared\t2"))
        .stdout(predicate::str::contains("missing\t1"))
        .stdout(predicate::str::contains("extra\t1"))
        .stdout(predicate::str::contains("ok\tAPI_KEY"))
        .stdout(predicate::str::contains("ok\tPORT"))
        .stdout(predicate::str::contains("missing\tREQUIRED_KEY"))
        .stdout(predicate::str::contains("extra\tLOCAL_ONLY"));
}

#[test]
fn diff_example_can_resolve_project_from_link() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");
    let example_path = project_dir.path().join(".env.example");

    fs::write(&env_path, "PORT=3000\n").expect("write env");
    fs::write(&example_path, "PORT=\nREQUIRED_KEY=\n").expect("write example");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "linked-diff-project"])
        .assert()
        .success();

    cli(&home)
        .current_dir(project_dir.path())
        .args([
            "diff",
            "--example",
            example_path.to_str().expect("utf8 path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("missing\tREQUIRED_KEY"));
}

#[test]
fn diff_between_projects_reports_shared_and_unique_keys() {
    let home = TempDir::new().expect("tempdir");
    let left_dir = TempDir::new().expect("tempdir");
    let right_dir = TempDir::new().expect("tempdir");
    let left_env_path = left_dir.path().join(".env");
    let right_env_path = right_dir.path().join(".env");

    fs::write(&left_env_path, "PORT=3000\nLEFT_ONLY=1\nSHARED=yes\n").expect("write left env");
    fs::write(&right_env_path, "PORT=4000\nRIGHT_ONLY=1\nSHARED=yes\n").expect("write right env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(left_dir.path())
        .args(["add", "left-project"])
        .assert()
        .success();
    cli(&home)
        .current_dir(right_dir.path())
        .args(["add", "right-project"])
        .assert()
        .success();

    cli(&home)
        .args(["diff", "--project", "left-project", "right-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shared\t2"))
        .stdout(predicate::str::contains("changed_values\t1"))
        .stdout(predicate::str::contains("changed_types\t0"))
        .stdout(predicate::str::contains("only_left\t1"))
        .stdout(predicate::str::contains("only_right\t1"))
        .stdout(predicate::str::contains("ok\tPORT"))
        .stdout(predicate::str::contains("ok\tSHARED"))
        .stdout(predicate::str::contains("value_changed\tPORT"))
        .stdout(predicate::str::contains("left_only\tLEFT_ONLY"))
        .stdout(predicate::str::contains("right_only\tRIGHT_ONLY"));
}

#[test]
fn diff_between_projects_reports_type_changes_separately() {
    let home = TempDir::new().expect("tempdir");
    let left_dir = TempDir::new().expect("tempdir");
    let right_dir = TempDir::new().expect("tempdir");
    let left_env_path = left_dir.path().join(".env");
    let right_env_path = right_dir.path().join(".env");

    fs::write(&left_env_path, "API_TOKEN=same\n").expect("write left env");
    fs::write(&right_env_path, "API_TOKEN=same\n").expect("write right env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(left_dir.path())
        .args(["add", "left-project"])
        .assert()
        .success();
    cli(&home)
        .current_dir(right_dir.path())
        .args(["add", "right-project"])
        .assert()
        .success();

    cli(&home)
        .args([
            "set",
            "--project",
            "right-project",
            "--plain",
            "API_TOKEN=same",
        ])
        .assert()
        .success();

    cli(&home)
        .args(["diff", "--project", "left-project", "right-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("changed_values\t0"))
        .stdout(predicate::str::contains("changed_types\t1"))
        .stdout(predicate::str::contains("type_changed\tAPI_TOKEN"));
}

#[test]
fn diff_without_mode_returns_error() {
    let home = TempDir::new().expect("tempdir");

    cli(&home).arg("init").assert().success();

    cli(&home)
        .arg("diff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires either --example"));
}

#[test]
fn gen_list_types_shows_supported_types() {
    let home = TempDir::new().expect("tempdir");

    cli(&home)
        .args(["gen", "--list-types"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jwt-secret"))
        .stdout(predicate::str::contains("uuid"))
        .stdout(predicate::str::contains("api-key"))
        .stdout(predicate::str::contains("token"))
        .stdout(predicate::str::contains("password"));
}

#[test]
fn gen_can_store_value_in_project() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "PORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "gen-project"])
        .assert()
        .success();

    cli(&home)
        .args([
            "gen",
            "--type",
            "jwt-secret",
            "--set",
            "JWT_SECRET",
            "--project",
            "gen-project",
            "--silent",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Value generated and saved."));

    cli(&home)
        .args(["vars", "--project", "gen-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JWT_SECRET\tsecret\t"));
}

#[test]
fn gen_supports_custom_hex_length() {
    let home = TempDir::new().expect("tempdir");

    cli(&home)
        .args(["gen", "--len", "64", "--hex"])
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let value = output.trim();
            value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
        }));
}

#[test]
fn gen_supports_custom_symbols_length_and_store() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "PORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "gen-custom-project"])
        .assert()
        .success();

    cli(&home)
        .args([
            "gen",
            "--len",
            "32",
            "--symbols",
            "--set",
            "CUSTOM_SECRET",
            "--project",
            "gen-custom-project",
            "--silent",
        ])
        .assert()
        .success();

    cli(&home)
        .args(["vars", "--project", "gen-custom-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("CUSTOM_SECRET"));
}

#[test]
fn gen_password_preset_outputs_four_words() {
    let home = TempDir::new().expect("tempdir");

    cli(&home)
        .args(["gen", "--type", "password"])
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let parts: Vec<_> = output.trim().split('-').collect();
            parts.len() == 4 && parts.iter().all(|part| !part.is_empty())
        }));
}

#[test]
fn gen_without_type_uses_interactive_env_override() {
    let home = TempDir::new().expect("tempdir");
    let mut cmd = cli(&home);
    cmd.env("ENVLT_GEN_TYPE", "password");
    cmd.arg("gen")
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let parts: Vec<_> = output.trim().split('-').collect();
            parts.len() == 4 && parts.iter().all(|part| !part.is_empty())
        }));
}

#[test]
fn gen_interactive_mode_can_store_value_via_env_overrides() {
    let home = TempDir::new().expect("tempdir");
    let project_dir = TempDir::new().expect("tempdir");
    let env_path = project_dir.path().join(".env");

    fs::write(&env_path, "PORT=3000\n").expect("write env");

    cli(&home).arg("init").assert().success();
    cli(&home)
        .current_dir(project_dir.path())
        .args(["add", "interactive-gen-project"])
        .assert()
        .success();

    let mut cmd = cli(&home);
    cmd.current_dir(project_dir.path());
    cmd.env("ENVLT_GEN_TYPE", "jwt-secret");
    cmd.env("ENVLT_GEN_SAVE", "yes");
    cmd.env("ENVLT_GEN_SET_KEY", "JWT_SECRET");
    cmd.arg("gen")
        .assert()
        .success()
        .stdout(predicate::function(|output: &str| {
            let value = output.trim();
            value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
        }));

    cli(&home)
        .args(["vars", "--project", "interactive-gen-project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JWT_SECRET\tsecret\t"));
}
