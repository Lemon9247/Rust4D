// 4D Cross-Section Render Shader
//
// This shader renders the 3D triangles produced by the slice compute shader.
// It applies view/projection transformation and W-depth based coloring.
//
// Features:
// - W-depth visualization: red (+W) to blue (-W) gradient
// - Basic diffuse lighting
// - Vertex color blending

// ============================================================================
// Data Structures
// ============================================================================

/// Vertex input from the sliced triangles
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) w_depth: f32,
}

/// Vertex output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) w_depth: f32,
}

/// Render uniforms
struct RenderUniforms {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    light_direction: vec3<f32>,
    _pad0: f32,
    ambient_strength: f32,
    diffuse_strength: f32,
    w_color_strength: f32,   // How much W-depth affects color (0-1)
    w_range: f32,            // Range of W values for normalization
}

// ============================================================================
// Uniforms
// ============================================================================

@group(0) @binding(0) var<uniform> uniforms: RenderUniforms;

// ============================================================================
// Vertex Shader
// ============================================================================

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Transform to clip space
    let world_pos = vec4<f32>(input.position, 1.0);
    let view_pos = uniforms.view_matrix * world_pos;
    output.clip_position = uniforms.projection_matrix * view_pos;

    // Pass through world position and normal for lighting
    output.world_position = input.position;
    output.world_normal = input.normal;

    // Pass through color and W-depth
    output.vertex_color = input.color;
    output.w_depth = input.w_depth;

    return output;
}

// ============================================================================
// Fragment Shader
// ============================================================================

/// Map W-depth to a color gradient
/// Positive W (towards the 4th dimension) = warm colors (red/orange)
/// Negative W (away from 4th dimension) = cool colors (blue/cyan)
/// Zero W = neutral (based on vertex color)
fn w_depth_to_color(w: f32, w_range: f32) -> vec3<f32> {
    // Normalize W to [-1, 1] range
    let w_normalized = clamp(w / w_range, -1.0, 1.0);

    // Create a gradient from blue (-W) through white (0) to red (+W)
    // Using a smooth interpolation for visual appeal
    let t = w_normalized * 0.5 + 0.5; // Map to [0, 1]

    // Cool (blue) to warm (red) gradient
    let cool_color = vec3<f32>(0.2, 0.4, 0.9);   // Blue
    let neutral_color = vec3<f32>(0.8, 0.8, 0.8); // Light gray
    let warm_color = vec3<f32>(0.9, 0.3, 0.2);   // Red

    // Two-part interpolation: blue -> neutral -> red
    if (t < 0.5) {
        return mix(cool_color, neutral_color, t * 2.0);
    } else {
        return mix(neutral_color, warm_color, (t - 0.5) * 2.0);
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the interpolated normal
    let normal = normalize(input.world_normal);

    // Normalize light direction
    let light_dir = normalize(uniforms.light_direction);

    // Calculate diffuse lighting (Lambert)
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse = n_dot_l * uniforms.diffuse_strength;

    // Total light contribution
    let light = uniforms.ambient_strength + diffuse;

    // Get W-depth based color
    let w_color = w_depth_to_color(input.w_depth, uniforms.w_range);

    // Blend vertex color with W-depth color
    let base_color = input.vertex_color.rgb;
    let blended_color = mix(base_color, w_color, uniforms.w_color_strength);

    // Apply lighting
    let final_color = blended_color * light;

    // Output with original alpha
    return vec4<f32>(final_color, input.vertex_color.a);
}
