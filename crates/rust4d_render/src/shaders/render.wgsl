// 4D Cross-Section Render Shader
//
// This shader renders the 3D triangles produced by the slice compute shader.
// It applies view/projection transformation, W-depth coloring, two-sided
// Blinn-Phong lighting, point lights, and distance fog.
//
// Important: slice-generated triangle winding is not stable across all 4D
// tetrahedron cases, so fragment shading must be two-sided. Back-face culling
// is disabled in the Rust render pipeline and normals are flipped toward the
// camera before lighting.

// ============================================================================
// Data Structures
// ============================================================================

/// Vertex input from the sliced triangles
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) w_depth: f32,
    @location(4) world_position: vec3<f32>,
}

/// Vertex output to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) w_depth: f32,
    @location(4) slice_world_position: vec3<f32>,
}

/// Point light, layout-matched with Rust `PointLightUniform`.
struct PointLight {
    position_radius: vec4<f32>,   // xyz position, w radius
    color_intensity: vec4<f32>,   // rgb color, w intensity
}

/// Render uniforms, layout-matched with Rust `RenderUniforms` (368 bytes).
struct RenderUniforms {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    light_direction: vec3<f32>,
    _pad0: f32,
    camera_position: vec3<f32>,
    _pad_camera: f32,
    ambient_strength: f32,
    diffuse_strength: f32,
    specular_strength: f32,
    specular_power: f32,
    w_color_strength: f32,
    w_range: f32,
    fog_density: f32,
    point_light_count: f32,
    fog_color: vec3<f32>,
    _pad_fog: f32,
    point_lights: array<PointLight, 4>,
    // Floor checkerboard: color A + cell size in `.w`, color B + enabled flag in `.w`.
    // Floor fragments are identified by a sentinel alpha (> 1.0) written into
    // the vertex color by CheckerboardGeometry; the fragment shader rewrites
    // the output alpha to 1.0 for those fragments.
    floor_color_a: vec4<f32>,
    floor_color_b: vec4<f32>,
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

    let world_pos = vec4<f32>(input.position, 1.0);
    let view_pos = uniforms.view_matrix * world_pos;
    output.clip_position = uniforms.projection_matrix * view_pos;

    output.world_position = input.position;
    output.world_normal = input.normal;
    output.vertex_color = input.color;
    output.w_depth = input.w_depth;
    output.slice_world_position = input.world_position;

    return output;
}

// ============================================================================
// Fragment Shader
// ============================================================================

/// Map W-depth to a color gradient.
/// Positive W (towards the 4th dimension) = warm colors (red/orange)
/// Negative W (away from the 4th dimension) = cool colors (blue/cyan)
/// Zero W = neutral (based on vertex color)
fn w_depth_to_color(w: f32, w_range: f32) -> vec3<f32> {
    let w_normalized = clamp(w / max(w_range, 0.0001), -1.0, 1.0);
    let t = w_normalized * 0.5 + 0.5;

    let cool_color = vec3<f32>(0.2, 0.4, 0.9);
    let neutral_color = vec3<f32>(0.8, 0.8, 0.8);
    let warm_color = vec3<f32>(0.9, 0.3, 0.2);

    if (t < 0.5) {
        return mix(cool_color, neutral_color, t * 2.0);
    }
    return mix(neutral_color, warm_color, (t - 0.5) * 2.0);
}

fn safe_normalize(v: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    let len = length(v);
    if (len < 0.00001) {
        return fallback;
    }
    return v / len;
}

fn blinn_phong(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    light_color: vec3<f32>,
    light_intensity: f32,
) -> vec3<f32> {
    let ndotl = max(dot(normal, light_dir), 0.0);
    let half_dir = safe_normalize(light_dir + view_dir, normal);
    let spec = pow(max(dot(normal, half_dir), 0.0), max(uniforms.specular_power, 1.0));
    return light_color * light_intensity * (
        ndotl * uniforms.diffuse_strength + spec * uniforms.specular_strength
    );
}

/// Sentinel alpha written into floor vertex colors by CheckerboardGeometry.
/// Values > 1.0 are outside the normal alpha range, so they cannot collide
/// with legitimate per-vertex alphas. Kept in sync with Rust's
/// `FLOOR_ALPHA_SENTINEL`.
const FLOOR_ALPHA_SENTINEL: f32 = 2.0;

/// Compute a crisp, resolution-independent checkerboard color from the sliced
/// world XZ. Returns the picked floor color (rgb) — the caller owns alpha.
fn floor_checker_color(world_xz: vec2<f32>) -> vec3<f32> {
    let cell_size = max(uniforms.floor_color_a.w, 0.0001);
    let cell = floor(world_xz.x / cell_size) + floor(world_xz.y / cell_size);
    // Floored modulo (handles negative cell sums): parity is 0 for even cells,
    // 1 for odd cells.
    let parity = cell - 2.0 * floor(cell / 2.0);
    if (parity < 1.0) {
        return uniforms.floor_color_a.rgb;
    }
    return uniforms.floor_color_b.rgb;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let base_normal = safe_normalize(input.world_normal, vec3<f32>(0.0, 0.0, 1.0));
    let view_dir = safe_normalize(uniforms.camera_position - input.world_position, vec3<f32>(0.0, 0.0, 1.0));

    // Two-sided shading: flip normals that face away from the camera.
    var normal = base_normal;
    if (dot(normal, view_dir) < 0.0) {
        normal = -normal;
    }

    let w_color = w_depth_to_color(input.w_depth, uniforms.w_range);

    // Floor fragments are tagged with a sentinel alpha. Recompute the checker
    // in fragment space from the sliced world XZ so cell boundaries stay crisp
    // regardless of floor face size (the per-vertex checker smears across
    // large hyperplane faces). When disabled (floor_color_b.w < 0.5), fall
    // back to the (smeared) per-vertex color.
    let is_floor = input.vertex_color.a > FLOOR_ALPHA_SENTINEL * 0.75;
    let checker_enabled = uniforms.floor_color_b.w > 0.5;
    var base_color = input.vertex_color.rgb;
    var out_alpha = input.vertex_color.a;
    if (is_floor) {
        out_alpha = 1.0;
        if (checker_enabled) {
            base_color = floor_checker_color(input.slice_world_position.xz);
        }
    }

    let blended_color = mix(base_color, w_color, uniforms.w_color_strength);

    var light = vec3<f32>(uniforms.ambient_strength);

    // Directional light. `light_direction` points from the surface toward the light.
    let dir_light = safe_normalize(uniforms.light_direction, vec3<f32>(0.3, 0.8, 0.4));
    light += blinn_phong(normal, view_dir, dir_light, vec3<f32>(1.0, 0.96, 0.88), 1.0);

    // Up to four point lights with smooth quadratic attenuation to zero at radius.
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        if (f32(i) < uniforms.point_light_count) {
            let pl = uniforms.point_lights[i];
            let to_light = pl.position_radius.xyz - input.world_position;
            let dist = length(to_light);
            let radius = max(pl.position_radius.w, 0.0001);
            let attenuation = pow(clamp(1.0 - dist / radius, 0.0, 1.0), 2.0);
            let light_dir = safe_normalize(to_light, dir_light);
            light += blinn_phong(
                normal,
                view_dir,
                light_dir,
                pl.color_intensity.rgb,
                pl.color_intensity.w * attenuation,
            );
        }
    }

    var final_color = blended_color * light;

    let distance_to_camera = length(uniforms.camera_position - input.world_position);
    let fog_amount = clamp(1.0 - exp(-distance_to_camera * uniforms.fog_density), 0.0, 1.0);
    final_color = mix(final_color, uniforms.fog_color, fog_amount);

    return vec4<f32>(final_color, out_alpha);
}
