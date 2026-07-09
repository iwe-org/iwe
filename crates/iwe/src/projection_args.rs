use liwe::query::cli::parse_projection;
use liwe::query::{Projection, ProjectionBase};
use serde_yaml::Mapping;

pub fn parse_projection_replace(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionBase::Empty)
}

pub fn parse_projection_extend(s: &str) -> Result<Projection, String> {
    parse_projection(s, ProjectionBase::Document)
}

pub fn unused_warn() -> Mapping {
    Mapping::new()
}
