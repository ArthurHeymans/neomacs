//! One renderer-device policy for every Neomacs GPU entry point.

fn requested_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    adapter.features()
        & (wgpu::Features::TEXTURE_FORMAT_16BIT_NORM
            | wgpu::Features::TEXTURE_FORMAT_NV12
            | wgpu::Features::TEXTURE_FORMAT_P010)
}

/// Create the renderer's device and queue with platform interop enabled when
/// the selected adapter can support it.
///
/// Keeping this policy here prevents standalone and windowed renderer entry
/// points from silently creating devices with different native-video
/// capabilities.
pub async fn request_renderer_device(
    adapter: &wgpu::Adapter,
    label: &'static str,
) -> Result<(wgpu::Device, wgpu::Queue), String> {
    let descriptor = wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: requested_features(adapter),
        required_limits: wgpu::Limits::default(),
        memory_hints: Default::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
    };

    std::cfg_select! {
        all(target_os = "linux", feature = "video") => {
            if linux::supports_native_video_extensions(adapter) {
                match linux::request_native_video_device(adapter, &descriptor) {
                    Ok(device) => return Ok(device),
                    Err(error) => tracing::warn!(
                        %error,
                        "failed to enable Linux native-video device extensions; falling back to a standard renderer device"
                    ),
                }
            } else {
                tracing::info!(
                    "Vulkan adapter does not expose the complete Linux native-video extension set"
                );
            }
        }
        _ => {}
    }

    adapter
        .request_device(&descriptor)
        .await
        .map_err(|error| format!("failed to create renderer device: {error}"))
}

#[cfg(all(target_os = "linux", feature = "video"))]
mod linux {
    use std::ffi::CStr;

    use wgpu::hal::api::Vulkan;

    const NATIVE_VIDEO_EXTENSIONS: [&CStr; 4] = [
        ash::khr::external_memory_fd::NAME,
        ash::ext::external_memory_dma_buf::NAME,
        ash::ext::image_drm_format_modifier::NAME,
        ash::ext::queue_family_foreign::NAME,
    ];

    pub(super) fn supports_native_video_extensions(adapter: &wgpu::Adapter) -> bool {
        // SAFETY: the guard is used only for immutable capability inspection
        // and is dropped before this function returns.
        unsafe { adapter.as_hal::<Vulkan>() }.is_some_and(|hal| {
            NATIVE_VIDEO_EXTENSIONS.iter().all(|extension| {
                hal.physical_device_capabilities()
                    .supports_extension(extension)
            })
        })
    }

    pub(super) fn request_native_video_device(
        adapter: &wgpu::Adapter,
        descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), String> {
        // SAFETY: `hal_adapter` is obtained from this exact wgpu adapter. The
        // callback only adds extensions proven supported above, preserves all
        // wgpu-required features/extensions, and the resulting HAL device is
        // immediately returned to the same adapter for wgpu-core ownership.
        unsafe {
            let hal_adapter = adapter
                .as_hal::<Vulkan>()
                .ok_or_else(|| "selected renderer adapter is not Vulkan".to_owned())?;
            let hal_device = hal_adapter
                .open_with_callback(
                    descriptor.required_features,
                    &descriptor.required_limits,
                    &descriptor.memory_hints,
                    Some(Box::new(|args| {
                        for extension in NATIVE_VIDEO_EXTENSIONS {
                            if !args.extensions.contains(&extension) {
                                args.extensions.push(extension);
                            }
                        }
                    })),
                )
                .map_err(|error| format!("failed to open Vulkan interop device: {error:?}"))?;
            adapter
                .create_device_from_hal(hal_device, descriptor)
                .map_err(|error| format!("failed to adopt Vulkan interop device: {error}"))
        }
    }
}
