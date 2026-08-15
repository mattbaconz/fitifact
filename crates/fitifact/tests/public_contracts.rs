use fitifact::artifact::{Artifact, AudioCodec, Container, VideoCodec};
use fitifact::check::check;
use fitifact::constraints::media_h264_mp4_aac;
use fitifact::contract::{
    ADAPTATION_SCHEMA, ARTIFACT_SCHEMA, CHECK_SCHEMA, CONSTRAINTS_SCHEMA, DOCTOR_SCHEMA,
    ERROR_SCHEMA, PLAN_SCHEMA,
};
use fitifact::doctor::{DoctorReport, DoctorTool};
use fitifact::error::{Error, ErrorCode, ErrorEnvelope};

#[test]
fn public_schema_constants_are_exact() {
    assert_eq!(CONSTRAINTS_SCHEMA, "fitifact.constraints/v1");
    assert_eq!(ARTIFACT_SCHEMA, "fitifact.artifact/v1");
    assert_eq!(CHECK_SCHEMA, "fitifact.check/v1");
    assert_eq!(PLAN_SCHEMA, "fitifact.plan/v1");
    assert_eq!(ADAPTATION_SCHEMA, "fitifact.adaptation/v1");
    assert_eq!(ERROR_SCHEMA, "fitifact.error/v1");
    assert_eq!(DOCTOR_SCHEMA, "fitifact.doctor/v1");
}

#[test]
fn check_results_emit_the_check_schema() {
    let artifact = Artifact::media(Container::Mp4, VideoCodec::H264, Some(AudioCodec::Aac), 100);
    let value = serde_json::to_value(check(&artifact, &media_h264_mp4_aac())).unwrap();
    assert_eq!(value["schema"], CHECK_SCHEMA);
}

#[test]
fn reusable_error_and_doctor_envelopes_emit_their_schemas() {
    let error = ErrorEnvelope::from(Error::new(ErrorCode::InputInvalid, "bad input"));
    assert_eq!(serde_json::to_value(error).unwrap()["schema"], ERROR_SCHEMA);

    let doctor = DoctorReport::new(vec![DoctorTool {
        name: "ffprobe".into(),
        available: true,
        version: Some("7.1".into()),
        detail: None,
    }]);
    assert_eq!(
        serde_json::to_value(doctor).unwrap()["schema"],
        DOCTOR_SCHEMA
    );
}
