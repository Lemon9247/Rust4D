//! Compute pipeline for 4D cross-section slicing
//!
//! This pipeline takes 4D geometry and produces 3D triangles
//! by intersecting with a hyperplane at a given W coordinate.
//!
//! Supports two modes:
//! - Legacy: 5-cells (Simplex4D) with complex prism handling
//! - Tetrahedra: Simpler tetrahedra with at most 2 triangles each

use wgpu::util::DeviceExt;

use super::lookup_tables::{EDGE_TABLE, TRI_TABLE, EDGES};
use super::types::{
    Simplex4D, SliceParams, Vertex3D, Vertex4D, GpuTetrahedron, AtomicCounter,
    TRIANGLE_VERTEX_COUNT,
};

/// Compute pipeline for slicing 4D geometry
#[allow(dead_code)] // Fields hold GPU resources that must outlive bind groups
pub struct SlicePipeline {
    /// Maximum number of triangles this pipeline can output
    max_triangles: usize,

    // ===== Legacy 5-cell pipeline =====
    /// The compute pipeline (legacy 5-cell)
    pipeline: wgpu::ComputePipeline,
    /// Bind group layout for simplices + output + counter + params
    bind_group_layout_main: wgpu::BindGroupLayout,
    /// Bind group layout for lookup tables
    bind_group_layout_tables: wgpu::BindGroupLayout,
    /// Lookup table buffers (edge table and triangle table)
    edge_table_buffer: wgpu::Buffer,
    tri_table_buffer: wgpu::Buffer,
    edges_buffer: wgpu::Buffer,
    /// Bind group for lookup tables (created once)
    tables_bind_group: wgpu::BindGroup,
    /// Input simplex buffer (created per frame or when geometry changes)
    simplex_buffer: Option<wgpu::Buffer>,
    simplex_count: u32,
    /// Main bind group for legacy pipeline
    main_bind_group: Option<wgpu::BindGroup>,

    // ===== Tetrahedra pipeline =====
    /// The compute pipeline (tetrahedra)
    tetra_pipeline: wgpu::ComputePipeline,
    /// Bind group layout for tetrahedra pipeline
    tetra_bind_group_layout: wgpu::BindGroupLayout,
    /// Vertex buffer (4D vertices)
    vertex_buffer: Option<wgpu::Buffer>,
    /// Tetrahedra buffer (indices into vertex buffer)
    tetra_buffer: Option<wgpu::Buffer>,
    tetra_count: u32,
    /// Bind group for tetrahedra pipeline
    tetra_bind_group: Option<wgpu::BindGroup>,

    // ===== Shared resources =====
    /// Output buffer for triangles
    output_buffer: wgpu::Buffer,
    /// Atomic counter buffer for triangle count
    counter_buffer: wgpu::Buffer,
    /// Staging buffer for reading counter back to CPU
    counter_staging_buffer: wgpu::Buffer,
    /// Slice parameters uniform buffer
    params_buffer: wgpu::Buffer,
    /// Whether to use tetrahedra pipeline (true) or legacy (false)
    use_tetrahedra: bool,
}

impl SlicePipeline {
    /// Create a new slice pipeline with the specified maximum triangle capacity
    ///
    /// # Arguments
    /// * `device` - The wgpu device
    /// * `max_triangles` - Maximum number of triangles to allocate buffer space for.
    ///   Each triangle requires 3 vertices × 48 bytes = 144 bytes.
    ///   Default in config is 1,000,000 triangles (~144 MB GPU memory).
    pub fn new(device: &wgpu::Device, max_triangles: usize) -> Self {
        // ===== Legacy 5-cell bind group layout =====
        let bind_group_layout_main = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Slice Main Bind Group Layout"),
            entries: &[
                // Simplices input buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output triangles buffer (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Atomic counter buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Slice parameters uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // ===== Tetrahedra bind group layout =====
        let tetra_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tetra Bind Group Layout"),
            entries: &[
                // Vertices buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Tetrahedra buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output triangles buffer (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Atomic counter buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Slice parameters uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group_layout_tables = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Slice Tables Bind Group Layout"),
            entries: &[
                // Edge table
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Triangle table
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Edges definition
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Slice Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout_main, &bind_group_layout_tables],
            push_constant_ranges: &[],
        });

        // Load shader
        let shader_source = include_str!("../shaders/slice.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Slice Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create compute pipeline (legacy)
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Slice Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // ===== Create tetrahedra pipeline =====
        let tetra_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tetra Pipeline Layout"),
            bind_group_layouts: &[&tetra_bind_group_layout],
            push_constant_ranges: &[],
        });

        let tetra_shader_source = include_str!("../shaders/slice_tetra.wgsl");
        let tetra_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tetra Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(tetra_shader_source.into()),
        });

        let tetra_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Tetra Compute Pipeline"),
            layout: Some(&tetra_pipeline_layout),
            module: &tetra_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create lookup table buffers
        let edge_table_data: Vec<u32> = EDGE_TABLE.iter().map(|&x| x as u32).collect();
        let edge_table_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Table Buffer"),
            contents: bytemuck::cast_slice(&edge_table_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Flatten triangle table to i32 array
        let tri_table_data: Vec<i32> = TRI_TABLE.iter().flat_map(|row| row.iter().map(|&x| x as i32)).collect();
        let tri_table_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangle Table Buffer"),
            contents: bytemuck::cast_slice(&tri_table_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Edges definition buffer
        let edges_data: Vec<u32> = EDGES.iter().flat_map(|e| [e[0] as u32, e[1] as u32]).collect();
        let edges_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edges Buffer"),
            contents: bytemuck::cast_slice(&edges_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create tables bind group
        let tables_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Tables Bind Group"),
            layout: &bind_group_layout_tables,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: edge_table_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tri_table_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: edges_buffer.as_entire_binding(),
                },
            ],
        });

        // Create output buffer sized by max_triangles parameter
        let output_size = (max_triangles * TRIANGLE_VERTEX_COUNT * std::mem::size_of::<Vertex3D>()) as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Output Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create counter buffer
        let counter_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Counter Buffer"),
            size: std::mem::size_of::<AtomicCounter>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        // Create staging buffer for reading counter
        let counter_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Counter Staging Buffer"),
            size: std::mem::size_of::<AtomicCounter>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create params buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Slice Params Buffer"),
            size: std::mem::size_of::<SliceParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            max_triangles,

            // Legacy 5-cell pipeline
            pipeline,
            bind_group_layout_main,
            bind_group_layout_tables,
            edge_table_buffer,
            tri_table_buffer,
            edges_buffer,
            tables_bind_group,
            simplex_buffer: None,
            simplex_count: 0,
            main_bind_group: None,

            // Tetrahedra pipeline
            tetra_pipeline,
            tetra_bind_group_layout,
            vertex_buffer: None,
            tetra_buffer: None,
            tetra_count: 0,
            tetra_bind_group: None,

            // Shared resources
            output_buffer,
            counter_buffer,
            counter_staging_buffer,
            params_buffer,
            use_tetrahedra: true, // Default to tetrahedra mode
        }
    }

    /// Upload simplices to the GPU (legacy mode)
    pub fn upload_simplices(&mut self, device: &wgpu::Device, simplices: &[Simplex4D]) {
        self.use_tetrahedra = false;
        self.simplex_count = simplices.len() as u32;

        // Create new simplex buffer
        self.simplex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simplex Buffer"),
            contents: bytemuck::cast_slice(simplices),
            usage: wgpu::BufferUsages::STORAGE,
        }));

        // Recreate main bind group
        self.main_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Slice Main Bind Group"),
            layout: &self.bind_group_layout_main,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.simplex_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    /// Upload tetrahedra and vertices to the GPU (new mode)
    pub fn upload_tetrahedra(&mut self, device: &wgpu::Device, vertices: &[Vertex4D], tetrahedra: &[GpuTetrahedron]) {
        self.use_tetrahedra = true;
        self.tetra_count = tetrahedra.len() as u32;

        // Create vertex buffer
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::STORAGE,
        }));

        // Create tetrahedra buffer
        self.tetra_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tetrahedra Buffer"),
            contents: bytemuck::cast_slice(tetrahedra),
            usage: wgpu::BufferUsages::STORAGE,
        }));

        // Recreate tetra bind group
        self.tetra_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tetra Bind Group"),
            layout: &self.tetra_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.vertex_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.tetra_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.counter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    /// Update slice parameters
    pub fn update_params(&self, queue: &wgpu::Queue, params: &SliceParams) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));
    }

    /// Reset the triangle counter to zero
    pub fn reset_counter(&self, queue: &wgpu::Queue) {
        let zero = AtomicCounter { count: 0 };
        queue.write_buffer(&self.counter_buffer, 0, bytemuck::bytes_of(&zero));
    }

    /// Run the slice compute pass
    ///
    /// This dispatches the compute shader to process all geometry.
    /// Call reset_counter() before this and update_params() with current parameters.
    pub fn run_slice_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.use_tetrahedra {
            self.run_tetra_slice_pass(encoder);
        } else {
            self.run_legacy_slice_pass(encoder);
        }
    }

    /// Run the legacy 5-cell slice pass
    fn run_legacy_slice_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.main_bind_group.is_none() || self.simplex_count == 0 {
            return;
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Slice Compute Pass (Legacy)"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, self.main_bind_group.as_ref().unwrap(), &[]);
        compute_pass.set_bind_group(1, &self.tables_bind_group, &[]);

        let workgroup_count = (self.simplex_count + 63) / 64;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Run the tetrahedra slice pass
    fn run_tetra_slice_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.tetra_bind_group.is_none() || self.tetra_count == 0 {
            return;
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Slice Compute Pass (Tetra)"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.tetra_pipeline);
        compute_pass.set_bind_group(0, self.tetra_bind_group.as_ref().unwrap(), &[]);

        let workgroup_count = (self.tetra_count + 63) / 64;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    /// Get the output buffer for use as vertex buffer in rendering
    pub fn output_buffer(&self) -> &wgpu::Buffer {
        &self.output_buffer
    }

    /// Get the counter buffer for indirect drawing
    pub fn counter_buffer(&self) -> &wgpu::Buffer {
        &self.counter_buffer
    }

    /// Get the number of simplices currently loaded (legacy mode)
    pub fn simplex_count(&self) -> u32 {
        self.simplex_count
    }

    /// Get the number of tetrahedra currently loaded
    pub fn tetrahedron_count(&self) -> u32 {
        self.tetra_count
    }

    /// Check if using tetrahedra mode
    pub fn is_tetrahedra_mode(&self) -> bool {
        self.use_tetrahedra
    }

    /// Get the primitive count (either simplices or tetrahedra depending on mode)
    pub fn primitive_count(&self) -> u32 {
        if self.use_tetrahedra {
            self.tetra_count
        } else {
            self.simplex_count
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GPU tests require a wgpu device which isn't available in unit tests
    // Integration tests should be used for full pipeline testing

    #[test]
    fn test_output_buffer_size_calculation() {
        // Test the buffer size calculation for various triangle counts
        let vertex_size = std::mem::size_of::<Vertex3D>();
        assert_eq!(vertex_size, 48); // 48 bytes per vertex

        // 100,000 triangles * 3 vertices * 48 bytes = 14,400,000 bytes
        let size_100k = 100_000 * TRIANGLE_VERTEX_COUNT * vertex_size;
        assert_eq!(size_100k, 14_400_000);

        // 1,000,000 triangles (config default) * 3 vertices * 48 bytes = 144,000,000 bytes
        let size_1m = 1_000_000 * TRIANGLE_VERTEX_COUNT * vertex_size;
        assert_eq!(size_1m, 144_000_000);
    }
}
