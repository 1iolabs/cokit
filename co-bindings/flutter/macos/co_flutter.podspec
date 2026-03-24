#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint co_flutter.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'co_flutter'
  s.version          = '0.1.0'
  s.summary          = 'A new Flutter FFI plugin project.'
  s.description      = <<-DESC
A new Flutter FFI plugin project.
                       DESC
  s.homepage         = 'http://example.com'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }

  # This will ensure the source files in Classes/ are included in the native
  # builds of apps using this FFI plugin. Podspec does not support relative
  # paths, so Classes contains a forwarder C file that relatively imports
  # `../src/*` so that the C sources can be shared among all target platforms.
  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'

  # If your plugin requires a privacy manifest, for example if it collects user
  # data, update the PrivacyInfo.xcprivacy file to describe your plugin's
  # privacy impact, and then uncomment this line. For more information,
  # see https://developer.apple.com/documentation/bundleresources/privacy_manifest_files
  # s.resource_bundles = {'co_flutter_privacy' => ['Resources/PrivacyInfo.xcprivacy']}

  s.dependency 'FlutterMacOS'

  s.platform = :osx, '10.11'
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'OTHER_LDFLAGS' => '-lco_bindings',
    'LIBRARY_SEARCH_PATHS' => '"$(BUILT_PRODUCTS_DIR)"',
  }
  s.swift_version = '5.0'

  s.script_phase = {
    :name => 'Build Rust library',
    :execution_position => :before_compile,
    :shell_path => '/bin/sh',
    :output_files => ['$(BUILT_PRODUCTS_DIR)/libco_bindings.dylib'],
    :script => <<-SCRIPT
      set -e

      export PATH="$HOME/.cargo/bin:$PATH"

      # Resolve symlinks (Flutter's .symlinks/) to get the real paths
      SRCROOT="$(cd "$PODS_TARGET_SRCROOT" && pwd -P)"
      # podspec dir (macos/) -> flutter/ -> co-bindings/ -> co-sdk/
      WORKSPACE_ROOT="$(cd "$SRCROOT/../../.." && pwd -P)"

      CARGO_ARGS="-p co-bindings -F frb"
      if [ "$CONFIGURATION" = "Release" ] || [ "$CONFIGURATION" = "Profile" ]; then
        CARGO_ARGS="$CARGO_ARGS --release"
        CARGO_PROFILE="release"
      else
        CARGO_PROFILE="debug"
      fi

      # Map Xcode architectures to Rust targets
      LIPO_INPUTS=""
      for ARCH in $ARCHS; do
        case "$ARCH" in
          x86_64) RUST_TARGET="x86_64-apple-darwin" ;;
          arm64)  RUST_TARGET="aarch64-apple-darwin" ;;
          *)      echo "error: unsupported architecture $ARCH" >&2; exit 1 ;;
        esac

        echo "Building Rust library ($CARGO_PROFILE) for $RUST_TARGET"
        cd "$WORKSPACE_ROOT"
        cargo build $CARGO_ARGS --target "$RUST_TARGET"

        LIPO_INPUTS="$LIPO_INPUTS $WORKSPACE_ROOT/target/$RUST_TARGET/$CARGO_PROFILE/libco_bindings.dylib"
      done

      echo "Creating universal binary"
      lipo -create $LIPO_INPUTS -output "${BUILT_PRODUCTS_DIR}/libco_bindings.dylib"
    SCRIPT
  }
end
