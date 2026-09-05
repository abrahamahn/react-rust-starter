// Ported from BSLT main/shared/src/modules/core/auth/auth.session.logic.ts.
// Cookie lifetime and server expiry share the SAME span; idle is derived from it.
pub const DAY: i64 = 86_400;
pub fn span_seconds(remember: bool) -> i64 {
    if remember { 30 * DAY } else { DAY / 2 }
}
pub fn idle_seconds(span: i64) -> i64 {
    span.min(7 * DAY)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_and_remembered_lifetimes_match_source_policy() {
        assert_eq!(span_seconds(false), 43_200);
        assert_eq!(span_seconds(true), 2_592_000);
        assert_eq!(idle_seconds(span_seconds(false)), 43_200);
        assert_eq!(idle_seconds(span_seconds(true)), 604_800);
    }
}
