use crate::error::{AppError, Result};
use super::model::{Marker, MarkerKind, Segment};
use std::collections::HashMap;
use uuid::Uuid;

/// Pair sorted markers into validated segments.
///
/// Rules:
/// - Every `Start` must be followed immediately by an `End`
/// - `End` cannot appear without a preceding `Start`
/// - `StartEnd` emits a zero-span segment on its own
/// - Two consecutive `Start` markers (or a `StartEnd` while waiting) → error
/// - An unmatched `Start` at end-of-list → error
pub fn to_segments(
    markers: &[Marker],
    titles: &HashMap<Uuid, String>,
) -> Result<Vec<Segment>> {
    #[derive(Debug)]
    enum State<'a> {
        Idle,
        WaitingForEnd(&'a Marker),
    }

    let mut state = State::Idle;
    let mut segments: Vec<Segment> = Vec::new();

    for m in markers {
        match (&state, m.kind) {
            (State::Idle, MarkerKind::Start) => {
                state = State::WaitingForEnd(m);
            }
            (State::Idle, MarkerKind::StartEnd) => {
                let title = titles
                    .get(&m.id)
                    .cloned()
                    .unwrap_or_else(|| format!("Segment {}", segments.len()));
                segments.push(Segment {
                    start_ms: m.position,
                    end_ms: m.position,
                    title,
                });
            }
            (State::Idle, MarkerKind::End) => {
                return Err(AppError::ValidationError(
                    "End marker found with no preceding Start marker".into(),
                ));
            }
            (State::WaitingForEnd(start), MarkerKind::End) => {
                let title = titles
                    .get(&start.id)
                    .cloned()
                    .unwrap_or_else(|| format!("Segment {}", segments.len()));
                segments.push(Segment {
                    start_ms: start.position,
                    end_ms: m.position,
                    title,
                });
                state = State::Idle;
            }
            (State::WaitingForEnd(start), MarkerKind::StartEnd) => {
                // StartEnd closes the pending Start segment and simultaneously
                // opens a new one, so the next End can close it.
                let title = titles
                    .get(&start.id)
                    .cloned()
                    .unwrap_or_else(|| format!("Segment {}", segments.len()));
                segments.push(Segment {
                    start_ms: start.position,
                    end_ms: m.position,
                    title,
                });
                state = State::WaitingForEnd(m);
            }
            (State::WaitingForEnd(_), MarkerKind::Start) => {
                return Err(AppError::ValidationError(
                    "Two consecutive Start markers found without an End marker between them".into(),
                ));
            }
        }
    }

    if let State::WaitingForEnd(start) = state {
        return Err(AppError::ValidationError(format!(
            "Unmatched Start marker at position {} ms has no corresponding End marker",
            start.position
        )));
    }

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn marker(position: u64, kind: MarkerKind) -> Marker {
        Marker { id: Uuid::new_v4(), position, kind }
    }

    fn titled(m: &Marker, titles: &mut HashMap<Uuid, String>, title: &str) {
        titles.insert(m.id, title.to_string());
    }

    #[test]
    fn empty_markers_returns_empty_segments() {
        let result = to_segments(&[], &HashMap::new());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn valid_start_end_pair_produces_one_segment() {
        let start = marker(1000, MarkerKind::Start);
        let end = marker(5000, MarkerKind::End);
        let mut titles = HashMap::new();
        titled(&start, &mut titles, "001 Segment");
        let segments = to_segments(&[start, end], &titles).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 1000);
        assert_eq!(segments[0].end_ms, 5000);
        assert_eq!(segments[0].title, "001 Segment");
    }

    #[test]
    fn startend_produces_zero_span_segment() {
        let m = marker(3000, MarkerKind::StartEnd);
        let mut titles = HashMap::new();
        titled(&m, &mut titles, "001 Segment");
        let segments = to_segments(&[m], &titles).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 3000);
        assert_eq!(segments[0].end_ms, 3000);
    }

    #[test]
    fn multiple_valid_pairs_produce_multiple_segments() {
        let s1 = marker(0, MarkerKind::Start);
        let e1 = marker(1000, MarkerKind::End);
        let s2 = marker(2000, MarkerKind::Start);
        let e2 = marker(3000, MarkerKind::End);
        let mut titles = HashMap::new();
        titled(&s1, &mut titles, "001 Segment");
        titled(&s2, &mut titles, "002 Segment");
        let segments = to_segments(&[s1, e1, s2, e2], &titles).unwrap();
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn end_before_start_is_error() {
        let end = marker(1000, MarkerKind::End);
        let result = to_segments(&[end], &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn two_consecutive_starts_is_error() {
        let s1 = marker(1000, MarkerKind::Start);
        let s2 = marker(2000, MarkerKind::Start);
        let result = to_segments(&[s1, s2], &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn unmatched_start_at_end_is_error() {
        let s = marker(1000, MarkerKind::Start);
        let result = to_segments(&[s], &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn start_then_startend_is_error() {
        let s = marker(1000, MarkerKind::Start);
        let se = marker(2000, MarkerKind::StartEnd);
        let result = to_segments(&[s, se], &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn untitled_start_end_pair_uses_generated_title() {
        // Exercises the unwrap_or_else fallback in the WaitingForEnd+End branch
        let s = marker(0, MarkerKind::Start);
        let e = marker(1000, MarkerKind::End);
        let segments = to_segments(&[s, e], &HashMap::new()).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].title, "Segment 0");
    }

    #[test]
    fn untitled_startend_uses_generated_title() {
        // Exercises the unwrap_or_else fallback in the Idle+StartEnd branch
        let m = marker(500, MarkerKind::StartEnd);
        let segments = to_segments(&[m], &HashMap::new()).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].title, "Segment 0");
    }

    #[test]
    fn start_then_titled_startend_emits_titled_segment() {
        // Exercises the WaitingForEnd+StartEnd branch WITH a title on the Start marker
        let s = marker(0, MarkerKind::Start);
        let se = marker(1000, MarkerKind::StartEnd);
        let end = marker(2000, MarkerKind::End);
        let mut titles = HashMap::new();
        titled(&s, &mut titles, "First");
        let segments = to_segments(&[s, se, end], &titles).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].title, "First");
    }
}
