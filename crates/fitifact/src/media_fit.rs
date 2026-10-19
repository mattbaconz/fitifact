use crate::artifact::Artifact;
use crate::constraints::{ConstraintSet, ConstraintValue, Field, Operator};

pub const MEDIA_FIT_ATTEMPTS: u8 = 7;
const MUX_SAFETY: f64 = 0.90;
const ASSUMED_AUDIO_BPS: u64 = 160_000;
const MIN_VIDEO_BPS: u64 = 80_000;
const MIN_EDGE: u32 = 160;
const SCALE_STEPS: u8 = 8;

/// The raw byte ceiling, folding any base64 ceiling into its raw equivalent.
pub fn file_bytes_limit(constraints: &ConstraintSet) -> Option<u64> {
    constraints
        .hard
        .iter()
        .filter(|constraint| constraint.op == Operator::Lte)
        .filter_map(|constraint| match (constraint.field, &constraint.value) {
            (Field::FileBytes, ConstraintValue::Integer(value)) => Some(*value),
            // N base64 characters hold at most 3 * floor(N / 4) raw bytes.
            (Field::FileBase64Bytes, ConstraintValue::Integer(value)) => Some(value / 4 * 3),
            _ => None,
        })
        .min()
}

pub fn file_bytes_floor(constraints: &ConstraintSet) -> Option<u64> {
    constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == Field::FileBytes && constraint.op == Operator::Gte)
        .filter_map(|constraint| match constraint.value {
            ConstraintValue::Integer(value) => Some(value),
            _ => None,
        })
        .max()
}

pub fn dimension_limit(constraints: &ConstraintSet, field: Field) -> Option<u32> {
    constraints
        .hard
        .iter()
        .filter(|constraint| constraint.field == field && constraint.op == Operator::Lte)
        .filter_map(|constraint| match constraint.value {
            ConstraintValue::Integer(value) => u32::try_from(value).ok(),
            _ => None,
        })
        .min()
}

pub fn even_dimension(value: u32) -> u32 {
    value.saturating_sub(value % 2).max(2)
}

pub fn scale_to_limits(
    width: u32,
    height: u32,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> (u32, u32) {
    let mut width = f64::from(width);
    let mut height = f64::from(height);
    if let Some(max_width) = max_width {
        let max_width = f64::from(max_width.max(2));
        if width > max_width {
            height *= max_width / width;
            width = max_width;
        }
    }
    if let Some(max_height) = max_height {
        let max_height = f64::from(max_height.max(2));
        if height > max_height {
            width *= max_height / height;
            height = max_height;
        }
    }
    (
        even_dimension(width.round().max(2.0) as u32),
        even_dimension(height.round().max(2.0) as u32),
    )
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

pub fn can_fit_at(
    width: u32,
    height: u32,
    max_bytes: u64,
    duration_ms: u64,
    has_audio: bool,
) -> bool {
    video_bitrate_from_budget(max_bytes, duration_ms, has_audio)
        .is_some_and(|bitrate| bitrate >= floor_video_bitrate_bps(width, height))
}

pub fn fit_output_size(
    width: u32,
    height: u32,
    max_width: Option<u32>,
    max_height: Option<u32>,
    max_bytes: Option<u64>,
    duration_ms: Option<u64>,
    has_audio: bool,
) -> Option<(u32, u32)> {
    let (mut width, mut height) = scale_to_limits(width, height, max_width, max_height);
    let Some(max_bytes) = max_bytes else {
        return Some((width, height));
    };
    let duration_ms = duration_ms?;
    for _ in 0..SCALE_STEPS {
        if can_fit_at(width, height, max_bytes, duration_ms, has_audio) {
            return Some((width, height));
        }
        let next_width = even_dimension(((width as f64) * 0.75) as u32);
        let next_height = even_dimension(((height as f64) * 0.75) as u32);
        if next_width < MIN_EDGE
            || next_height < MIN_EDGE
            || (next_width == width && next_height == height)
        {
            break;
        }
        width = next_width;
        height = next_height;
    }
    can_fit_at(width, height, max_bytes, duration_ms, has_audio).then_some((width, height))
}

pub fn can_fit_media(artifact: &Artifact, max_bytes: u64) -> bool {
    can_fit_media_with_limits(artifact, max_bytes, None, None)
}

pub fn can_fit_media_with_limits(
    artifact: &Artifact,
    max_bytes: u64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> bool {
    let Some(video) = artifact.first_video() else {
        return artifact
            .duration_ms
            .is_some_and(|duration| can_fit_audio(duration, max_bytes));
    };
    let (Some(width), Some(height)) = (video.width, video.height) else {
        return false;
    };
    fit_output_size(
        width,
        height,
        max_width,
        max_height,
        Some(max_bytes),
        artifact.duration_ms,
        artifact.first_audio().is_some(),
    )
    .is_some()
}

/// The video bitrates a size-fitting encode tries in order: what the byte budget allows (never
/// more than `cap`), then a quarter less per attempt down to the pixel-based quality floor.
/// `None` when even the floor would overflow the budget.
pub fn video_bitrate_ladder(
    max_bytes: u64,
    duration_ms: u64,
    has_audio: bool,
    (width, height): (u32, u32),
    cap: Option<u64>,
) -> Option<Vec<u64>> {
    let budget = video_bitrate_from_budget(max_bytes, duration_ms, has_audio)?;
    let initial = cap.map_or(budget, |cap| budget.min(cap));
    let floor = floor_video_bitrate_bps(width, height);
    (initial >= floor).then(|| bitrate_ladder(initial, floor))
}

/// What an encoder without a constant-quality mode (WebCodecs H.264) spends when no size limit
/// forces less: about 0.1 bit per pixel at 30 fps, near what x264 spends at CRF 23.
// ponytail: one bits-per-pixel guess for all footage; measure against x264 CRF 23 if
// browser encodes look worse than desktop ones.
pub fn quality_video_bitrate_bps(width: u32, height: u32) -> u64 {
    (u64::from(width) * u64::from(height) * 3).max(floor_video_bitrate_bps(width, height))
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

pub const MIN_AUDIO_BPS: u64 = 32_000;
pub const DEFAULT_AUDIO_BPS: u64 = 160_000;

pub fn audio_bitrate_ladder() -> &'static [u64] {
    &[160_000, 128_000, 96_000, 64_000, 48_000, 32_000]
}

pub fn estimated_audio_bytes(duration_ms: u64, bitrate_bps: u64) -> u64 {
    let secs = duration_ms as f64 / 1000.0;
    (secs * bitrate_bps as f64 / 8.0 / MUX_SAFETY).ceil() as u64
}

pub fn can_fit_audio(duration_ms: u64, max_bytes: u64) -> bool {
    estimated_audio_bytes(duration_ms, MIN_AUDIO_BPS) <= max_bytes
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

    #[test]
    fn a_cap_starts_the_ladder_lower_but_keeps_the_same_floor() {
        // 1 MB over 10 s with audio leaves 720 kb/s minus the assumed 160 kb/s soundtrack.
        let uncapped = video_bitrate_ladder(1_000_000, 10_000, true, (640, 360), None).unwrap();
        assert_eq!(uncapped[0], 560_000);
        let capped =
            video_bitrate_ladder(1_000_000, 10_000, true, (640, 360), Some(300_000)).unwrap();
        assert_eq!(capped[..2], [300_000, 225_000]);
        assert_eq!(capped.last(), Some(&MIN_VIDEO_BPS));
        assert_eq!(uncapped.last(), Some(&MIN_VIDEO_BPS));
        assert!(video_bitrate_ladder(10_000, 10_000, true, (640, 360), None).is_none());
    }

    #[test]
    fn the_quality_bitrate_scales_with_pixels_above_the_floor() {
        assert_eq!(quality_video_bitrate_bps(1920, 1080), 6_220_800);
        assert_eq!(quality_video_bitrate_bps(16, 16), MIN_VIDEO_BPS);
    }

    #[test]
    fn even_dimension_never_returns_odd_or_zero() {
        assert_eq!(even_dimension(1920), 1920);
        assert_eq!(even_dimension(1921), 1920);
        assert_eq!(even_dimension(1), 2);
    }

    #[test]
    fn scale_to_limits_keeps_aspect_and_even_edges() {
        let (width, height) = scale_to_limits(1920, 1080, Some(1000), None);
        assert_eq!(width, 1000);
        assert_eq!(height, 562);
        assert_eq!(width % 2, 0);
        assert_eq!(height % 2, 0);
    }

    #[test]
    fn fit_output_size_scales_when_the_current_frame_cannot_hold_the_budget() {
        let fitted = fit_output_size(1920, 1080, None, None, Some(40_000), Some(1_000), true);
        let (width, height) = fitted.expect("40 KB / 1s should fit after scaling");
        assert!(!can_fit_at(1920, 1080, 40_000, 1_000, true));
        assert_eq!((width, height), (1440, 810));
        assert!(can_fit_at(width, height, 40_000, 1_000, true));
    }

    #[test]
    fn three_second_audio_fits_a_typical_upload_ceiling() {
        assert!(can_fit_audio(3_000, 40_000));
        assert!(!can_fit_audio(3_000, 2_000));
    }

    #[test]
    fn file_bytes_limit_folds_a_base64_ceiling() {
        let constraints = compile(ConstraintInput {
            max_bytes: Some(2_000_000),
            max_base64_bytes: Some(2_000_003),
            ..ConstraintInput::default()
        })
        .unwrap();
        assert_eq!(file_bytes_limit(&constraints), Some(1_500_000));
    }
}
