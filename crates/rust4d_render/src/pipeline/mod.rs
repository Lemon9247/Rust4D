//! Rendering pipeline components
//!
//! This module contains the compute and render pipelines for
//! 4D cross-section rendering.

pub mod lookup_tables;
pub mod types;
pub mod slice_pipeline;
pub mod render_pipeline;

// Re-export lookup tables (tetrahedra tables only)
pub use lookup_tables::{
    TETRA_EDGES, TETRA_EDGE_TABLE, TETRA_TRI_TABLE, TETRA_TRI_COUNT,
    tetra_edge_count, tetra_crossed_edges,
};

// Re-export types
pub use types::{
    Vertex4D, Vertex3D, SliceParams, RenderUniforms,
    AtomicCounter, GpuTetrahedron, MAX_OUTPUT_TRIANGLES, TRIANGLE_VERTEX_COUNT,
};

// Re-export pipelines
pub use slice_pipeline::SlicePipeline;
pub use render_pipeline::{RenderPipeline, DrawIndirectArgs, perspective_matrix, look_at_matrix, mat4_mul};
