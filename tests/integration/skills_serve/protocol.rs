//! The handshake, the tool list, a first search and the JSON-RPC error answers.

use serde_json::json;

use super::{Server, fixture};

#[test]
fn initialize_echoes_the_protocol_version_and_lists_the_tools_capability() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    let answer = server.initialize();

    assert_eq!(answer["jsonrpc"], "2.0");
    assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(answer["result"]["capabilities"], json!({ "tools": {} }));
    assert!(answer["result"]["serverInfo"]["name"].is_string());
}

#[test]
fn tools_list_names_head_search_and_get_with_their_arguments() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    server.initialize();
    let answer = server.request("tools/list", &json!({}));

    let tools = answer["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["head", "search", "get"]);
    let properties = |name: &str| {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        let mut keys: Vec<String> = tool["inputSchema"]["properties"]
            .as_object()
            .expect("properties")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    assert_eq!(properties("get"), ["file", "skill"]);
    assert_eq!(
        properties("search"),
        ["categories", "cursor", "kinds", "limit", "requests"]
    );
}

#[test]
fn search_answers_each_request_with_the_version_of_head() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    server.initialize();
    let arguments = json!({ "requests": [{ "id": "T1", "needs": ["anything at all"] }] });
    let (is_error, value) = server.call("search", &arguments);
    let head = server.head();

    assert!(!is_error, "search failed: {value}");
    assert_eq!(value["index_version"], head["version"]);
    assert_eq!(value["results"][0]["request"], "T1");
    assert!(value["results"][0]["items"].is_array());
}

#[test]
fn a_notification_gets_no_answer_and_an_unknown_method_gets_32601() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    server.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    server.send(&json!({ "jsonrpc": "2.0", "method": "no/such/notification" }));
    let answer = server.request("no/such/method", &json!({}));

    assert_eq!(answer["error"]["code"], -32601);
}

#[test]
fn a_line_that_is_not_json_gets_32700_and_the_server_goes_on() {
    let mut server = Server::start(&[&fixture("lib-a")]);
    let answer = server.raw("{ this is not json");

    assert_eq!(answer["error"]["code"], -32700);
    assert_eq!(answer["id"], json!(null));
    assert_eq!(server.request("ping", &json!({}))["result"], json!({}));
}
