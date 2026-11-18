use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Window {
    pub start: usize,
    pub end: usize,
}

/// Bilingual routing uses actual Whisper posteriors, not transcript script.
/// Starter gates, NOT calibrated confidence: at least 5% joint candidate mass,
/// and 80% of that mass for the selected language. Others remain unknown.
pub fn select_language(telugu: f32, english: f32) -> Option<&'static str> {
    if [telugu, english]
        .iter()
        .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
    {
        return None;
    }
    let support = telugu + english;
    if support < 0.05 || telugu.max(english) / support < 0.8 {
        return None;
    }
    Some(if telugu > english { "te" } else { "en" })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bilingual_routing_rejects_ambiguity_and_low_support() {
        assert_eq!(select_language(0.3, 0.01), Some("te"));
        assert_eq!(select_language(0.01, 0.8), Some("en"));
        assert_eq!(select_language(0.3, 0.3), None);
        assert_eq!(select_language(0.01, 0.001), None);
        assert_eq!(select_language(f32::NAN, 0.9), None);
    }
}
