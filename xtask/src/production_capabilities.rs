use serde::Deserialize;

const WORKSPACE_MANIFEST: &str = include_str!("../../Cargo.toml");
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CargoCapability {
    Video,
    Webview,
}

impl CargoCapability {
    pub(crate) const fn feature_name(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Webview => "webview",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProductionVideoBackend {
    None,
    DynamicGstreamer,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
struct PlatformCapabilities {
    cargo_features: Vec<CargoCapability>,
    video_backend: ProductionVideoBackend,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CapabilityManifest {
    schema_version: u32,
    linux: PlatformCapabilities,
    darwin: PlatformCapabilities,
    windows: PlatformCapabilities,
}

#[derive(Debug, Deserialize)]
struct WorkspaceMetadata {
    #[serde(rename = "neomacs-production-capabilities")]
    production_capabilities: CapabilityManifest,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    metadata: WorkspaceMetadata,
}

#[derive(Debug, Deserialize)]
struct CargoManifest {
    workspace: Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostPlatform {
    Linux,
    Darwin,
    Windows,
}

impl HostPlatform {
    fn current() -> Result<Self, String> {
        if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Ok(Self::Darwin)
        } else if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else {
            Err(format!(
                "no production capability profile for target OS {}",
                std::env::consts::OS
            ))
        }
    }

    fn select(self, manifest: CapabilityManifest) -> PlatformCapabilities {
        match self {
            Self::Linux => manifest.linux,
            Self::Darwin => manifest.darwin,
            Self::Windows => manifest.windows,
        }
    }
}

/// A validated distribution build policy.
///
/// The workspace manifest is the serialization seam shared with Nix.  Past
/// that seam, callers see enums rather than feature/backend strings, and the
/// constructor rejects combinations that cannot produce a runnable package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProductionCapabilities(PlatformCapabilities);

impl ProductionCapabilities {
    pub(crate) fn for_host() -> Result<Self, String> {
        let cargo: CargoManifest = toml::from_str(WORKSPACE_MANIFEST)
            .map_err(|error| format!("invalid production capability metadata: {error}"))?;
        let manifest = cargo.workspace.metadata.production_capabilities;
        if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(format!(
                "unsupported production capability schema {}; expected {}",
                manifest.schema_version, SUPPORTED_SCHEMA_VERSION
            ));
        }

        let capabilities = HostPlatform::current()?.select(manifest);
        if capabilities.video_backend == ProductionVideoBackend::DynamicGstreamer
            && !capabilities
                .cargo_features
                .contains(&CargoCapability::Video)
        {
            return Err("dynamic-gstreamer requires the typed `video` Cargo capability".to_owned());
        }
        Ok(Self(capabilities))
    }

    pub(crate) fn cargo_features(&self) -> &[CargoCapability] {
        &self.0.cargo_features
    }

    pub(crate) fn cargo_feature_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.cargo_features()
            .iter()
            .copied()
            .map(CargoCapability::feature_name)
    }

    pub(crate) const fn video_backend(&self) -> ProductionVideoBackend {
        self.0.video_backend
    }
}
