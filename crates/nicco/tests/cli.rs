use std::process::{Command, Output};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nicco"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_version_and_argument_errors() {
    for args in [&["--help"][..], &["--version"]] {
        assert!(run(args).status.success());
    }
    let result = run(&["status", "--bogus"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}

#[test]
fn demo_outputs_are_labeled_and_json_is_clean() {
    let result = run(&["status", "--demo", "--json"]);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["source"], "demo");
    let human = run(&["status", "--demo"]);
    assert!(human.status.success());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .contains("DEMO (synthetic data)")
    );
}

#[test]
fn tui_rejects_pipes_before_terminal_setup() {
    let result = run(&["tui", "--demo"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("requires terminal")
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_live_status_has_error_exit_and_no_json() {
    let result = run(&["status", "--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("only on Linux")
    );
}
