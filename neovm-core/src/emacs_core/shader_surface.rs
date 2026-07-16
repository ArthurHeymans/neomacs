//! Elisp builtins for shader surfaces (`doc/display-engine/SHADER_SURFACES.md`).
//!
//! `neomacs-surface-create` allocates a compositor-rendered GPU texture from
//! user WGSL (or raw RGBA8 pixels) and returns an integer surface id; the id
//! is shown inline via a `(surface :id N :width W :height H)` display
//! property. NeoMacs extension — gate uses on `(featurep 'neomacs-surface)`.

use super::error::{EvalResult, signal};
use super::eval::{
    Context, ShaderSurfaceContent, ShaderSurfaceCreateRequest, ShaderSurfaceUniformInit,
};
use super::value::{Value, list_to_vec};

fn surface_error(message: impl Into<String>) -> super::error::Flow {
    signal("error", vec![Value::string(message.into())])
}

fn plist_get(args: &[Value], key: &str) -> Option<Value> {
    let mut i = 0;
    while i + 1 < args.len() {
        if args[i].as_symbol_name() == Some(key) {
            return Some(args[i + 1]);
        }
        i += 2;
    }
    None
}

fn number_to_f32(value: Value) -> Option<f32> {
    if let Some(int) = value.as_int() {
        Some(int as f32)
    } else {
        value.as_float().map(|float| float as f32)
    }
}

fn dimension(value: Option<Value>, key: &str) -> Result<u32, super::error::Flow> {
    let value = value.ok_or_else(|| {
        surface_error(format!("neomacs-surface-create: {key} is required"))
    })?;
    let px = number_to_f32(value)
        .filter(|px| px.is_finite() && *px >= 1.0)
        .ok_or_else(|| {
            surface_error(format!(
                "neomacs-surface-create: {key} must be a positive number"
            ))
        })?;
    Ok(px.round() as u32)
}

/// Parse a `(name . VALUE)` uniform entry: VALUE is a number (one component)
/// or a vector of 1..=4 numbers.
fn parse_uniform_entry(entry: Value) -> Result<ShaderSurfaceUniformInit, super::error::Flow> {
    if !entry.is_cons() {
        return Err(surface_error(
            "neomacs-surface-create: :uniforms entries must be (NAME . VALUE) pairs",
        ));
    }
    let name_value = entry.cons_car();
    let name = name_value
        .as_symbol_name()
        .map(str::to_owned)
        .or_else(|| {
            name_value
                .as_lisp_string()
                .and_then(|s| s.as_utf8_str().map(str::to_owned))
        })
        .ok_or_else(|| {
            surface_error("neomacs-surface-create: uniform names must be symbols or strings")
        })?;
    let value = entry.cons_cdr();
    let mut components = [0.0f32; 4];
    let count;
    if let Some(scalar) = number_to_f32(value) {
        components[0] = scalar;
        count = 1u8;
    } else if let Some(elements) = value.as_vector_data() {
        let elements = elements.as_slice();
        if elements.is_empty() || elements.len() > 4 {
            return Err(surface_error(format!(
                "neomacs-surface-create: uniform {name} must have 1..=4 components"
            )));
        }
        for (slot, element) in elements.iter().enumerate() {
            components[slot] = number_to_f32(*element).ok_or_else(|| {
                surface_error(format!(
                    "neomacs-surface-create: uniform {name} components must be numbers"
                ))
            })?;
        }
        count = elements.len() as u8;
    } else {
        return Err(surface_error(format!(
            "neomacs-surface-create: uniform {name} value must be a number or vector"
        )));
    }
    Ok(ShaderSurfaceUniformInit {
        name,
        value: components,
        components: count,
    })
}

/// (neomacs-surface-create &rest PLIST)
///
/// Keys: `:shader WGSL-STRING` or `:pixels UNIBYTE-STRING` (exactly one),
/// `:width N`, `:height N` (required), `:uniforms ALIST`, `:animate BOOL`
/// (default t for shader surfaces). Returns the integer surface id, or
/// signals an error — including WGSL compile errors with naga diagnostics.
pub(crate) fn builtin_neomacs_surface_create(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    if args.len() % 2 != 0 {
        return Err(surface_error(
            "neomacs-surface-create: expected keyword/value pairs",
        ));
    }
    let width = dimension(plist_get(&args, ":width"), ":width")?;
    let height = dimension(plist_get(&args, ":height"), ":height")?;
    let shader = plist_get(&args, ":shader").filter(|value| !value.is_nil());
    let pixels = plist_get(&args, ":pixels").filter(|value| !value.is_nil());
    let animate = plist_get(&args, ":animate")
        .map(|value| !value.is_nil())
        .unwrap_or(true);

    let content = match (shader, pixels) {
        (Some(shader), None) => {
            let source = shader
                .as_lisp_string()
                .and_then(|s| s.as_utf8_str().map(str::to_owned))
                .ok_or_else(|| {
                    surface_error("neomacs-surface-create: :shader must be a string")
                })?;
            let mut uniforms = Vec::new();
            if let Some(list) = plist_get(&args, ":uniforms").filter(|value| !value.is_nil()) {
                let entries = list_to_vec(&list).ok_or_else(|| {
                    surface_error("neomacs-surface-create: :uniforms must be an alist")
                })?;
                for entry in entries {
                    uniforms.push(parse_uniform_entry(entry)?);
                }
            }
            ShaderSurfaceContent::Wgsl { source, uniforms }
        }
        (None, Some(pixels)) => {
            let data = pixels
                .as_lisp_string()
                .map(|s| s.as_bytes().to_vec())
                .ok_or_else(|| {
                    surface_error(
                        "neomacs-surface-create: :pixels must be a unibyte string of RGBA bytes",
                    )
                })?;
            let expected = width as usize * height as usize * 4;
            if data.len() < expected {
                return Err(surface_error(format!(
                    "neomacs-surface-create: :pixels has {} bytes, need {expected} ({width}x{height} RGBA)",
                    data.len()
                )));
            }
            ShaderSurfaceContent::Pixels { data }
        }
        (Some(_), Some(_)) => {
            return Err(surface_error(
                "neomacs-surface-create: :shader and :pixels are mutually exclusive",
            ));
        }
        (None, None) => {
            return Err(surface_error(
                "neomacs-surface-create: one of :shader or :pixels is required",
            ));
        }
    };

    let animate = animate && matches!(content, ShaderSurfaceContent::Wgsl { .. });
    let request = ShaderSurfaceCreateRequest {
        content,
        width,
        height,
        animate,
    };
    let host = eval.display_host.as_ref().ok_or_else(|| {
        surface_error("neomacs-surface-create: no GUI display host in this session")
    })?;
    let id = host
        .create_shader_surface(request)
        .map_err(surface_error)?;
    Ok(Value::fixnum(id as i64))
}

/// (neomacs-surface-set-uniform ID NAME VALUE)
///
/// NAME is the symbol/string used in `:uniforms` at create time; VALUE is a
/// number or a vector of 1..=4 numbers. Cheap: writes a uniform slot, no
/// shader recompile.
pub(crate) fn builtin_neomacs_surface_set_uniform(
    eval: &mut Context,
    args: Vec<Value>,
) -> EvalResult {
    let id = args[0]
        .as_int()
        .filter(|id| *id >= 0)
        .ok_or_else(|| surface_error("neomacs-surface-set-uniform: ID must be a surface id"))?;
    let entry = parse_uniform_entry(Value::cons(args[1], args[2]))?;
    if let Some(host) = eval.display_host.as_ref() {
        host.set_shader_surface_uniform(id as u32, &entry.name, entry.value)
            .map_err(surface_error)?;
    }
    Ok(Value::NIL)
}

/// (neomacs-surface-destroy ID) — free the surface's GPU objects.
pub(crate) fn builtin_neomacs_surface_destroy(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    let id = args[0]
        .as_int()
        .filter(|id| *id >= 0)
        .ok_or_else(|| surface_error("neomacs-surface-destroy: ID must be a surface id"))?;
    if let Some(host) = eval.display_host.as_ref() {
        host.destroy_shader_surface(id as u32).map_err(surface_error)?;
    }
    Ok(Value::NIL)
}

/// (neomacs-surface-available-p) — non-nil when a GUI display host that can
/// render shader surfaces is attached.
pub(crate) fn builtin_neomacs_surface_available_p(
    eval: &mut Context,
    _args: Vec<Value>,
) -> EvalResult {
    Ok(Value::bool_val(eval.display_host.is_some()))
}
