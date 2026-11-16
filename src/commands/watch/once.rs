//! `rex watch --once`: the whole view as plain text, one line per node in a tree
//! with guides, for agents, tests and terminals that cannot draw the screen.

use super::data::WatchData;
use super::facts::{answer_command, options_text};
use super::nav::Node;
use super::view::{NOTHING_YET, RowText, counted, labelled, tree_guides};

/// The text that `--once` prints, every line ending in a newline.
pub fn once_text(data: &WatchData) -> String {
    let mut lines: Vec<(usize, String)> = Vec::new();
    for row in data.rows(None) {
        push_node(data, &row.node, 1, &mut lines);
    }
    let waiting = data.waiting_count();
    let mut text = format!(
        "rex watch  source {}  ·  {}  ·  {}  ·  {} waiting\n",
        data.sources.text(),
        counted(data.trees.len(), "tree", "trees"),
        counted(data.jobs.len(), "job", "jobs"),
        counted(waiting, "question", "questions"),
    );
    if lines.is_empty() {
        text.push_str(NOTHING_YET);
        text.push('\n');
        return text;
    }
    let depths: Vec<usize> = lines.iter().map(|(depth, _)| *depth).collect();
    for (guide, (_, line)) in tree_guides(&depths).into_iter().zip(lines) {
        text.push_str(&guide);
        text.push_str(&line);
        text.push('\n');
    }
    text
}

/// The node's line, then its waiting questions, then each node right below it.
fn push_node(data: &WatchData, node: &Node, depth: usize, lines: &mut Vec<(usize, String)>) {
    let Some(row) = data.row_text(node) else {
        return;
    };
    lines.push((depth, row_line(&row)));
    let own_questions = match node {
        Node::Tree(id) => data.tree(id).map(|bundle| bundle.questions.as_slice()),
        Node::Job(id) => data.job(id).map(|bundle| bundle.questions.as_slice()),
        Node::Run(_) => None,
    };
    let waiting = own_questions
        .unwrap_or_default()
        .iter()
        .filter(|question| !question.is_answered);
    for question in waiting {
        lines.push((
            depth + 1,
            format!(
                "? {}  {}  options {}  answer with {}",
                question.id,
                question.question,
                options_text(question),
                answer_command(question)
            ),
        ));
    }
    for child in data
        .rows(Some(node))
        .into_iter()
        .filter(|row| row.indent == 0)
    {
        push_node(data, &child.node, depth + 1, lines);
    }
}

/// The same facts as the node's row on the screen.
pub fn row_line(row: &RowText) -> String {
    let mut parts = vec![labelled(row.level, &row.name), row.state.clone()];
    if !row.origin.is_empty() {
        parts.push(row.origin.to_owned());
    }
    parts.extend([
        row.model.clone(),
        row.time.clone(),
        row.cost.clone(),
        format!("hit {}", row.hit),
    ]);
    if row.waiting > 0 {
        parts.push(format!("{} waiting", row.waiting));
    }
    parts.push(row.detail.clone());
    parts.join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::live::{Snapshot, epoch_ms};

    /// The snapshot of a live run of one tree, with the roles its plan gives:
    /// lead T5, workers T1 and T3, a helper below each worker.
    const LIVE_RUN: &str = include_str!("../../../tests/fixtures/watch/live-run.json");

    #[test]
    fn rows_of_a_live_run_say_a_role_that_is_the_level_word_once() {
        let snapshot = Snapshot::parse(LIVE_RUN).unwrap();
        let part = snapshot.part(epoch_ms(&snapshot.written_at).unwrap());
        let text = once_text(&WatchData::merge(vec![part], None));

        for wanted in [
            "lead T5",
            "worker T1",
            "worker T3",
            "helper T2",
            "helper T4",
        ] {
            assert!(
                text.contains(&format!("─ {wanted}  done")),
                "no row named {wanted} in:\n{text}"
            );
        }
    }
}
