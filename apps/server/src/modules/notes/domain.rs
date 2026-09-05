use crate::platform::http::Error;
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WriteNote {
    pub title: String,
    pub body: String,
    pub revision: Option<i32>,
}
impl WriteNote {
    pub fn validate(mut self, updating: bool) -> Result<Self, Error> {
        self.title = self.title.trim().into();
        if self.title.is_empty()
            || self.title.chars().count() > 120
            || self.title.chars().any(char::is_control)
            || self.body.chars().count() > 10_000
            || (updating && !self.revision.is_some_and(|v| v > 0 && v < i32::MAX))
            || (!updating && self.revision.is_some())
        {
            return Err(Error::InvalidInput);
        }
        Ok(self)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteNote {
    pub revision: i32,
}
#[derive(Serialize)]
pub(super) struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub revision: i32,
    pub updated_at: i64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_and_versions_are_explicit() {
        assert!(
            WriteNote {
                title: "  ".into(),
                body: "".into(),
                revision: None
            }
            .validate(false)
            .is_err()
        );
        assert!(
            WriteNote {
                title: "title".into(),
                body: "x".repeat(10001),
                revision: None
            }
            .validate(false)
            .is_err()
        );
        assert!(
            WriteNote {
                title: "title".into(),
                body: "".into(),
                revision: None
            }
            .validate(true)
            .is_err()
        );
        assert_eq!(
            WriteNote {
                title: " title ".into(),
                body: " ".into(),
                revision: None
            }
            .validate(false)
            .unwrap()
            .title,
            "title"
        );
    }
}
