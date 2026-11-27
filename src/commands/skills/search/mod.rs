//! Search of a skill library by code: three lookups in order of trust over an index of the served
//! skills, and one answer that `rex skills search` and the serve `search` tool both give.

mod answer;
mod claims;
mod cli;
mod files;
mod index;
mod index_file;
mod keyword;
mod load;
mod page;
mod request;
mod response;
mod vocabulary;

pub use answer::search;
pub use claims::{BASE_LANGUAGE, language_gaps, lookup_a, lookup_b, required_skills};
pub use files::{DEFAULT_MAX_TOPICS, choose_files};
pub use index::{
    Claims, FileEntry, FileRole, Index, IndexVersion, Kind, OpenWhen, Placement, Scope, SkillEntry,
    SkillMetadata,
};
pub use index_file::{IndexFile, IndexFileFile, IndexFileSkill};
pub use keyword::{
    CARD_WEIGHT, INTENTS_WEIGHT, KEYWORD_SCORE_RATIO, KeywordTable, MAX_KEYWORD_CANDIDATES,
    NAME_WEIGHT, Ranked, lookup_c,
};
pub use load::{CARD_WORDS, SkillSource};
pub use page::{
    HASH_TOKENS_PER_EIGHT_BYTES, MAX_TOKENS_PER_ITEM, TEXT_TOKENS_PER_FIVE_BYTES, estimated_tokens,
};
pub use request::{
    AskedTerm, DEFAULT_LIMIT, Evidence, Facts, FormTerms, MAX_NEEDS, Need, RequestForm,
    SearchRequest, TermField, TextQuery,
};
pub use response::{
    Candidate, Dropped, FileChoice, FileItem, Found, Gap, Item, Question, RequestResult,
    SearchError, SearchResponse, Slot, Tier,
};
pub use vocabulary::{Alias, BroaderTerm, Relation, TermGroup, Vocabulary};

pub(crate) use cli::run as run_search;
pub(crate) use index_file::run as run_index;
