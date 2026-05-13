pub mod generate;
pub mod plan;

pub struct MigrationStep {
    pub step: i32,
    pub operation: String,
    pub sql: String,
    pub target: Option<String>,
}
