//! Integration tests of `rex oms`: the specification example, its JSON form,
//! the duration words and a bad value.

use assert_cmd::Command;

const SPECIFICATION_EXAMPLE: [&str; 12] = [
    "--budget",
    "low",
    "--intelligence",
    "high",
    "--autonomy",
    "high",
    "--reliability",
    "high",
    "--consequence",
    "high",
    "--verification",
    "low",
];

fn rex_oms(arguments: &[&str]) -> std::process::Output {
    Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("oms")
        .args(arguments)
        .output()
        .expect("run rex oms")
}

fn stdout_of_success(arguments: &[&str]) -> String {
    let output = rex_oms(arguments);
    assert_eq!(
        output.status.code(),
        Some(0),
        "rex oms {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn oms_prints_six_lines_for_the_specification_example() {
    let expected = "agent=model-medium-effort-high\n\
                    demand=model-xhigh-effort-max\n\
                    rules=T1,T2,E1,E2,E4,C1,C2,F1\n\
                    conflict=budget\n\
                    inputs=-LHHHHL\n\
                    version=1\n";

    assert_eq!(stdout_of_success(&SPECIFICATION_EXAMPLE), expected);
}

#[test]
fn oms_json_matches_the_specification_example() {
    let mut arguments = SPECIFICATION_EXAMPLE.to_vec();
    arguments.push("--json");
    let expected = concat!(
        r#"{"agent":"model-medium-effort-high","demand":"model-xhigh-effort-max","#,
        r#""rules":["T1","T2","E1","E2","E4","C1","C2","F1"],"conflict":["budget"],"#,
        r#""inputs":"-LHHHHL","version":1}"#,
        "\n"
    );

    assert_eq!(stdout_of_success(&arguments), expected);
}

#[test]
fn oms_duration_accepts_fast_and_long() {
    // High autonomy and hard checking ask for xhigh effort, so a fast
    // duration caps it and a long one does not.
    let rest = ["--autonomy", "high", "--verification", "low"];
    let output_for = |duration: &str| {
        let mut arguments = vec!["--duration", duration];
        arguments.extend(rest);
        stdout_of_success(&arguments)
    };

    assert_eq!(output_for("fast"), output_for("low"));
    assert_eq!(output_for("long"), output_for("high"));
    assert_ne!(output_for("fast"), output_for("long"));
}

#[test]
fn oms_unknown_value_exits_2() {
    let output = rex_oms(&["--budget", "huge"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty(), "stderr names the bad value");
    assert!(output.stdout.is_empty(), "stdout stays empty");
}
