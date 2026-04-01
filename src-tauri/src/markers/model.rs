use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkerKind {
    Start,
    End,
    StartEnd,
}

/// A raw time-position marker. Labels belong to segments, not markers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub id: Uuid,
    /// Milliseconds from the start of the media file.
    pub position: u64,
    pub kind: MarkerKind,
}

/// A validated segment resolved from a Start+End pair or a StartEnd marker.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub title: String,
}
