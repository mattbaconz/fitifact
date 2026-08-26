use crate::artifact::Artifact;
use crate::constraints::{ConstraintSet, ConstraintValue, Field, Operator};

pub const MEDIA_FIT_ATTEMPTS: u8 = 7;
const MUX_SAFETY: f64 = 0.90;
const ASSUMED_AUDIO_BPS: u64 = 160_000;
const MIN_VIDEO_BPS: u64 = 80_000;

pub fn file_bytes_limit(constraints: &ConstraintSet) -> Option<u64> {
    constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == Field::FileBytes && constraint.op == Operator::Lte)
        .filter_map(|constraint| match constraint.value {
            ConstraintValue::Integer(value) => Some(value),
            _ => None,
        })
        .min()
}

pub fn floor_video_bitrate_bps(width: u32, height: u32) -> u64 {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    (pixels / 16).max(MIN_VIDEO_BPS)
}

pub fn video_bitrate_from_budget(max_bytes: u64, duration_ms: u64, has_audio: bool) -> Option<u64> {
    if duration_ms == 0 {
        return None;
    }
    let duration_s = duration_ms as f64 / 1000.0;
    let total_bps = (max_bytes as f64) * 8.0 / duration_s * MUX_SAFETY;
    let audio = if has_audio {
        ASSUMED_AUDIO_BPS as f64
    } else {
        0.0
    };
    if total_bps <= audio {
        return None;
    }
    Some((total_bps - audio) as u64)
}

pub fn can_fit_media(artifact: &Artifact, max_bytes: u64) -> bool {
    let Some(duration_ms) = artifact.duration_ms else {
        return false;
    };
    let Some(video) = artifact.first_video() else {
        return false;
    };
    let (Some(width), Some(height)) = (video.width, video.height) else {
        return false;
    };
    let Some(bitrate) =
        video_bitrate_from_budget(max_bytes, duration_ms, artifact.first_audio().is_some())
    else {
        return false;
    };
    bitrate >= floor_video_bitrate_bps(width, height)
}

pub fn bitrate_ladder(initial: u64, floor: u64) -> Vec<u64> {
    let mut values = Vec::new();
    let mut current = initial;
    for _ in 0..MEDIA_FIT_ATTEMPTS {
        if current < floor {
            break;
        }
        values.push(current);
        current = ((current as f64) * 0.75) as u64;
    }
    if floor <= initial && values.last().copied() != Some(floor) {
        values.push(floor);
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{AudioCodec, Container, VideoCodec};
    use crate::constraints::{ConstraintInput, compile};

    #[test]
    fn one_second_1080p_cannot_fit_a_kilobyte_ceiling() {
        let artifact = crate::artifact::Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            5_000_000,
        );
        assert!(!can_fit_media(&artifact, 999));
    }

    #[test]
    fn two_megabyte_ceiling_is_plausible_for_one_second_1080p() {
        let artifact = crate::artifact::Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            5_000_000,
        );
        assert!(can_fit_media(&artifact, 2_000_000));
    }

    #[test]
    fn file_bytes_limit_uses_the_strictest_ceiling() {
        let constraints = compile(ConstraintInput {
            max_bytes: Some(2_000_000),
            ..ConstraintInput::default()
        })
        .unwrap();
        assert_eq!(file_bytes_limit(&constraints), Some(2_000_000));
    }

    #[test]
    fn bitrate_ladder_stops_at_the_floor() {
        let ladder = bitrate_ladder(1_000_000, 400_000);
        assert_eq!(ladder[0], 1_000_000);
        assert!(ladder.last().copied() == Some(400_000));
        assert!(ladder.len() as u8 <= MEDIA_FIT_ATTEMPTS + 1);
    }
}
