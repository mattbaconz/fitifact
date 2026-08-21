use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

macro_rules! contract_schema {
    ($type:ident, $constant:ident, $value:literal) => {
        pub const $constant: &str = $value;

        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        pub struct $type;

        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str($constant)
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                if value == $constant {
                    Ok(Self)
                } else {
                    Err(D::Error::custom(format!(
                        "expected schema {}, found {value}",
                        $constant
                    )))
                }
            }
        }
    };
}

contract_schema!(
    ConstraintsSchema,
    CONSTRAINTS_SCHEMA,
    "fitifact.constraints/v1"
);
contract_schema!(ArtifactSchema, ARTIFACT_SCHEMA, "fitifact.artifact/v1");
contract_schema!(CheckSchema, CHECK_SCHEMA, "fitifact.check/v1");
contract_schema!(PlanSchema, PLAN_SCHEMA, "fitifact.plan/v1");
contract_schema!(
    ImageAdaptPlanSchema,
    IMAGE_ADAPT_PLAN_SCHEMA,
    "fitifact.image-adapt-plan/v1"
);
contract_schema!(
    AdaptationSchema,
    ADAPTATION_SCHEMA,
    "fitifact.adaptation/v1"
);
contract_schema!(ErrorSchema, ERROR_SCHEMA, "fitifact.error/v1");
contract_schema!(DoctorSchema, DOCTOR_SCHEMA, "fitifact.doctor/v1");
contract_schema!(BenchSchema, BENCH_SCHEMA, "fitifact.bench/v1");
contract_schema!(
    RequirementsSchema,
    REQUIREMENTS_SCHEMA,
    "fitifact.requirements/v1"
);
