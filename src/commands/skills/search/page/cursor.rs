//! The cursor that continues a result: the offset of the next page, bound to one request on one
//! version of the library so that it never continues another list.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::super::index::IndexVersion;
use super::super::keyword::KeywordSettings;
use super::super::request::{RequestForm, SearchRequest};
use super::super::response::SearchError;

const CURSOR_TAG: &str = "c1";
const BINDING_DIGITS: usize = 16;

/// What binds a cursor to one request on one version of the library.
pub(crate) struct CursorKey {
    version: String,
    request: String,
}

/// The part of a request that the cursor binds, in this field order. The
/// `facts` of the form are sorted by name, so their order as sent does not count.
/// A keyword ratio other than the built-in one is bound too, since it cuts
/// another list.
#[derive(Serialize)]
struct Bound<'a> {
    form: Value,
    kinds: &'a Option<Vec<String>>,
    categories: &'a Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyword_ratio: Option<f64>,
}

impl CursorKey {
    pub(crate) fn new(
        version: &IndexVersion,
        request: &SearchRequest,
        form: &RequestForm,
        settings: &KeywordSettings,
    ) -> Self {
        let mut form = serde_json::to_value(form).expect("a request form is JSON");
        if let Some(facts) = form.get_mut("facts").and_then(Value::as_object_mut) {
            facts.sort_keys();
        }
        let bound = Bound {
            form,
            kinds: &request.kinds,
            categories: &request.categories,
            keyword_ratio: settings.changed_ratio(),
        };
        let text = serde_json::to_string(&bound).expect("a request form is JSON");
        let digest: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let version = version.0.strip_prefix("sha256:").unwrap_or(&version.0);
        CursorKey {
            version: version.chars().take(BINDING_DIGITS).collect(),
            request: digest[..BINDING_DIGITS].to_owned(),
        }
    }

    /// The cursor of the page that starts at `offset`.
    pub(crate) fn cursor(&self, offset: usize) -> String {
        format!("{CURSOR_TAG}.{offset}.{}.{}", self.version, self.request)
    }

    /// The offset that `cursor` continues from. The caller checks it against
    /// the length of the list.
    pub(crate) fn read(&self, cursor: &str) -> Result<usize, SearchError> {
        let bad = |reason: &str| SearchError::BadCursor {
            reason: reason.to_owned(),
        };
        let parts: Vec<&str> = cursor.split('.').collect();
        let [tag, offset, version, request] = parts[..] else {
            return Err(bad("it is not four parts joined by `.`"));
        };
        if tag != CURSOR_TAG {
            return Err(bad("its tag is not `c1`"));
        }
        if offset.is_empty() || !offset.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(bad("its offset is not a decimal number"));
        }
        let offset: usize = offset.parse().map_err(|_| bad("its offset is too large"))?;
        if !is_binding(version) || !is_binding(request) {
            return Err(bad("its bindings are not 16 lower-case hex digits"));
        }
        if request != self.request {
            return Err(bad("it was made for another request"));
        }
        if version != self.version {
            return Err(SearchError::StaleCursor);
        }
        Ok(offset)
    }
}

fn is_binding(text: &str) -> bool {
    text.len() == BINDING_DIGITS
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
