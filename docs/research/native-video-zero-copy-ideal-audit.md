# Native video zero-copy ideal-design audit

Status: complete architecture audit, 2026-09-02. Sources are platform-owner
documentation and first-party project source.

## Verdict

Neomacs is using the right macOS import primitive: a Metal-compatible
`CVPixelBuffer`, mapped plane-by-plane with `CVMetalTextureCache`, then wrapped
by wgpu without a pixel upload. Apple calls the result a live binding to the
underlying `MTLTexture`, so this boundary is genuinely zero-copy rather than a
GPU blit.

The pipeline is not yet entitled to claim *decoder-to-compositor* zero-copy.
`AVPlayerItemVideoOutput` returns a `CVPixelBuffer`, but Apple does not promise
that this is the hardware decoder's original output allocation. Its `copy` name
expresses Core Foundation ownership, not documented pixel-copy behavior. The
current conservative `GpuInteropCopy { reported_bytes: None }` classification
is therefore honest, although it conflates an unknown decoder boundary with a
known direct Metal import.

Replacing AVPlayer with raw VideoToolbox everywhere is not automatically the
ideal design. AVPlayer owns demuxing, streaming, seeking, timing, and format
changes. A lower-level VideoToolbox decoder is justified as a strict
performance tier only when measurements show the AVPlayer path is a bottleneck
or policy requires a provable hardware/shared-pool path.

The ideal architecture is consequently tiered:

1. Keep the AVPlayer output path as the broad, production playback backend.
2. Make its Core Video-to-Metal import a proven `BorrowedNativeSurface` stage,
   while reporting decoder-output provenance separately as `Unknown`.
3. Add a VideoToolbox path only as an independently measured strict tier. It
   may claim direct decoder output only after runtime evidence says hardware
   decode is active and the decoder/client pixel-buffer pool is shared.
4. Keep `AVSampleBufferDisplayLayer` or `AVPlayerLayer` as an optional native
   overlay path, not the universal inline-video path: those APIs own
   presentation and do not give Neomacs a texture to compose inside its wgpu
   scene.

The proposed Windows direction is also correct in principle, but D3D11On12
should not be the first choice on every supported Windows release. The tier
order should be:

1. On Windows 11, probe for a D3D12-aware Media Foundation decoder and request
   decoder-owned D3D12 NV12/P010 resources on wgpu's own device. Use Media
   Foundation's D3D12 synchronization object for GPU-ready and GPU-release
   dependencies.
2. Otherwise, have a Source Reader decode into decoder-owned D3D11 textures on
   a D3D11On12 device created over wgpu's D3D12 device and queue. Import those
   surfaces through `UnwrapUnderlyingResource`/`ReturnUnderlyingResource`.
3. Retain the current Media Engine `TransferVideoFrame` path as a robust
   GPU-resident fallback. Microsoft calls it a blit, so it must remain
   `GpuBlit`, not direct.
4. Make CPU decode/upload an explicit compatibility policy, never a silent
   consequence of requesting the fast path.

This makes genuine no-pixel-copy playback achievable on both platforms, but
only as a runtime-negotiated result. Codec support, OS version, GPU/driver,
resource format, adapter identity, and synchronization support are all part of
the result; `target_os` alone cannot establish it.

In this document, "direct" means that Neomacs observes the decoder output
resource itself being sampled by the compositor without an intervening pixel
transfer. It does not assert that a codec implementation or driver made no
internal reference-frame, resolve, tiling, or post-processing copy. Those
operations are opaque to public APIs. Diagnostics should say which boundary
was observed instead of advertising an absolute "zero copies anywhere" claim.

## Apple evidence

### Playback output does not prove decoder-surface identity

[`AVPlayerItemVideoOutput`](https://developer.apple.com/documentation/avfoundation/avplayeritemvideooutput)
is documented as an object that outputs frames from a player item. Its
[`copyPixelBuffer(forItemTime:itemTimeForDisplay:)`](https://developer.apple.com/documentation/avfoundation/avplayeritemvideooutput/copypixelbuffer%28foritemtime%3Aitemtimefordisplay%3A%29?language=objc)
method retrieves an image appropriate for a requested display time, marks that
image as acquired, and transfers release responsibility to the caller. The
documentation does not state that the returned buffer is the decoder's
original allocation, nor that no conversion or materialization occurred.

This means Neomacs must not infer a physical copy from the word `copy`, but it
must not infer decoder-direct memory from the returned `CVPixelBuffer` either.

Apple says this method is normally called in response to a `CVDisplayLink` or
`CADisplayLink`. The related
[`itemTime(forHostTime:)`](https://developer.apple.com/documentation/avfoundation/avplayeritemoutput/itemtime%28forhosttime%3A%29?changes=__2)
documentation explains how to select the item time for the next screen refresh.
For precise frame selection, Neomacs should drive pulls from its actual render
deadline and convert that host time through the output, instead of treating a
fixed-frequency poll of `AVPlayerItem.currentTime()` as the final design.

For macOS 14.2 and newer,
[`AVPlayerVideoOutput`](https://developer.apple.com/documentation/avfoundation/avplayervideooutput)
is the newer player-level output API. Its
[`sample(forHostTime:)`](https://developer.apple.com/documentation/avfoundation/avplayervideooutput/sample%28forhosttime%3A%29?changes=_1%2C_1)
returns tagged buffers, the presentation deadline, and the active
configuration for a host time. It is worth an availability-gated adapter for
new media features, but its documentation likewise does not prove native
decoder allocation identity, so it does not by itself upgrade the transfer
classification.

### Core Video to Metal is the correct no-copy import boundary

Apple's
[`kCVPixelBufferMetalCompatibilityKey`](https://developer.apple.com/documentation/corevideo/kcvpixelbuffermetalcompatibilitykey?changes=__2_2)
requests buffers compatible with Metal. Apple's
[`CVMetalTextureCache`](https://developer.apple.com/documentation/corevideo/cvmetaltexturecache-q3j?changes=latest_minor)
overview explicitly describes directly reading GPU-based Core Video image
buffers from Metal.

Most importantly,
[`CVMetalTextureCacheCreateTextureFromImage`](https://developer.apple.com/documentation/corevideo/cvmetaltexturecachecreatetexturefromimage%28_%3A_%3A_%3A_%3A_%3A_%3A_%3A_%3A_%3A%29?changes=_3_2&language=objc)
documents a live binding to the underlying `MTLTexture`. It also gives the
canonical bi-planar mapping used by Neomacs:

- NV12 luma plane: `MTLPixelFormatR8Unorm` at full size.
- NV12 CbCr plane: `MTLPixelFormatRG8Unorm` at half width and height.
- The plane index selects which plane is mapped; this is not a conversion to an
  intermediate RGB texture.

Neomacs's corresponding P010 mapping to `R16Unorm` and `RG16Unorm` follows the
same plane-view model and preserves 10-bit data for shader conversion. Apple's
[`format420YpCbCr10BiPlanarVideoRange`](https://developer.apple.com/documentation/accelerate/vimagecvimageformat/format/format420ypcbcr10biplanarvideorange)
documentation identifies the representation as two-plane 4:2:0 with the 10
bits stored in the most significant bits of 16-bit components. Apple's
[HDR guidance](https://developer.apple.com/av-foundation/Incorporating-HDR-video-with-Dolby-Vision-into-your-apps.pdf)
identifies `kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange` as a commonly used
10-bit playback format and stresses retaining per-frame HDR metadata.

### Strong hardware/shared-pool evidence requires VideoToolbox

Apple describes
[`VideoToolbox`](https://developer.apple.com/documentation/videotoolbox?changes=latest_m_3&language=objc)
as direct access to hardware encoders and decoders. A
[`VTDecompressionSession`](https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession-api-collection)
accepts destination image-buffer attributes and emits decompressed
`CVImageBuffer` objects through its output callback.

A strict Neomacs backend should:

- put `kCVPixelBufferMetalCompatibilityKey = true` and NV12/P010 format
  requirements in `destinationImageBufferAttributes`;
- request hardware decoding with
  [`kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder`](https://developer.apple.com/documentation/videotoolbox/kvtvideodecoderspecification_requirehardwareacceleratedvideodecoder),
  or prefer it and inspect
  `kVTDecompressionPropertyKey_UsingHardwareAcceleratedVideoDecoder` when
  fallback is allowed;
- query
  [`kVTDecompressionPropertyKey_PixelBufferPoolIsShared`](https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_pixelbufferpoolisshared?changes=l_1&language=objc).
  Apple says this becomes false when separate pools are required because the
  decoder and client attributes are incompatible;
- obtain diagnostics from
  [`kVTDecompressionPropertyKey_PixelBufferPool`](https://developer.apple.com/documentation/videotoolbox/kvtdecompressionpropertykey_pixelbufferpool),
  whose buffers are guaranteed compatible with the client's requested
  attributes;
- preserve the returned pixel-buffer lease and treat the image as immutable.
  Apple's
  [`VTDecompressionOutputHandler`](https://developer.apple.com/documentation/videotoolbox/vtdecompressionoutputhandler)
  warns that the decoder may still reference the image when the modifiable flag
  is absent.

`hardware decoder = true` plus `shared pool = true` is the strongest public API
evidence available for a decoder-direct surface. It should be reported as
runtime evidence, not assumed from platform or codec name.

### Native display layers are a separate fast path

[`AVSampleBufferDisplayLayer`](https://developer.apple.com/documentation/AVFoundation/AVSampleBufferDisplayLayer)
can accept compressed or uncompressed samples and owns their display. Apple's
HDR guidance recommends AVPlayer/AVPlayerLayer or
AVSampleBufferDisplayLayer when the system should manage the HDR presentation
pipeline.

That is potentially the most power-efficient path for a plain rectangular
video overlay. It is not a replacement for Neomacs's composited path when text,
clipping, transforms, opacity, or other glyphs must participate in the same
wgpu render graph. Model it as `NativeOverlay`, a distinct presentation mode
with explicit eligibility rules, rather than pretending it is a sampled GPU
surface.

## Windows evidence

### `TransferVideoFrame` is a good fallback, not zero-copy

Microsoft documents
[`IMFMediaEngine::TransferVideoFrame`](https://learn.microsoft.com/en-us/windows/win32/api/mfmediaengine/nf-mfmediaengine-imfmediaengine-transfervideoframe)
as copying the current video frame to a destination surface and, more
specifically, as a blit in frame-server mode. Keeping an NV12 destination
avoids an avoidable YUV-to-RGB conversion and keeping the destination GPU
resident avoids a CPU round trip, but neither changes that contract into a
borrowed decoder surface.

The present Neomacs design -- a bounded pool of D3D12 NV12/BGRA textures,
wrapped for D3D11 so Media Engine can blit into them, then sampled by wgpu -- is
therefore a sound compatibility tier. It is not the final tier for the stated
performance goal.

### Decoder-owned D3D11 surfaces are the broad direct path

The
[`MF_SOURCE_READER_D3D_MANAGER`](https://learn.microsoft.com/en-us/windows/win32/medfound/mf-source-reader-d3d-manager)
contract exists specifically to give Source Reader decoders a Direct3D device.
When the decoder supports DXVA, it uses that device to allocate video buffers.
The returned sample's
[`IMFDXGIBuffer::GetResource`](https://learn.microsoft.com/en-us/windows/win32/api/mfobjects/nf-mfobjects-imfdxgibuffer-getresource)
can expose the underlying `ID3D11Texture2D`. This avoids asking Media Engine to
copy its current frame into an application allocation.

For a D3D12 wgpu renderer, create the Media Foundation device manager from a
D3D11On12 device layered over the exact D3D12 device and command queue used by
wgpu. On Windows 10 version 2004 and later,
[`ID3D11On12Device2::UnwrapUnderlyingResource`](https://learn.microsoft.com/en-us/windows/win32/api/d3d11on12/nf-d3d11on12-id3d11on12device2-unwrapunderlyingresource)
can unwrap textures created by the D3D11 device, transitions them to `COMMON`,
and schedules waits for pending D3D11 work on the supplied D3D12 queue.
[`ReturnUnderlyingResource`](https://learn.microsoft.com/en-us/windows/win32/api/d3d11on12/nf-d3d11on12-id3d11on12device2-returnunderlyingresource)
checks the texture back into D3D11On12 with fences identifying all pending
D3D12 consumer work.

That pair is the correct direct interop primitive, subject to four hard
requirements:

- decoder surfaces must originate from that same D3D11On12 device;
- the surface format, array slice, plane views, and wgpu descriptor must match;
- the `IMFSample` and texture must remain leased until the consumer fence has
  been handed to `ReturnUnderlyingResource`;
- no D3D11/D3D11On12 work may use a resource while it is checked out.

These are correctness requirements, not optional optimizations. The wgpu
[`create_texture_from_hal`](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html#method.create_texture_from_hal)
safety contract independently requires a texture from the same underlying
device, a truthful descriptor, and the correct initial state.

### Prefer native D3D12 Media Foundation resources when supported

Windows 11 Media Foundation defines
[`MF_MT_D3D_RESOURCE_VERSION` and D3D12 resource attributes](https://learn.microsoft.com/en-us/windows/win32/medfound/d3d12-attributes).
A decoder advertises D3D12 support through
[`MF_SA_D3D12_AWARE`](https://learn.microsoft.com/en-us/windows/win32/medfound/mf-sa-d3d12-aware),
which is read-only and defaults to false. Therefore D3D12 output is a probed
capability, not something Neomacs can force on every decoder.

When available, this path is a better fit than D3D11On12 because the decoded
resource is already an `ID3D12Resource` on the compositor API. Media
Foundation's
[`IMFD3D12SynchronizationObjectCommands`](https://learn.microsoft.com/en-us/windows/win32/api/mfd3d12/nn-mfd3d12-imfd3d12synchronizationobjectcommands)
provides the necessary protocol: enqueue a ready wait on the consumer command
queue before reading the sample, and enqueue release after the consumer work
so the decoder does not recycle the resource early. The associated
[`MF_D3D12_SYNCHRONIZATION_OBJECT`](https://learn.microsoft.com/en-us/windows/win32/medfound/d3d12-mf-guids)
is available from Windows 11.

This should be the preferred Windows tier where the selected decoder and
format actually negotiate it. Direct D3D12 video decoding is not universal:
Microsoft requires applications to query codec/profile/format support, and
the
[`Direct3D 12 Video overview`](https://learn.microsoft.com/en-us/windows/win32/medfound/direct3d-12-video-overview)
describes capability tiers and explicit reference-frame and fence management.
Neomacs should use Media Foundation's transform when available rather than
implementing codec parsers and D3D12 decode submission itself.

### Driver variability makes fallback and telemetry mandatory

Chromium's production
[`D3D11VideoDecoder`](https://github.com/chromium/chromium/blob/main/media/gpu/windows/d3d11_video_decoder.cc)
uses bounded picture buffers, pauses when all buffers are in use, carries a
release callback back to the decoder, preserves color/HDR metadata, and marks
whether a frame is WebGPU-compatible. Chromium also has a specific workaround
that disables NV12 D3D11-to-D3D12 sharing on affected configurations. This is
strong evidence that the direct path should be runtime-probed and recover to a
GPU blit when interop fails; a compile-time Windows branch is insufficient.

The lesson is to copy Chromium's ownership and recovery contract, not its
process architecture. Neomacs does not need a browser-sized shared-image
system.

## Lifetime and synchronization requirements

Apple's `CVMetalTextureCacheCreateTextureFromImage` documentation requires a
strong reference to each returned `CVMetalTexture` until the GPU has finished
commands that access it; Apple specifically suggests releasing the reference
from a Metal command-buffer completion handler.

Neomacs currently retains the `CVPixelBuffer` and `CVMetalTexture` objects in a
frame/surface lease, which is directionally correct. The stronger abstraction
is a GPU-completion-aware native-frame lease:

```text
NativeFrameLease
  owns CVPixelBuffer
  owns CVMetalTexture for every imported plane
  releases/recycles only after wgpu reports it no longer uses the texture
```

wgpu-hal 30's Metal
[`Device::texture_from_raw`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/device.rs#L415-L435)
accepts a `DropCallback`. wgpu documents that callback as a signal that wgpu is
no longer using a resource. Capturing the retained `CVMetalTexture` in that
callback is a more explicit match for Apple's contract than relying on the
renderer to keep a parallel lease alive for an assumed number of frames.

Chromium follows the same production pattern in its
[`VideoToolboxFrameConverter`](https://chromium.googlesource.com/chromium/src/+/HEAD/media/gpu/mac/video_toolbox_frame_converter.cc):
it wraps the decoded pixel buffer's IOSurface, retains the `CVImageBuffer`, and
releases it only after the consumer's release sync token has completed. That
supports the lease shape without requiring Neomacs to reproduce Chromium's
shared-image or multi-process infrastructure.

Wrapping the HAL texture with
[`wgpu::Device::create_texture_from_hal`](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html#method.create_texture_from_hal)
is necessarily `unsafe`. Its contract requires the native texture to come from
the same underlying device, the descriptor to match, the texture to be
initialized, and the initial resource state to be truthful. The unsafe code
should remain confined to the macOS importer; neither the common video model
nor renderer callers should manipulate Metal handles.

The Windows lease has the same semantic shape even though the native objects
differ:

```text
NativeFrameLease
  owns IMFSample and its DXGI texture
  owns the wgpu/HAL view for every sampled plane
  carries the decode-ready queue dependency
  returns/releases the native sample only after GPU consumer completion
```

Do not approximate this with "keep N frames alive." Completion must be tied to
the actual queue submission. The pool should be bounded and apply
backpressure/drop policy when every slot is leased; it must not allocate one
new native surface per rendered frame or block the editor waiting on a CPU
fence.

## wgpu 30 constraints and opportunities

wgpu 30 now has a public
[`Device::create_external_texture`](https://docs.rs/wgpu/30.0.1/wgpu/struct.Device.html#method.create_external_texture)
API that builds an external texture from already-created plane texture views.
Its
[`ExternalTextureDescriptor`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-types/src/texture/external_texture.rs#L45-L120)
models YUV-to-RGB, gamut conversion, transfer functions, and sampling
transforms. This is useful after Core Video planes have been imported, but it
does not import a `CVPixelBuffer` or `IOSurface`; the private Metal HAL bridge is
still required.

The current
[`ExternalTextureFormat`](https://docs.rs/wgpu/30.0.0/wgpu/enum.ExternalTextureFormat.html)
supports RGBA, NV12, and Yu12, but not P010. In contrast,
[`TextureFormat`](https://docs.rs/wgpu/30.0.1/wgpu/enum.TextureFormat.html)
does model both NV12 and P010. Consequently:

- wgpu `ExternalTexture` is a good future common sampling abstraction for the
  NV12 tier;
- switching the whole Neomacs pipeline to it now would regress the P010/HDR
  tier or require a second renderer path;
- the existing pair-of-plane texture abstraction remains a defensible common
  denominator, provided colorimetry and transfer metadata stay typed and are
  tested against the wgpu external-texture conversion semantics.

wgpu's earlier
[`ExternalTexture` RFC](https://github.com/gfx-rs/wgpu/issues/3145)
also makes the API boundary explicit: native platform-memory import is outside
the portable public API and belongs in backend/HAL-specific code. That supports
Neomacs's current structure of private platform importers behind a portable
decoded-frame interface.

## Recommended cross-platform architecture

Keep one deep renderer-facing module and make the platform machinery private:

```text
VideoSystem (open/control/close by VideoId)
  -> PlaybackBackend (demux, decode, clock, format changes)
  -> NativeFrameLease (opaque storage + typed timing/color/geometry)
  -> FrameImporter (runtime capability negotiation)
  -> SampledFrameLease (planes or packed texture + GPU completion release)
  -> compositor
```

Use `cfg_select!` once in the platform factory to select the macOS, Windows,
Linux, or unsupported implementation at compile time. Do not use it to decide
whether a particular codec, pixel format, hardware decoder, interop route, or
overlay works: those decisions require runtime capability negotiation and
measured failure recovery.

The preferred tier order is:

| Platform | Tier 1 | Tier 2 | Tier 3 | Tier 4 |
| --- | --- | --- | --- | --- |
| macOS | Verified VideoToolbox shared-pool surface when strict mode justifies the extra playback machinery | AVPlayer output with direct Core Video-to-Metal import and unknown upstream provenance | GPU conversion/blit | CPU upload |
| Windows | D3D12-aware Media Foundation output on Windows 11 | Decoder-owned D3D11 texture through D3D11On12 | Media Engine `TransferVideoFrame` GPU blit | CPU upload |

For normal playback, the broad platform player may remain the default even
when a stricter tier exists. “Highest theoretical throughput” is not the same
as “best default”: replacing AVPlayer or Media Engine also assumes
responsibility for demuxing, seeking, adaptive streams, audio synchronization,
rate changes, discontinuities, and decoder recovery. A strict backend should
earn that complexity with measurements.

### Typed path evidence

Do not use one enum to compress all stages into a single optimistic label.
Represent the independently knowable facts:

```rust
enum DecodeResidency {
    HardwareSharedPool,
    HardwareUnverified,
    Software,
    Unknown,
}

enum CompositorImport {
    BorrowedNativeSurface,
    GpuBlit,
    CpuUpload,
}

enum PresentationPath {
    WgpuComposited,
    NativeOverlay,
}
```

The current macOS AVPlayer route should report
`(Unknown, BorrowedNativeSurface, WgpuComposited)`. A verified VideoToolbox
route may report `(HardwareSharedPool, BorrowedNativeSurface,
WgpuComposited)`. These states make unproved claims unrepresentable while
still recognizing that the Core Video-to-Metal half is genuinely direct.

The present Windows Media Engine route should report
`(HardwareUnverified, GpuBlit, WgpuComposited)` unless Media Foundation exposes
stronger decoder evidence. A successful decoder-owned D3D11/D3D12 path should
report `BorrowedNativeSurface`; hardware status remains a separate fact.

The frame's typed semantic descriptor must also retain coded/visible size,
crop, chroma siting, bit depth, range, matrix, primaries, transfer function,
HDR metadata, presentation timestamp, duration, and playback epoch. Native
handles stay inside platform modules. This prevents a “zero-copy” optimization
from silently producing the wrong color, crop, frame after seek, or lifetime.

### Direct YUV sampling

Preserving NV12/P010 to the final composition shader is the right default.
It avoids a full-frame intermediate RGB allocation and lets crop, scaling,
YUV conversion, transfer conversion, tone mapping, and editor composition be
combined in one render pass. A native video processor or wgpu
`ExternalTexture` may implement this stage where it is faster and semantically
equivalent, but the common contract should remain typed planes plus color
metadata so P010/HDR and platform-specific fallbacks do not disappear.

### Overlay/direct-scanout is a separate optimization

Native layers and hardware overlays may save composition bandwidth and power,
but they cannot be the universal representation of inline editor video.
Chromium's Windows
[`OverlayProcessor`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/components/viz/service/display/overlay_processor_win.cc)
uses eligibility and fallback strategies, and even fullscreen promotion must
account for controls or captions above the video. Neomacs should similarly
promote only a simple eligible rectangle and fall back immediately when text,
arbitrary clipping, opacity, filters, non-supported transforms, screenshots,
or color requirements demand wgpu composition.

## Fallback policy

Expose policy separately from observed path:

- `RequireDirectSurface`: reject both GPU blits and CPU uploads.
- `AllowGpuBlit`: preserve GPU residency while accepting one visible copy.
- `AllowCpuUpload`: compatibility mode; log and count every use.

For Neomacs's performance-oriented default, `AllowGpuBlit` is the better floor.
CPU upload should be opt-in or an explicit last-resort compatibility setting,
not an invisible fallback. Every tier transition should preserve playback
position and epoch, produce one structured diagnostic, and avoid retry loops
on every frame.

## Recommended implementation order

1. Split decoder provenance, compositor import, and presentation path in the
   shared model; keep the existing public `VideoId` session API stable.
2. Bind every native lease to actual GPU completion. On macOS, capture retained
   Core Video/Metal objects in the HAL drop callback. On Windows, carry the
   Media Foundation/D3D fence handoff in the lease.
3. Drive macOS AVPlayer frame selection from the compositor's display deadline
   and keep its currently direct plane import.
4. Add the decoder-owned D3D11 Source Reader path and recover once to the
   existing Media Engine GPU blit when negotiation/import fails.
5. Prototype D3D12-aware Media Foundation output on Windows 11 behind a runtime
   capability and telemetry gate; promote it only after multi-vendor testing.
6. Add a raw VideoToolbox backend only if strict-policy demand or measurements
   justify taking ownership of the lower-level playback work.
7. Consider native overlay promotion last, after the sampled path is correct,
   because overlay eligibility is a compositor concern rather than a decoder
   abstraction.

## Risks and required gates

- **Unsafe native import:** wgpu HAL wrapping trusts Neomacs to provide the
  exact device, descriptor, state, plane, and lifetime. Keep each unsafe block
  in a small platform importer and test it on real Intel, AMD, NVIDIA, and
  Apple GPU configurations.
- **Windows version and driver spread:** direct D3D12 Media Foundation
  synchronization is Windows 11-only; D3D11On12 resource unwrapping requires
  Windows 10 2004 or later; individual decoders and drivers may still reject
  shared NV12/P010 resources. Capability failure must demote the session once,
  not crash or repeatedly rebuild it.
- **Playback-stack scope:** raw VideoToolbox or low-level Source Reader/MFT
  paths move timing, A/V sync, seeking, streaming, discontinuity, and recovery
  work into Neomacs. Implement them as optional tiers behind the same session
  API and compare measured wins before making them defaults.
- **Resource starvation:** retaining decoder frames until GPU completion can
  exhaust the decoder pool. Bound in-flight frames, consume the newest useful
  frame, drop stale frames before import, and never reuse a native surface
  based only on elapsed time.
- **Color correctness:** direct import is not a win if limited/full range,
  chroma siting, HDR transfer, or display conversion is lost. Exercise SDR,
  HDR10/PQ, HLG, 8-bit, 10-bit, odd crop, rotation, and resolution-change
  vectors.
- **Protected video:** protected playback often cannot expose a sampleable
  decoder surface to an ordinary wgpu render pass. Treat a protected native
  layer as a separate capability and return a typed unsupported result when
  policy forbids it.
- **Device loss and adapter mismatch:** invalidate all frame leases and rebuild
  the platform decoder/importer when the wgpu device generation changes. Never
  import across adapter identities and assume it remains zero-copy.

## Performance acceptance criteria

No implementation should acquire a `zero-copy` label merely because it uses a
hardware decoder. Measure and expose at least:

- hardware-decode active/inactive;
- shared decoder/client pool true/false/unknown;
- direct plane imports, GPU blits, and CPU uploads per frame;
- pixel-format negotiation and fallbacks (P010 -> NV12 -> BGRA);
- pool allocations, reuse, backpressure, and in-flight high-water mark;
- missed presentation deadlines and dropped frames;
- CPU time, GPU time, memory bandwidth, and power for representative 4K/HDR
  streams.

The strict mode should reject `GpuBlit` and `CpuUpload`; the compatible default
may fall back but must make that fallback observable.
