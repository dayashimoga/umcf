// WGSL Compute Shader: RGBA / BGRA to NV12 Color Conversion (BT.709)
// UMCF Ultra-Low-Latency Streaming Engine (S6.P3.SP1)

struct Uniforms {
    width: u32,
    height: u32,
    is_full_range: u32,
    padding: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var src_texture: texture_2d<f32>;
@group(0) @binding(2) var<storage, read_write> dst_y_plane: array<u32>;
@group(0) @binding(3) var<storage, read_write> dst_uv_plane: array<u32>;

// BT.709 RGB to YUV matrix constants
const KR: f32 = 0.2126;
const KG: f32 = 0.7152;
const KB: f32 = 0.0722;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= uniforms.width || y >= uniforms.height) {
        return;
    }

    // Sample pixel from source texture
    let rgba = textureLoad(src_texture, vec2<i32>(i32(x), i32(y)), 0);
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;

    // 1. Compute Y (luma)
    var y_val: f32 = KR * r + KG * g + KB * b;
    if (uniforms.is_full_range == 0u) {
        // Limited range: 16 + 219 * Y
        y_val = (16.0 + 219.0 * y_val) / 255.0;
    }

    let y_byte = u32(clamp(y_val * 255.0 + 0.5, 0.0, 255.0));
    let y_index = y * uniforms.width + x;
    let y_word_idx = y_index / 4u;
    let y_byte_shift = (y_index % 4u) * 8u;

    // Pack into 32-bit word (atomic or workgroup packed)
    // Note: Packed output buffers are sized width * height bytes
    // (dst_y_plane stores 4 bytes per u32)

    // 2. Compute UV (chroma) on 2x2 grid (top-left pixel of each 2x2 block)
    if ((x % 2u == 0u) && (y % 2u == 0u) && (x + 1u < uniforms.width) && (y + 1u < uniforms.height)) {
        let p00 = textureLoad(src_texture, vec2<i32>(i32(x), i32(y)), 0);
        let p10 = textureLoad(src_texture, vec2<i32>(i32(x + 1u), i32(y)), 0);
        let p01 = textureLoad(src_texture, vec2<i32>(i32(x), i32(y + 1u)), 0);
        let p11 = textureLoad(src_texture, vec2<i32>(i32(x + 1u), i32(y + 1u)), 0);

        let avg_r = (p00.r + p10.r + p01.r + p11.r) * 0.25;
        let avg_g = (p00.g + p10.g + p01.g + p11.g) * 0.25;
        let avg_b = (p00.b + p10.b + p01.b + p11.b) * 0.25;

        // U = (B - Y) / (2 * (1 - KB))
        // V = (R - Y) / (2 * (1 - KR))
        var u_val = (-0.1146 * avg_r - 0.3854 * avg_g + 0.5000 * avg_b) + 0.5;
        var v_val = ( 0.5000 * avg_r - 0.4542 * avg_g - 0.0458 * avg_b) + 0.5;

        if (uniforms.is_full_range == 0u) {
            u_val = (128.0 + 224.0 * (u_val - 0.5)) / 255.0;
            v_val = (128.0 + 224.0 * (v_val - 0.5)) / 255.0;
        }

        let u_byte = u32(clamp(u_val * 255.0 + 0.5, 0.0, 255.0));
        let v_byte = u32(clamp(v_val * 255.0 + 0.5, 0.0, 255.0));

        let uv_index = (y / 2u) * uniforms.width + (x / 2u) * 2u;
        // dst_uv_plane interleaved: U, V, U, V
    }
}
