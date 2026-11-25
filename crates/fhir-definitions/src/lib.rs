#![forbid(unsafe_code)]
//! FHIR R5 definitions registry.
//!
//! Loads and indexes StructureDefinitions, ValueSets, CodeSystems and ConceptMaps from
//! the bundled FHIR R5 artifact files.

mod conformance;
mod input;
#[cfg(test)]
mod loaded_fields_tests;
mod loader;
mod model;
mod registry;
mod shared;
mod type_code;

pub use input::{MAX_INPUT_BYTES, read_capped};
pub use model::{
    AdditionalBinding, Binding, ChoiceValue, CodeSystem, Concept, ConceptMap, ConceptMapElement,
    ConceptMapGroup, ConceptMapTarget, Constraint, Discriminator, ElementDefinition, Include,
    Slicing, StructureDefinition, TypeProfiles, ValueSet,
};
pub use registry::{CORE_PREFIX, Registry};
pub use shared::{DEFINITIONS_DIR_ENV, SharedRegistryError};
pub use type_code::{capitalise, is_primitive_type};

use thiserror::Error;

/// Errors that can occur while loading FHIR definitions.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error in {file}: {source}")]
    Json {
        file: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("malformed FHIR definitions in {file}: {message}")]
    MalformedBundle { file: String, message: String },

    #[error("cannot read {file}: {source}")]
    Read {
        file: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{url} in {file} is already defined")]
    Conflict { url: String, file: String },

    #[error("unsupported definition in {file}: {message}")]
    Unsupported { file: String, message: String },

    #[error("{file}: {message}")]
    Limit { file: String, message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
