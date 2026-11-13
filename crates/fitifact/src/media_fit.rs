use crate::artifact::{Artifact, AudioCodec, Rational};
use crate::constraints::{ConstraintSet, ConstraintValue, Field, Operator};

pub const MEDIA_FIT_ATTEMPTS: u8 = 7;
const MUX_SAFETY: f64 = 0.90;
const MIN_VIDEO_BPS: u64 = 80_000;
const MIN_EDGE: u32 = 160;
const SCALE_STEPS: u8 = 8;
/// The fewest bits one pixel of one frame may get.
///
/// Measured 2026-11-13 with this engine's x264 settings on 60 s of fast 1080p60 game footage:
/// five sizes from 480x270 to 1920x1080, 30 and 60 fps, at 283, 600, and 1,200 kb/s. At 0.002
/// the encoder sat at its coarsest quantiser, overshot the bitrate it was given, and frames
/// decoded as flat grey (VMAF under 7). From 0.005 to 0.018 the picture was mush. At each
/// bitrate the best-scoring size and frame rate spent 0.03 to 0.04. Sizes step down by 0.56x
/// in pixels, so a floor of 0.02 lands each plan between 0.02 and 0.036.
// ponytail: one floor for all footage, set on hard footage. Easy footage (a talking head, a
// screen recording) could keep a larger picture; a short trial encode per file would tell.
const FLOOR_BITS_PER_PIXEL: f64 = 0.02;
/// The frame rate the floor assumes when a probe reports none.
const ASSUMED_FPS: f64 = 30.0;
/// Sources at or above this rate can drop to about 30 fps when the picture would be starved.
const REDUCIBLE_FPS: f64 = 48.0;
/// The AAC bitrate the soundtrack is encoded at when the picture needs its bits.
pub const COMPACT_AUDIO_BPS: u64 = 96_000;

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

/// Frames per second, or the assumed rate when the probe reported none.
pub fn frames_per_second(rate: Option<Rational>) -> f64 {
    rate.filter(|rate| rate.numerator > 0 && rate.denominator > 0)
        .map_or(ASSUMED_FPS, |rate| {
            f64::from(rate.numerator) / f64::from(rate.denominator)
        })
}

/// The lower frame rate a starved encode may use: the source rate divided by a whole number, so
/// every kept frame is an original one, landing near 30 fps. `None` for sources under 48 fps.
pub fn reduced_frame_rate(rate: Option<Rational>) -> Option<Rational> {
    let fps = frames_per_second(rate);
    let rate = rate.filter(|_| fps >= REDUCIBLE_FPS)?;
    let divisor = (fps / 30.0).round() as u32;
    Rational::new(rate.numerator, rate.denominator.checked_mul(divisor)?)
}

/// The lowest video bitrate an encode of this picture may use. Under it the encoder runs out of
/// quantiser and the picture breaks down, however well the file fits.
pub fn floor_video_bitrate_bps(width: u32, height: u32, fps: f64) -> u64 {
    let pixels = f64::from(width) * f64::from(height);
    ((pixels * fps * FLOOR_BITS_PER_PIXEL) as u64).max(MIN_VIDEO_BPS)
}

/// What is left for video after the container's overhead and a soundtrack of `audio_bps`.
pub fn video_bitrate_from_budget(max_bytes: u64, duration_ms: u64, audio_bps: u64) -> Option<u64> {
    if duration_ms == 0 {
        return None;
    }
    let duration_s = duration_ms as f64 / 1000.0;
    let total_bps = (max_bytes as f64) * 8.0 / duration_s * MUX_SAFETY;
    let audio = audio_bps as f64;
    if total_bps <= audio {
        return None;
    }
    Some((total_bps - audio) as u64)
}

/// The picture and soundtrack a size-fitting encode starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitSource {
    pub width: u32,
    pub height: u32,
    pub frame_rate: Option<Rational>,
    /// What the soundtrack costs if the plan leaves it as it is ([`kept_audio_bps`]); `None`
    /// without sound.
    pub audio_bps: Option<u64>,
}

/// The measured bitrate of a soundtrack an encode copies (AAC). `None` when the probe reported
/// none, or when the sound is not AAC and gets encoded anyway.
pub fn copied_audio_bps(artifact: &Artifact) -> Option<u64> {
    artifact
        .first_audio()
        .filter(|audio| audio.codec == Some(AudioCodec::Aac))
        .and_then(|audio| audio.bitrate_bps)
        .filter(|bitrate| *bitrate > 0)
}

/// What the soundtrack costs if the plan leaves it as it is: its measured bitrate when it is
/// copied, else the default encode rate. `None` without sound.
pub fn kept_audio_bps(artifact: &Artifact) -> Option<u64> {
    artifact.first_audio()?;
    Some(copied_audio_bps(artifact).unwrap_or(DEFAULT_AUDIO_BPS))
}

/// What a size-fitting encode will produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaFit {
    pub width: u32,
    pub height: u32,
    /// The lowered frame rate; `None` keeps the source's.
    pub frame_rate: Option<Rational>,
    /// The bitrate the soundtrack is encoded at to leave room for the picture; `None` keeps it
    /// as it would otherwise be (copied, or encoded at the default rate).
    pub audio_bps: Option<u64>,
    /// The lowest video bitrate the encode may use; `None` without a byte limit.
    pub video_floor_bps: Option<u64>,
}

/// Fits the picture to the dimension limits, then to the byte limit. When the byte budget cannot
/// hold the picture above its quality floor, the sound gives up bitrate first, then the frame
/// rate drops toward 30 fps, then the picture shrinks a quarter at a time. The sound is encoded
/// smaller only when that is what lets the picture keep a frame rate or a size: a soundtrack that
/// fits beside the picture finally chosen, or is already at the compact rate, is left alone.
/// `None` when even the smallest picture would fall under the floor.
pub fn fit_output(
    source: FitSource,
    max_width: Option<u32>,
    max_height: Option<u32>,
    max_bytes: Option<u64>,
    duration_ms: Option<u64>,
) -> Option<MediaFit> {
    let (mut width, mut height) =
        scale_to_limits(source.width, source.height, max_width, max_height);
    let Some(max_bytes) = max_bytes else {
        return Some(MediaFit {
            width,
            height,
            frame_rate: None,
            audio_bps: None,
            video_floor_bps: None,
        });
    };
    let duration_ms = duration_ms?;
    let fits = |width, height, frame_rate: Option<Rational>, audio_bps: Option<u64>| {
        let floor = floor_video_bitrate_bps(
            width,
            height,
            frames_per_second(frame_rate.or(source.frame_rate)),
        );
        let sound = audio_bps.or(source.audio_bps).unwrap_or(0);
        video_bitrate_from_budget(max_bytes, duration_ms, sound)
            .filter(|budget| *budget >= floor)
            .map(|_| MediaFit {
                width,
                height,
                frame_rate,
                audio_bps,
                video_floor_bps: Some(floor),
            })
    };
    let compact = source
        .audio_bps
        .filter(|kept| *kept > COMPACT_AUDIO_BPS)
        .map(|_| COMPACT_AUDIO_BPS);
    let reduced = reduced_frame_rate(source.frame_rate);
    for _ in 0..=SCALE_STEPS {
        // Without a lower rate or smaller sound to try, a candidate repeats the one before it.
        for frame_rate in [None, reduced] {
            for audio_bps in [None, compact] {
                let fitted = fits(width, height, frame_rate, audio_bps);
                if fitted.is_some() {
                    return fitted;
                }
            }
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
    None
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
    fit_output(
        FitSource {
            width,
            height,
            frame_rate: video.frame_rate,
            audio_bps: kept_audio_bps(artifact),
        },
        max_width,
        max_height,
        Some(max_bytes),
        artifact.duration_ms,
    )
    .is_some()
}

/// The video bitrates a size-fitting encode tries in order: what the byte budget leaves after a
/// soundtrack of `audio_bps` (never more than `cap`), then a quarter less per attempt down to
/// `floor`, the plan's quality floor. `None` when even the floor would overflow the budget.
pub fn video_bitrate_ladder(
    max_bytes: u64,
    duration_ms: u64,
    audio_bps: u64,
    floor: u64,
    cap: Option<u64>,
) -> Option<Vec<u64>> {
    let budget = video_bitrate_from_budget(max_bytes, duration_ms, audio_bps)?;
    let initial = cap.map_or(budget, |cap| budget.min(cap.max(floor)));
    (initial >= floor).then(|| bitrate_ladder(initial, floor))
}

/// What an encoder without a constant-quality mode (WebCodecs H.264) spends when no size limit
/// forces less: about 0.1 bit per pixel at 30 fps, near what x264 spends at CRF 23.
// ponytail: one bits-per-pixel guess for all footage; measure against x264 CRF 23 if
// browser encodes look worse than desktop ones.
pub fn quality_video_bitrate_bps(width: u32, height: u32) -> u64 {
    (u64::from(width) * u64::from(height) * 3).max(MIN_VIDEO_BPS)
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

    fn rate(numerator: u32, denominator: u32) -> Option<Rational> {
        Rational::new(numerator, denominator)
    }

    fn source(width: u32, height: u32, frame_rate: Option<Rational>) -> FitSource {
        FitSource {
            width,
            height,
            frame_rate,
            audio_bps: Some(DEFAULT_AUDIO_BPS),
        }
    }

    /// The video budget a fit leaves, and the floor it promised to stay above.
    fn budget_and_floor(fit: MediaFit, max_bytes: u64, duration_ms: u64) -> (u64, u64) {
        let sound = fit.audio_bps.unwrap_or(DEFAULT_AUDIO_BPS);
        (
            video_bitrate_from_budget(max_bytes, duration_ms, sound).unwrap(),
            fit.video_floor_bps.unwrap(),
        )
    }

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
        // 1 MB over 10 s leaves 720 kb/s minus the 160 kb/s soundtrack.
        let ladder = |max_bytes, cap| {
            video_bitrate_ladder(max_bytes, 10_000, DEFAULT_AUDIO_BPS, 80_000, cap)
        };
        let uncapped = ladder(1_000_000, None).unwrap();
        assert_eq!(uncapped[0], 560_000);
        let capped = ladder(1_000_000, Some(300_000)).unwrap();
        assert_eq!(capped[..2], [300_000, 225_000]);
        assert_eq!(capped.last(), Some(&80_000));
        assert_eq!(uncapped.last(), Some(&80_000));
        assert!(ladder(10_000, None).is_none());
        // A cap under the floor never drags an attempt below the floor.
        assert_eq!(ladder(1_000_000, Some(50_000)).unwrap(), [80_000]);
    }

    #[test]
    fn a_ladder_never_goes_under_the_floor_the_plan_set() {
        // The 7-minute clip's old ladder ended at 129,600 b/s for 1080p60. With a real floor the
        // budget is simply refused at that size.
        let floor = floor_video_bitrate_bps(1920, 1080, 60.0);
        assert!(
            video_bitrate_ladder(20_000_000, 414_400, DEFAULT_AUDIO_BPS, floor, None).is_none()
        );
        let floor = floor_video_bitrate_bps(640, 360, 30.0);
        let ladder =
            video_bitrate_ladder(20_000_000, 414_400, COMPACT_AUDIO_BPS, floor, None).unwrap();
        assert!(ladder.iter().all(|bitrate| *bitrate >= floor), "{ladder:?}");
    }

    #[test]
    fn the_floor_grows_with_frame_rate_and_picture_size() {
        let at = floor_video_bitrate_bps;
        assert_eq!(at(1920, 1080, 60.0), 2 * at(1920, 1080, 30.0));
        assert!(at(1920, 1080, 30.0) > at(1280, 720, 30.0));
        assert_eq!(at(16, 16, 30.0), MIN_VIDEO_BPS);
        // The old floor let 1080p60 run at 129,600 b/s, where every frame came out grey or mush.
        assert!(at(1920, 1080, 60.0) > 1_000_000);
    }

    #[test]
    fn high_frame_rates_reduce_by_a_whole_divisor_toward_thirty() {
        for (from, to) in [
            (rate(60, 1), rate(60, 2)),
            (rate(60_000, 1001), rate(60_000, 2002)),
            (rate(50, 1), rate(50, 2)),
            (rate(120, 1), rate(120, 4)),
            (rate(144, 1), rate(144, 5)),
            (rate(48, 1), rate(48, 2)),
        ] {
            assert_eq!(reduced_frame_rate(from), to, "{from:?}");
        }
        for kept in [
            rate(30, 1),
            rate(30_000, 1001),
            rate(24, 1),
            rate(0, 1),
            None,
        ] {
            assert_eq!(reduced_frame_rate(kept), None, "{kept:?}");
        }
        assert_eq!(frames_per_second(None), 30.0);
        assert_eq!(frames_per_second(rate(0, 1)), 30.0);
    }

    #[test]
    fn a_roomy_budget_changes_nothing() {
        // 30 s of 1080p60 into 20 MB leaves about 4.6 Mb/s for video.
        let fit = fit_output(
            source(1920, 1080, rate(60, 1)),
            None,
            None,
            Some(20_000_000),
            Some(30_000),
        )
        .unwrap();
        assert_eq!((fit.width, fit.height), (1920, 1080));
        assert_eq!((fit.frame_rate, fit.audio_bps), (None, None));
    }

    #[test]
    fn a_seven_minute_1080p60_clip_gives_up_sound_frame_rate_and_size_to_fit_20_mb() {
        // The clip that came out grey: 414.4 s, 1920x1080 at 60 fps, AAC sound, 20 MB limit.
        let (max_bytes, duration_ms) = (20_000_000, 414_400);
        let fit = fit_output(
            source(1920, 1080, rate(60, 1)),
            None,
            None,
            Some(max_bytes),
            Some(duration_ms),
        )
        .expect("a smaller picture fits");
        assert_eq!(fit.audio_bps, Some(COMPACT_AUDIO_BPS));
        assert_eq!(fit.frame_rate, rate(60, 2));
        assert_eq!((fit.width, fit.height), (810, 454));
        let (budget, floor) = budget_and_floor(fit, max_bytes, duration_ms);
        // 251 kb/s of video where the old plan gave 1080p60 187 kb/s.
        assert_eq!((budget, floor), (251_490, 220_644));
        assert!(budget >= floor, "{budget} under {floor}");
        assert_eq!(
            floor,
            floor_video_bitrate_bps(fit.width, fit.height, 30.0),
            "the floor is the one for the picture actually planned"
        );
    }

    #[test]
    fn the_frame_rate_drops_before_the_picture_shrinks_and_sound_that_fits_is_kept() {
        // 100 s of 1080p60 into 20 MB: too little for 60 fps even with smaller sound, enough for
        // 30 fps beside the sound as it is.
        let fit = fit_output(
            source(1920, 1080, rate(60, 1)),
            None,
            None,
            Some(20_000_000),
            Some(100_000),
        )
        .unwrap();
        assert_eq!((fit.width, fit.height), (1920, 1080));
        assert_eq!(fit.frame_rate, rate(60, 2));
        assert_eq!(fit.audio_bps, None);
    }

    #[test]
    fn smaller_sound_is_tried_before_a_lower_frame_rate() {
        // 55 s of 1080p60 into 20 MB: 2.46 Mb/s beside 160 kb/s sound is just under the 2.49 Mb/s
        // floor, and 2.52 Mb/s beside 96 kb/s sound is over it.
        let fit = fit_output(
            source(1920, 1080, rate(60, 1)),
            None,
            None,
            Some(20_000_000),
            Some(55_000),
        )
        .unwrap();
        assert_eq!((fit.width, fit.height, fit.frame_rate), (1920, 1080, None));
        assert_eq!(fit.audio_bps, Some(COMPACT_AUDIO_BPS));
    }

    #[test]
    fn sound_counts_at_its_measured_bitrate() {
        let fit = |audio_bps, duration_ms| {
            fit_output(
                FitSource {
                    audio_bps: Some(audio_bps),
                    ..source(1920, 1080, rate(60, 1))
                },
                None,
                None,
                Some(20_000_000),
                Some(duration_ms),
            )
            .unwrap()
        };
        // 54 s: sound at 160 kb/s leaves the picture enough, sound at 320 kb/s does not.
        assert_eq!(fit(160_000, 54_000).audio_bps, None);
        assert_eq!(fit(320_000, 54_000).audio_bps, Some(COMPACT_AUDIO_BPS));
        // Sound already at or under the compact rate is never encoded again: the 7-minute clip
        // with 48 kb/s sound keeps it and has 299 kb/s for the picture.
        for small in [48_000, COMPACT_AUDIO_BPS] {
            let fit = fit(small, 414_400);
            assert_eq!((fit.width, fit.height), (810, 454), "{small}");
            assert_eq!((fit.frame_rate, fit.audio_bps), (rate(60, 2), None));
        }
    }

    #[test]
    fn only_copied_aac_counts_at_its_measured_bitrate() {
        let mut artifact = crate::artifact::Artifact::media(
            Container::Mp4,
            VideoCodec::H264,
            Some(AudioCodec::Aac),
            5_000_000,
        );
        let sound = |artifact: &mut crate::artifact::Artifact, codec, bitrate_bps| {
            let audio = artifact.first_audio_mut().unwrap();
            audio.codec = Some(codec);
            audio.bitrate_bps = bitrate_bps;
            kept_audio_bps(artifact)
        };
        assert_eq!(
            sound(&mut artifact, AudioCodec::Aac, Some(189_000)),
            Some(189_000)
        );
        assert_eq!(
            sound(&mut artifact, AudioCodec::Aac, None),
            Some(DEFAULT_AUDIO_BPS)
        );
        assert_eq!(
            sound(&mut artifact, AudioCodec::Aac, Some(0)),
            Some(DEFAULT_AUDIO_BPS)
        );
        // Opus is encoded to AAC at the default rate, whatever it measured.
        assert_eq!(
            sound(&mut artifact, AudioCodec::Opus, Some(64_000)),
            Some(DEFAULT_AUDIO_BPS)
        );
        let silent = crate::artifact::Artifact::media(Container::Mp4, VideoCodec::H264, None, 1);
        assert_eq!(kept_audio_bps(&silent), None);
    }

    #[test]
    fn a_clip_too_long_for_any_watchable_picture_is_refused() {
        // 30 minutes into 20 MB leaves 80 kb/s for everything.
        assert_eq!(
            fit_output(
                source(1920, 1080, rate(60, 1)),
                None,
                None,
                Some(20_000_000),
                Some(1_800_000),
            ),
            None
        );
    }

    #[test]
    fn silent_video_has_no_sound_to_give_up() {
        let fit = fit_output(
            FitSource {
                audio_bps: None,
                ..source(1920, 1080, rate(30, 1))
            },
            None,
            None,
            Some(20_000_000),
            Some(414_400),
        )
        .unwrap();
        // All 347 kb/s goes to the picture, still short of the 393 kb/s that 1080x606 needs.
        assert_eq!((fit.width, fit.height, fit.audio_bps), (810, 454, None));
        assert_eq!(fit.frame_rate, None);
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
    fn fit_output_scales_when_the_current_frame_cannot_hold_the_budget() {
        // 40 KB over 1 s leaves 288 kb/s in all: 128 kb/s of video beside the sound as it is.
        let fit = fit_output(
            source(1920, 1080, rate(30, 1)),
            None,
            None,
            Some(40_000),
            Some(1_000),
        )
        .expect("40 KB / 1s should fit after scaling");
        // Each step keeps three quarters of each edge, rounded down to even: 1440x810,
        // 1080x606, 810x454 (too much even with smaller sound), then the first that fits.
        assert_eq!((fit.width, fit.height), (606, 340));
        assert_eq!(fit.audio_bps, None);
        let (budget, floor) = budget_and_floor(fit, 40_000, 1_000);
        assert!(budget >= floor);
    }

    #[test]
    fn without_a_byte_limit_only_the_dimension_limits_apply() {
        let fit = fit_output(
            source(1920, 1080, rate(60, 1)),
            Some(1000),
            None,
            None,
            None,
        );
        assert_eq!(
            fit,
            Some(MediaFit {
                width: 1000,
                height: 562,
                frame_rate: None,
                audio_bps: None,
                video_floor_bps: None,
            })
        );
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
