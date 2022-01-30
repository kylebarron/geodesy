pub mod inner_op;
pub mod op;
pub mod op_descriptor;
pub mod parsed_parameters;

pub mod etc;
pub mod parameter;
pub mod provider;
pub mod raw_parameters;

use log::error;
use std::io;
use thiserror::Error;

/// Preamble for InnerOp modules (built-in or user defined)
pub mod inner_op_authoring {
    pub use log::error;
    pub use log::warn;

    pub use super::etc;
    pub use super::inner_op::InnerOp;
    pub use super::op::Op;
    pub use super::op::*;
    pub use super::op_descriptor::OpDescriptor;
    pub use super::parameter::OpParameter;
    pub use super::parsed_parameters::ParsedParameters;
    pub use super::provider::Minimal;
    pub use super::provider::Provider;
    pub use super::raw_parameters::RawParameters;
    pub use super::Direction;
    pub use super::Error;
    pub use geodesy::CoordinateTuple;
}

/// Preamble for crate-internal modules
pub(crate) mod internal {

    pub use std::collections::BTreeMap;
    pub use std::collections::BTreeSet;

    pub use log::error;
    pub use log::warn;
    pub use uuid::Uuid;

    pub use super::etc;
    pub use super::inner_op::InnerOp;
    pub use super::inner_op::OpConstructor;
    pub use super::op::Op;
    pub use super::op_descriptor::OpDescriptor;
    pub use super::parameter::OpParameter;
    pub use super::parsed_parameters::ParsedParameters;
    pub use super::provider::Minimal;
    pub use super::provider::Provider;
    pub use super::raw_parameters::RawParameters;
    pub use super::Direction;
    pub use super::Error;
    pub use geodesy::CoordinateTuple;
    pub use geodesy::Ellipsoid;
}

/// Preamble for external use
pub mod preamble {
    pub use super::op::Op;
    pub use super::provider::Minimal;
    pub use super::Direction;
    pub use super::Error;
    pub use geodesy::CoordinateTuple as C;
}

// pub use op::Op;

#[derive(Error, Debug)]
pub enum Error {
    #[error("i/o error")]
    Io(#[from] io::Error),

    #[error("error: {0}")]
    General(&'static str),

    #[error("syntax error: {0}")]
    Syntax(String),

    #[error("{0}: {1}")]
    Operator(&'static str, &'static str),

    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("{message:?} (expected {expected:?}, found {found:?})")]
    Unexpected {
        message: String,
        expected: String,
        found: String,
    },

    #[error("operator {0} not found{1}")]
    NotFound(String, String),

    #[error("recursion too deep for {0}, at {1}")]
    Recursion(String, String),

    #[error("missing required parameter {0}")]
    MissingParam(String),

    #[error("malformed value for parameter {0}: {1}")]
    BadParam(String, String),

    #[error("unknown error")]
    Unknown,
}

/// `Fwd`: Indicate that a two-way operator, function, or method,
/// should run in the *forward* direction.
/// `Inv`: Indicate that a two-way operator, function, or method,
/// should run in the *inverse* direction.
#[derive(Debug, PartialEq)]
pub enum Direction {
    Fwd,
    Inv,
}