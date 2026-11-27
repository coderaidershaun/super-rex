//! Integration tests of `rex skills serve`: each one starts the binary as a child
//! process and speaks JSON-RPC over its stdin and stdout, as any MCP client would.

mod errors;
mod head;
mod plugin_library;
mod protocol;
mod served_set;
mod shadowing;

use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/skills-serve")
        .join(name)
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("make folder");
    for entry in fs::read_dir(from).expect("read folder") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

/// A running `rex skills serve` and the client side of its pipes.
pub(super) struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Server {
    pub(super) fn start(libraries: &[&Path]) -> Server {
        let mut args: Vec<&OsStr> = Vec::new();
        for library in libraries {
            args.push(OsStr::new("--library"));
            args.push(library.as_os_str());
        }
        Server::start_with(&args)
    }

    /// Starts `rex skills serve` followed by `args`.
    pub(super) fn start_with(args: &[&OsStr]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rex"))
            .args(["skills", "serve"])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start rex skills serve");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Server {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    /// Sends one raw line and reads one answer line.
    pub(super) fn raw(&mut self, line: &str) -> Value {
        writeln!(self.stdin, "{line}").expect("write request");
        self.read_line()
    }

    pub(super) fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("write message");
    }

    pub(super) fn read_line(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read answer");
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("answer is not JSON: {line:?}"))
    }

    pub(super) fn request(&mut self, method: &str, params: &Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let answer = self.read_line();
        assert_eq!(answer["id"], json!(id), "answer carries the request id");
        answer
    }

    pub(super) fn initialize(&mut self) -> Value {
        let answer = self.request(
            "initialize",
            &json!({ "protocolVersion": "2025-06-18", "capabilities": {},
                     "clientInfo": { "name": "test-client", "version": "0" } }),
        );
        self.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        answer
    }

    /// The tool result of a call: `(is_error, parsed JSON of its text)`.
    pub(super) fn call(&mut self, tool: &str, arguments: &Value) -> (bool, Value) {
        let answer = self.request(
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        );
        let result = &answer["result"];
        let text = result["content"][0]["text"].as_str().expect("text content");
        assert_eq!(result["content"][0]["type"], "text");
        let value = serde_json::from_str(text).expect("text is JSON");
        (result["isError"].as_bool().expect("isError"), value)
    }

    pub(super) fn head(&mut self) -> Value {
        let (is_error, value) = self.call("head", &json!({}));
        assert!(!is_error, "head failed: {value}");
        value
    }

    pub(super) fn get_error_code(&mut self, skill: &str, file: &str) -> (String, String) {
        let (is_error, value) = self.call("get", &json!({ "skill": skill, "file": file }));
        assert!(is_error, "get {skill} {file} should fail, got {value}");
        let code = value["code"].as_str().expect("code").to_string();
        (code, value.to_string())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
