//! The hook registration that `rex init` merges into a repo's Claude Code settings,
//! so that `rex hook` hears every sub-agent start, stop and call, `rex gate` can
//! refuse a start once the job has spent its limit, and a test run shows only its failures.

use serde_json::{Value, json};

/// The command of each of SuperRex's hook entries. An entry is SuperRex's own when
/// its command is exactly this text. The suffix gives exit code 0 and no output
/// whatever program named `rex` is found first on `PATH`, because a hook that exits
/// with code 2 stops the agent. It finds `rex` by `PATH`, as the skills do, so
/// the hook and the skills run the same binary, and the entry does not go stale
/// when rex is installed in another place.
pub const HOOK_COMMAND: &str = "rex hook >/dev/null 2>&1 || true";

/// The command of the gate before each start of a sub-agent. Its standard output
/// must reach Claude Code, because that is where a refusal goes. `|| true` turns any
/// failure of `rex`, such as a missing command, into exit code 0 with no output, so
/// the call goes on unchanged.
pub const GATE_COMMAND: &str = "rex gate 2>/dev/null || true";

/// The command before each shell command, which pipes a test command's output
/// through `rex test-filter`. Its standard output must reach Claude Code, which
/// reads the changed command there. `|| true` turns any failure of `rex`, such as a
/// missing command, into exit code 0 with no output, so the call goes on unchanged.
pub const TEST_HOOK_COMMAND: &str = "rex test-hook 2>/dev/null || true";

/// Every command that marks an entry as SuperRex's own.
pub const REX_COMMANDS: [&str; 3] = [HOOK_COMMAND, GATE_COMMAND, TEST_HOOK_COMMAND];

/// The value of the `hooks` key of the settings file: sub-agent start and stop for
/// every sub-agent, and the end of each call of the sub-agent tool. No entry is
/// `async`, because an async hook still running when the session ends is killed
/// and its event lost. A failed call is not heard: one that fails before its run
/// starts leaves no run to record. The gate runs before each call of the
/// sub-agent tool, and the test hook before each shell command.
pub fn hook_registration() -> Value {
    let hooks = || json!([{ "type": "command", "command": HOOK_COMMAND, "timeout": 10 }]);
    let gate = json!([{ "type": "command", "command": GATE_COMMAND, "timeout": 10 }]);
    let test_hook = json!([{ "type": "command", "command": TEST_HOOK_COMMAND, "timeout": 10 }]);
    json!({
        "SubagentStart": [{ "hooks": hooks() }],
        "SubagentStop": [{ "hooks": hooks() }],
        "PostToolUse": [{ "matcher": "Agent|Task", "hooks": hooks() }],
        "PreToolUse": [
            { "matcher": "Agent|Task", "hooks": gate },
            { "matcher": "Bash", "hooks": test_hook },
        ],
    })
}
