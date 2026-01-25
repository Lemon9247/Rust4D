//! Compute pipeline for 4D cross-section slicing
//!
//! This pipeline takes 4D simplices (5-cells) and produces 3D triangles
//! by intersecting them with a hyperplane at a given W coordinate.

use wgpu::util::DeviceExt;

use super::lookup_tables::{EDGE_TABLE, TRI_TABLE, EDGES};
use super::types::{
    Simplex4D, SliceParams, Vertex3D, AtomicCounter,
    MAX_OUTPUT_TRIANGLES, TRIANGLE_VERTEX_COUNT,
};

/// Compute pipeline for slicing 4D geometry
#[allow(dead_code)] // Fields hold GPU resources that must outlive bind groups
pub struct SlicePipeline {
    /// The compute pipeline
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
    /// Output buffer for triangles
    output_buffer: wgpu::Buffer,
    /// Atomic counter buffer for triangle count
    counter_buffer: wgpu::Buffer,
    /// Staging buffer for reading counter back to CPU
    counter_staging_buffer: wgpu::Buffer,
    /// Slice parameters uniform buffer
    params_buffer: wgpu::Buffer,
    /// Input simplex buffer (created per frame or when geometry changes)
    simplex_buffer: Option<wgpu::Buffer>,
    simplex_count: u32,
    /// Main bind group (recreated when simplex buffer changes)
    main_bind_group: Option<wgpu::BindGroup>,
}

impl SlicePipeline {
    /// Create a new slice pipeline
    pub fn new(device: &wgpu::Device) -> Self {
        // Create bind group layouts
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

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Slice Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
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

        // Create output buffer
        let output_size = (MAX_OUTPUT_TRIANGLES * TRIANGLE_VERTEX_COUNT * std::mem::size_of::<Vertex3D>()) as u64;
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
            pipeline,
            bind_group_layout_main,
            bind_group_layout_tables,
            edge_table_buffer,
            tri_table_buffer,
            edges_buffer,
            tables_bind_group,
            output_buffer,
            counter_buffer,
            counter_staging_buffer,
            params_buffer,
            simplex_buffer: None,
            simplex_count: 0,
            main_bind_group: None,
        }
    }

    /// Upload simplices to the GPU
    pub fn upload_simplices(&mut self, device: &wgpu::Device, simplices: &[Simplex4D]) {
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
    /// This dispatches the compute shader to process all simplices.
    /// Call reset_counter() before this and update_params() with current parameters.
    pub fn run_slice_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.main_bind_group.is_none() || self.simplex_count == 0 {
            return;
        }

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Slice Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, self.main_bind_group.as_ref().unwrap(), &[]);
        compute_pass.set_bind_group(1, &self.tables_bind_group, &[]);

        // Dispatch one workgroup per simplex (can optimize later with larger workgroups)
        let workgroup_count = (self.simplex_count + 63) / 64; // 64 threads per workgroup
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

    /// Get the number of simplices currently loaded
    pub fn simplex_count(&self) -> u32 {
        self.simplex_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GPU tests require a wgpu device which isn't available in unit tests
    // Integration tests should be used for full pipeline testing

    #[test]
    fn test_output_buffer_size() {
        let expected_size = MAX_OUTPUT_TRIANGLES * TRIANGLE_VERTEX_COUNT * std::mem::size_of::<Vertex3D>();
        // 100,000 triangles * 3 vertices * 48 bytes = 14,400,000 bytes
        assert_eq!(expected_size, 14_400_000);
    }
}
