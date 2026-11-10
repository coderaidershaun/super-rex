//! The hook registration that `rex init` merges into a repo's Claude Code settings,
//! so that `rex hook` hears every sub-agent start, stop and call.

use serde_json::{Value, json};

/// The command of each of SuperRex's hook entries. An entry is SuperRex's own when
/// its command is exactly this text. The suffix gives exit code 0 and no output
/// whatever program named `rex` is found first on `PATH`, because a hook that exits
/// with code 2 stops the agent. It finds `rex` by `PATH`, as the skills do, so
/// the hook and the skills run the same binary, and the entry does not go stale
/// when rex is installed in another place.
pub const HOOK_COMMAND: &str = "rex hook >/dev/null 2>&1 || true";

/// The value of the `hooks` key of the settings file: sub-agent start and stop for
/// every sub-agent, and the end of each call of the sub-agent tool. No entry is
/// `async`, because an async hook still running when the session ends is killed
/// and its event lost. A failed call is not heard: one that fails before its run
/// starts leaves no run to record.
pub fn hook_registration() -> Value {
    let hooks = || json!([{ "type": "command", "command": HOOK_COMMAND, "timeout": 10 }]);
    json!({
        "SubagentStart": [{ "hooks": hooks() }],
        "SubagentStop": [{ "hooks": hooks() }],
        "PostToolUse": [{ "matcher": "Agent|Task", "hooks": hooks() }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_is_the_documented_hooks_value() {
        let documented = r#"{
            "SubagentStart": [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
            "SubagentStop":  [{ "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }],
            "PostToolUse":   [{ "matcher": "Agent|Task", "hooks": [{ "type": "command", "command": "rex hook >/dev/null 2>&1 || true", "timeout": 10 }] }]
        }"#;
        let documented: Value = serde_json::from_str(documented).unwrap();
        assert_eq!(
            hook_registration().to_string(),
            documented.to_string(),
            "same keys in the same order"
        );
    }
}
