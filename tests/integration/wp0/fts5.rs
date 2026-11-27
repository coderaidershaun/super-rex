//! The SQLite that `rusqlite` bundles has FTS5, and `bm25()` takes one weight
//! per column, so keyword search can rank a name match above an intent match.

use rusqlite::Connection;

/// Each row holds the word `latency` in one column only, in a column of three
/// words like every other column, so only the column weights decide the order.
const ROWS: [(&str, &str, &str); 8] = [
    (
        "latency budget rules",
        "write clear code",
        "pick good names",
    ),
    (
        "testing first method",
        "measures latency carefully",
        "write failing tests",
    ),
    (
        "error types guide",
        "typed error values",
        "reduce tail latency",
    ),
    (
        "folder layout rules",
        "where files belong",
        "split big modules",
    ),
    (
        "comment writing rules",
        "when to comment",
        "delete stale comments",
    ),
    (
        "diagram drawing tool",
        "draw flow charts",
        "sketch a sequence",
    ),
    (
        "spike probe method",
        "throwaway code first",
        "prove one unknown",
    ),
    ("review branch method", "read the diff", "find real bugs"),
];

fn search_index() -> Connection {
    let connection = Connection::open_in_memory().expect("open in-memory SQLite");
    connection
        .execute_batch("CREATE VIRTUAL TABLE skills USING fts5(name, card, intents);")
        .expect("create FTS5 table; this fails when FTS5 is not compiled in");
    for (name, card, intents) in ROWS {
        connection
            .execute(
                "INSERT INTO skills (name, card, intents) VALUES (?1, ?2, ?3)",
                (name, card, intents),
            )
            .expect("insert row");
    }
    connection
}

/// The names that match `latency`, best first, under the weights of name,
/// card and intents. `bm25()` is lower for a better match.
fn ranked_names(connection: &Connection, weights: [f64; 3]) -> Vec<String> {
    let mut statement = connection
        .prepare(
            "SELECT name FROM skills WHERE skills MATCH 'latency' \
             ORDER BY bm25(skills, ?1, ?2, ?3)",
        )
        .expect("prepare ranked search");
    statement
        .query_map(weights, |row| row.get(0))
        .expect("run ranked search")
        .collect::<Result<_, _>>()
        .expect("read ranked names")
}

#[test]
fn bundled_sqlite_is_compiled_with_fts5() {
    let connection = Connection::open_in_memory().expect("open in-memory SQLite");
    let (version, has_fts5): (String, i64) = connection
        .query_row(
            "SELECT sqlite_version(), sqlite_compileoption_used('ENABLE_FTS5')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read SQLite version and compile options");

    println!("sqlite_version() = {version}; sqlite_compileoption_used('ENABLE_FTS5') = {has_fts5}");
    assert_eq!(has_fts5, 1, "SQLite {version} was built without FTS5");
}

#[test]
fn bm25_column_weights_of_the_spec_rank_name_then_intents_then_card() {
    let connection = search_index();

    let spec_order = ranked_names(&connection, [4.0, 2.0, 3.0]);
    let card_first_order = ranked_names(&connection, [1.0, 4.0, 2.0]);

    println!("weights 4, 2, 3: {spec_order:?}");
    println!("weights 1, 4, 2: {card_first_order:?}");
    assert_eq!(
        spec_order,
        [
            "latency budget rules",
            "error types guide",
            "testing first method"
        ]
    );
    assert_eq!(
        card_first_order,
        [
            "testing first method",
            "error types guide",
            "latency budget rules"
        ]
    );
}
