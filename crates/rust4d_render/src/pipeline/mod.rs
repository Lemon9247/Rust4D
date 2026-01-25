//! Rendering pipeline components
//!
//! This module contains the compute and render pipelines for
//! 4D cross-section rendering.

pub mod lookup_tables;
pub mod types;
pub mod slice_pipeline;
pub mod render_pipeline;

// Re-export lookup tables
pub use lookup_tables::{EDGES, EDGE_TABLE, TRI_TABLE};

// Re-export types
pub use types::{
    Vertex4D, Simplex4D, Vertex3D, SliceParams, RenderUniforms,
    AtomicCounter, MAX_OUTPUT_TRIANGLES, TRIANGLE_VERTEX_COUNT,
};

// Re-export pipelines
pub use slice_pipeline::SlicePipeline;
pub use render_pipeline::{RenderPipeline, DrawIndirectArgs, perspective_matrix, look_at_matrix, mat4_mul};
