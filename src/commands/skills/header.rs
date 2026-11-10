//! Reads the header at the top of a `SKILL.md`. Listing and checking skills
//! both read it here, so the two commands always see the same fields.

use yaml_rust2::yaml::Hash;
use yaml_rust2::{ScanError, Yaml, YamlLoader};

/// A `SKILL.md` cut at its header. Line ends are LF.
pub(super) struct Parts {
    /// The lines between the opening and the closing `---`.
    pub(super) header: String,
    /// Everything after the closing `---` line.
    pub(super) body: String,
}

impl Parts {
    /// Cuts `text` at its header. `None` when line 1 is not exactly `---` or
    /// no later line is exactly `---`.
    pub(super) fn split(text: &str) -> Option<Parts> {
        let text = text.replace("\r\n", "\n");
        let rest = text.strip_prefix("---\n")?;
        let mut offset = 0;
        for line in rest.split_inclusive('\n') {
            if line.strip_suffix('\n').unwrap_or(line) == "---" {
                return Some(Parts {
                    header: rest[..offset].to_owned(),
                    body: rest[offset + line.len()..].to_owned(),
                });
            }
            offset += line.len();
        }
        None
    }
}

/// Why a header could not be read as a YAML mapping.
pub(super) enum HeaderError {
    Yaml(ScanError),
    NotMapping,
}

/// The parsed header, with the lines it was written as.
pub(super) struct Header {
    fields: Hash,
    written: String,
}

impl Header {
    /// Parses the header lines that [`Parts::split`] gives.
    pub(super) fn parse(header: &str) -> Result<Header, HeaderError> {
        let documents = YamlLoader::load_from_str(header).map_err(HeaderError::Yaml)?;
        match documents.into_iter().next() {
            Some(Yaml::Hash(fields)) => Ok(Header {
                fields,
                written: header.to_owned(),
            }),
            _ => Err(HeaderError::NotMapping),
        }
    }

    /// The header of a whole `SKILL.md`, or `None` when it has no header or
    /// the header is not a YAML mapping.
    pub(super) fn of_skill_file(text: &str) -> Option<Header> {
        let parts = Parts::split(text)?;
        Header::parse(&parts.header).ok()
    }

    pub(super) fn value(&self, key: &str) -> Option<&Yaml> {
        self.fields.get(&Yaml::String(key.to_owned()))
    }

    /// The value of `key` as one line of text, or `None` when it is missing or
    /// is not a string.
    pub(super) fn text(&self, key: &str) -> Option<String> {
        self.value(key)?.as_str().map(one_line)
    }

    /// Every key in written order. A key that is not a string is shown as
    /// YAML's debug form, so that a check can still name it.
    pub(super) fn keys(&self) -> Vec<String> {
        self.fields
            .keys()
            .map(|key| match key {
                Yaml::String(key) => key.clone(),
                other => format!("{other:?}"),
            })
            .collect()
    }

    /// The text of `key` as written, when its value is plain (it starts on
    /// the key's own line and not with a quote or a block mark). The lines
    /// that continue a plain value are joined on, so the text can be compared
    /// with what YAML gave back.
    pub(super) fn written_plain_text(&self, key: &str) -> Option<String> {
        let mut lines = self.written.lines();
        let first = lines.find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))?;
        let first = first.trim();
        if first.is_empty() || first.starts_with(['>', '|', '\'', '"']) {
            return None;
        }
        let continued = lines.take_while(|line| line.is_empty() || line.starts_with([' ', '\t']));
        let written: Vec<&str> = std::iter::once(first).chain(continued).collect();
        Some(one_line(&written.join(" ")))
    }
}

/// Makes each run of spaces, tabs and line ends one space, with no space at
/// either end.
pub(super) fn one_line(text: &str) -> String {
    let words: Vec<&str> = text
        .split([' ', '\t', '\n', '\r'])
        .filter(|word| !word.is_empty())
        .collect();
    words.join(" ")
}
