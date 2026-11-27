//! Proofs of two facts that the skill library rests on: the bundled SQLite has
//! FTS5 with weighted BM25, and `rex skills check` passes a nested `metadata` header.

mod fts5;
mod skill_metadata;
