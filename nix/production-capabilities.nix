{
  lib,
  pkgs,
  source,
}:
let
  workspaceManifest = builtins.fromTOML (builtins.readFile (source + "/Cargo.toml"));
  manifest = workspaceManifest.workspace.metadata.neomacs-production-capabilities;
  knownCargoCapabilities = [
    "video"
    "webview"
  ];
  platform =
    if pkgs.stdenv.isLinux then
      "linux"
    else if pkgs.stdenv.isDarwin then
      "darwin"
    else
      throw "Neomacs has no production capability profile for ${pkgs.stdenv.hostPlatform.system}";
  profile = manifest.${platform};
  cargoFeatures = profile.cargo-features;
  videoBackend = profile.video-backend;
  unknownFeatures = lib.subtractLists knownCargoCapabilities cargoFeatures;
in
assert lib.assertMsg (
  manifest.schema-version == 1
) "unsupported Neomacs production capability schema";
assert lib.assertMsg (
  unknownFeatures == [ ]
) "unknown Neomacs production Cargo capabilities: ${lib.concatStringsSep ", " unknownFeatures}";
assert lib.assertMsg (builtins.elem videoBackend [
  "none"
  "dynamic-gstreamer"
]) "unknown Neomacs production video backend: ${videoBackend}";
assert lib.assertMsg (
  videoBackend != "dynamic-gstreamer" || builtins.elem "video" cargoFeatures
) "dynamic-gstreamer requires the video Cargo capability";
{
  inherit cargoFeatures videoBackend;
}
