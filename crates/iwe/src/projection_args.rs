use liwe::query::cli::parse_projection;
use liwe::query::{Projection, ProjectionMode};
use serde_yaml::Mapping;

pub fn parse_projection_replace(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionMode::Replace)
}

pub fn parse_projection_extend(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionMode::Extend)
}

pub fn unused_warn() -> Mapping {
    Mapping::new()
}
