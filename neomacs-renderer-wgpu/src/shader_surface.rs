//! Shader-surface WGSL composition and validation.
//!
//! A shader surface (`doc/display-engine/SHADER_SURFACES.md`) is a texture the
//! compositor renders from a user-supplied WGSL fragment written against a
//! Shadertoy-compatible contract: the user defines
//!
//! ```wgsl
//! fn mainImage(fragCoord: vec2<f32>) -> vec4<f32>
//! ```
//!
//! and this module prepends the generated prelude (uniform block, per-uniform
//! accessor functions, fullscreen-triangle vertex entry, fragment entry that
//! flips `fragCoord` to Shadertoy's y-up convention). Composition is
//! deterministic so the Lisp thread can validate with naga and the render
//! thread can compile the identical source.

/// Number of user `vec4<f32>` uniform slots in the prelude's `custom` array.
pub const SURFACE_USER_UNIFORM_SLOTS: usize = 8;

/// Uniform-buffer size in bytes: iResolution + iMouse + (iTime, iTimeDelta,
/// iFrame, pad) + 8 custom vec4 slots.
pub const SURFACE_UNIFORM_BYTES: u64 = (4 + 4 + 4 + 4 * SURFACE_USER_UNIFORM_SLOTS as u64) * 4;

/// Initial value for one user uniform: Lisp `(name . value)` pairs arrive as
/// a name, up to four components, and the component count (1..=4) that picks
/// the accessor's WGSL type (f32/vec2/vec3/vec4).
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceUniformInit {
    pub name: String,
    pub value: [f32; 4],
    pub components: u8,
}

/// WGSL identifier for a user uniform name: non-alphanumerics become `_`,
/// prefixed `u_` (`speed` -> `u_speed`, `my-color` -> `u_my_color`).
pub fn uniform_accessor_name(name: &str) -> String {
    let mut ident = String::with_capacity(name.len() + 2);
    ident.push_str("u_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            ident.push(ch);
        } else {
            ident.push('_');
        }
    }
    ident
}

fn accessor_return(components: u8) -> (&'static str, &'static str) {
    match components {
        1 => ("f32", ".x"),
        2 => ("vec2<f32>", ".xy"),
        3 => ("vec3<f32>", ".xyz"),
        _ => ("vec4<f32>", ""),
    }
}

/// Compose the full WGSL module: generated prelude + the user source.
///
/// `uniforms` lists `(name, components)` in slot order. WGSL module-scope
/// declarations are order-independent, so the prelude's fragment entry may
/// call `mainImage` before the user source defines it.
pub fn compose_surface_wgsl(user_source: &str, uniforms: &[(String, u8)]) -> String {
    let mut src = String::with_capacity(user_source.len() + 1024);
    src.push_str(
        "// ---- neomacs shader-surface prelude (generated) ----\n\
         struct NeoUniforms {\n\
         \x20   iResolution: vec4<f32>,\n\
         \x20   // xy: hover position in physical px (origin bottom-left, y-up);\n\
         \x20   // persists at the last hover position when the pointer leaves.\n\
         \x20   // zw: reserved for click state, currently 0.\n\
         \x20   iMouse: vec4<f32>,\n\
         \x20   iTime: f32,\n\
         \x20   iTimeDelta: f32,\n\
         \x20   iFrame: f32,\n\
         \x20   _neo_pad0: f32,\n\
         \x20   custom: array<vec4<f32>, 8>,\n\
         }\n\
         @group(0) @binding(0) var<uniform> u: NeoUniforms;\n\
         // Channel input (Shadertoy-style): another surface's texture bound\n\
         // via `:channel0 ID`; unbound channels sample transparent black.\n\
         @group(0) @binding(1) var iChannel0: texture_2d<f32>;\n\
         @group(0) @binding(2) var iChannel0Sampler: sampler;\n",
    );
    for (slot, (name, components)) in uniforms.iter().enumerate().take(SURFACE_USER_UNIFORM_SLOTS)
    {
        let (ret, swizzle) = accessor_return(*components);
        let ident = uniform_accessor_name(name);
        src.push_str(&format!(
            "fn {ident}() -> {ret} {{ return u.custom[{slot}]{swizzle}; }}\n"
        ));
    }
    src.push_str(
        "@vertex\n\
         fn neo_vs_main(@builtin(vertex_index) neo_vi: u32) -> @builtin(position) vec4<f32> {\n\
         \x20   var neo_pos = array<vec2<f32>, 3>(\n\
         \x20       vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));\n\
         \x20   return vec4<f32>(neo_pos[neo_vi], 0.0, 1.0);\n\
         }\n\
         @fragment\n\
         fn neo_fs_main(@builtin(position) neo_pos: vec4<f32>) -> @location(0) vec4<f32> {\n\
         \x20   // Shadertoy fragCoord convention: origin bottom-left, y up.\n\
         \x20   return mainImage(vec2<f32>(neo_pos.x, u.iResolution.y - neo_pos.y));\n\
         }\n\
         // ---- user source ----\n",
    );
    src.push_str(user_source);
    src.push('\n');
    src
}

/// Compose and validate a user shader with naga (parse + full validation, no
/// GPU device needed). Returns the composed module source on success and a
/// span-annotated human-readable error on failure — the Lisp thread turns the
/// error into a signal from `neomacs-surface-create`.
pub fn validate_surface_wgsl(
    user_source: &str,
    uniforms: &[(String, u8)],
) -> Result<String, String> {
    if uniforms.len() > SURFACE_USER_UNIFORM_SLOTS {
        return Err(format!(
            "too many uniforms: {} given, {} slots available",
            uniforms.len(),
            SURFACE_USER_UNIFORM_SLOTS
        ));
    }
    let source = compose_surface_wgsl(user_source, uniforms);
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|err| err.emit_to_string_with_path(&source, "surface.wgsl"))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    );
    validator
        .validate(&module)
        .map_err(|err| err.emit_to_string_with_path(&source, "surface.wgsl"))?;
    Ok(source)
}
