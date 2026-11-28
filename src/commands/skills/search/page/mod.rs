//! From the kept candidates of one request to one page of its result: their order, the item of
//! each, the cursor that continues the result, and the size limit.

mod cursor;
mod item;
mod order;
mod size;

pub(super) use cursor::CursorKey;
pub(super) use item::{item, write_confirms};
pub(super) use order::{first_kept_requirer, order};
pub(super) use size::fit;
pub use size::{HASH_TOKENS_PER_EIGHT_BYTES, TEXT_TOKENS_PER_FIVE_BYTES, estimated_tokens};
