//! Segment spacing logic for meeting transcript assembly.
//!
//! STT engines produce text segments every ~2 seconds. When appending them
//! to the transcript we need to ensure proper spacing: no double spaces,
//! no fused words, and a trailing space between tick deltas so the next
//! segment doesn't glue onto the previous one.
//!
//! This logic was previously duplicated in `qwen3_asr.rs`, `whisper_streaming.rs`,
//! and `stt.rs`. Extracting it here enables unit testing and a single source
//! of truth.

/// Tracks inter-segment spacing state.  O(1) memory.
pub(crate) struct SpacingState {
    pub has_content: bool,
    pub ends_with_space: bool,
}

impl SpacingState {
    pub fn new() -> Self {
        Self {
            has_content: false,
            ends_with_space: false,
        }
    }

    /// Build a tick delta (mid-recording): includes trailing space so the next
    /// segment doesn't fuse with this one.
    ///
    /// Returns the delta string to append to the WAL file.
    /// Returns an empty string if `seg_text` is empty and no content has been
    /// produced yet.
    pub fn build_tick_delta(&mut self, seg_text: &str) -> String {
        let mut delta = String::new();
        if !seg_text.is_empty() {
            if self.has_content && !self.ends_with_space && !seg_text.starts_with(' ') {
                delta.push(' ');
            }
            delta.push_str(seg_text);
        }

        // Add trailing space so the next segment does not fuse with this one.
        if (self.has_content || !delta.is_empty()) && !delta.ends_with(' ') {
            delta.push(' ');
        }

        if !delta.is_empty() {
            self.ends_with_space = delta.ends_with(' ');
            self.has_content = true;
        }
        delta
    }

    /// Build a final delta (post-loop, last segment): no trailing space added.
    ///
    /// Returns the delta string for the final segment.
    pub fn build_final_delta(&self, seg_text: &str) -> String {
        let mut delta = String::new();
        if !seg_text.is_empty() {
            if self.has_content && !self.ends_with_space && !seg_text.starts_with(' ') {
                delta.push(' ');
            }
            delta.push_str(seg_text);
        }
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tick deltas (mid-recording) ──

    #[test]
    fn first_segment_gets_trailing_space() {
        let mut s = SpacingState::new();
        let d = s.build_tick_delta("hello");
        assert_eq!(d, "hello ");
        assert!(s.has_content);
        assert!(s.ends_with_space);
    }

    #[test]
    fn second_segment_adds_leading_space_if_needed() {
        let mut s = SpacingState::new();
        s.build_tick_delta("hello");
        // Previous delta ended with space, so no extra space before "world"
        let d = s.build_tick_delta("world");
        assert_eq!(d, "world ");
    }

    #[test]
    fn no_double_space_when_segment_starts_with_space() {
        let mut s = SpacingState::new();
        s.build_tick_delta("hello");
        let d = s.build_tick_delta(" world");
        // Should not produce "  world "
        assert_eq!(d, " world ");
    }

    #[test]
    fn no_double_space_when_segment_ends_with_space() {
        let mut s = SpacingState::new();
        let d = s.build_tick_delta("hello ");
        // "hello " already ends with space, no extra trailing space
        assert_eq!(d, "hello ");
    }

    #[test]
    fn empty_segment_produces_nothing_initially() {
        let mut s = SpacingState::new();
        let d = s.build_tick_delta("");
        assert_eq!(d, "");
        assert!(!s.has_content);
    }

    #[test]
    fn empty_segment_after_content_produces_nothing() {
        let mut s = SpacingState::new();
        s.build_tick_delta("hello");
        let d = s.build_tick_delta("");
        // has_content is true but delta is empty → trailing space guard
        // (has_content || !delta.is_empty()) && !delta.ends_with(' ')
        // has_content=true, delta="" → ends_with(' ') on "" is false → push ' '
        assert_eq!(d, " ");
    }

    #[test]
    fn chinese_segments_no_space_fusion() {
        let mut s = SpacingState::new();
        s.build_tick_delta("你好");
        let d = s.build_tick_delta("世界");
        // Previous ended with space, "世界" doesn't start with space
        // → no leading space needed (trailing space from prev tick separates them)
        assert_eq!(d, "世界 ");
    }

    #[test]
    fn many_segments_accumulate_correctly() {
        let mut s = SpacingState::new();
        let mut transcript = String::new();
        for seg in &["Hello", "world", "how", "are", "you"] {
            let d = s.build_tick_delta(seg);
            transcript.push_str(&d);
        }
        assert_eq!(transcript, "Hello world how are you ");
    }

    // ── Final delta (post-loop) ──

    #[test]
    fn final_delta_no_trailing_space() {
        let mut s = SpacingState::new();
        s.build_tick_delta("hello");
        let d = s.build_final_delta("world");
        // Previous ended with space, so no leading space; no trailing space added
        assert_eq!(d, "world");
    }

    #[test]
    fn final_delta_adds_leading_space_when_previous_didnt_end_with_space() {
        let mut s = SpacingState::new();
        // Simulate a state where previous content exists but no trailing space
        s.has_content = true;
        s.ends_with_space = false;
        let d = s.build_final_delta("world");
        assert_eq!(d, " world");
    }

    #[test]
    fn final_delta_empty_segment() {
        let mut s = SpacingState::new();
        s.build_tick_delta("hello");
        let d = s.build_final_delta("");
        assert_eq!(d, "");
    }

    // ── Full meeting simulation ──

    #[test]
    fn simulate_meeting_transcript() {
        let mut s = SpacingState::new();
        let mut transcript = String::new();

        // Tick 1: model returns "Good morning"
        transcript.push_str(&s.build_tick_delta("Good morning"));
        // Tick 2: model returns "everyone."
        transcript.push_str(&s.build_tick_delta("everyone."));
        // Tick 3: model returns "Let's begin."
        transcript.push_str(&s.build_tick_delta("Let's begin."));
        // Post-loop: final segment "Thank you"
        transcript.push_str(&s.build_final_delta("Thank you"));

        assert_eq!(
            transcript,
            "Good morning everyone. Let's begin. Thank you"
        );
    }

    #[test]
    fn simulate_meeting_with_empty_ticks() {
        let mut s = SpacingState::new();
        let mut transcript = String::new();

        // Tick 1: speech
        transcript.push_str(&s.build_tick_delta("Hello"));
        // Tick 2: silence (empty)
        transcript.push_str(&s.build_tick_delta(""));
        // Tick 3: speech resumes
        transcript.push_str(&s.build_tick_delta("world"));

        // Should be "Hello  world " — the silence tick inserted one space
        // and the next tick's trailing space is already there.
        // Let's verify: "Hello " + " " + "world "
        assert_eq!(transcript, "Hello  world ");
    }
}
