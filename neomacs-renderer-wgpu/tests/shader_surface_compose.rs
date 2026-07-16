//! CPU-only tests for shader-surface WGSL composition and naga validation
//! (no GPU device needed).

use neomacs_renderer_wgpu::shader_surface::{
    compose_surface_wgsl, uniform_accessor_name, validate_surface_wgsl,
};

const PLASMA: &str = "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let uv = fragCoord / u.iResolution.xy;
    return vec4<f32>(0.5 + 0.5 * cos(u.iTime + uv.xyx + vec3<f32>(0.0, 2.0, 4.0)), 1.0);
}";

#[test]
fn valid_shader_composes_and_validates() {
    let uniforms = vec![("speed".to_owned(), 1u8), ("tint".to_owned(), 3u8)];
    let composed = validate_surface_wgsl(PLASMA, &uniforms).expect("valid shader");
    assert!(composed.contains("struct NeoUniforms"));
    assert!(composed.contains("fn u_speed() -> f32 { return u.custom[0].x; }"));
    assert!(composed.contains("fn u_tint() -> vec3<f32> { return u.custom[1].xyz; }"));
    assert!(composed.contains("neo_fs_main"));
    assert!(composed.ends_with("}\n"));
}

#[test]
fn shader_using_uniform_accessors_validates() {
    let source = "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
        return vec4<f32>(u_tint() * u_speed(), 1.0);
    }";
    let uniforms = vec![("speed".to_owned(), 1u8), ("tint".to_owned(), 3u8)];
    validate_surface_wgsl(source, &uniforms).expect("accessors resolve");
}

#[test]
fn syntax_error_reports_span() {
    let err = validate_surface_wgsl("fn mainImage(", &[]).expect_err("parse error");
    assert!(err.contains("error"), "diagnostic missing: {err}");
}

#[test]
fn missing_main_image_is_rejected() {
    let err =
        validate_surface_wgsl("fn not_main() -> f32 { return 0.0; }", &[]).expect_err("no entry");
    assert!(err.contains("mainImage"), "should mention mainImage: {err}");
}

#[test]
fn wrong_main_image_signature_is_rejected() {
    let source = "fn mainImage(fragCoord: vec2<f32>) -> f32 { return 0.0; }";
    validate_surface_wgsl(source, &[]).expect_err("wrong return type");
}

#[test]
fn too_many_uniforms_rejected() {
    let uniforms: Vec<(String, u8)> = (0..9).map(|i| (format!("u{i}"), 1u8)).collect();
    let err = validate_surface_wgsl(PLASMA, &uniforms).expect_err("9 uniforms");
    assert!(err.contains("too many uniforms"));
}

#[test]
fn accessor_names_are_sanitized() {
    assert_eq!(uniform_accessor_name("speed"), "u_speed");
    assert_eq!(uniform_accessor_name("my-color"), "u_my_color");
    assert_eq!(uniform_accessor_name("weird name!"), "u_weird_name_");
}

#[test]
fn lisp_style_uniform_names_compose_into_valid_wgsl() {
    let source = "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
        return vec4<f32>(vec3<f32>(u_glow_strength()), 1.0);
    }";
    let uniforms = vec![("glow-strength".to_owned(), 1u8)];
    validate_surface_wgsl(source, &uniforms).expect("kebab-case name sanitized");
}

#[test]
fn compose_is_deterministic() {
    let uniforms = vec![("a".to_owned(), 2u8)];
    assert_eq!(
        compose_surface_wgsl(PLASMA, &uniforms),
        compose_surface_wgsl(PLASMA, &uniforms)
    );
}
