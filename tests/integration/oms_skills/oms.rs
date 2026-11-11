//! Integration test of `rex oms`: the specification example in plain and JSON form.

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

fn stdout_of_success(arguments: &[&str]) -> String {
    let output = Command::cargo_bin("rex")
        .expect("find rex binary")
        .arg("oms")
        .args(arguments)
        .output()
        .expect("run rex oms");
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
                    version=2\n";
    assert_eq!(stdout_of_success(&SPECIFICATION_EXAMPLE), expected);

    let mut arguments = SPECIFICATION_EXAMPLE.to_vec();
    arguments.push("--json");
    let expected_json = concat!(
        r#"{"agent":"model-medium-effort-high","demand":"model-xhigh-effort-max","#,
        r#""rules":["T1","T2","E1","E2","E4","C1","C2","F1"],"conflict":["budget"],"#,
        r#""inputs":"-LHHHHL","version":2}"#,
        "\n"
    );
    assert_eq!(stdout_of_success(&arguments), expected_json);
}
