{
  description = "Flake providing Veloren, a multiplayer voxel RPG written in Rust.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci = {
      url = "github:90-008/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.parts.follows = "parts";
      inputs.dream2nix.follows = "d2n";
      inputs.crane.follows = "crane";
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    d2n = {
      url = "github:NeuralModder/dream2nix/git-fetcher-no-shallow";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane/v0.21.0";
      flake = false;
    };
  };

  outputs = inp: let
    lib = inp.nci.inputs.nixpkgs.lib;

    git = let
      sourceInfo = inp.self.sourceInfo;
      shortRev = lib.strings.concatStrings (lib.lists.take 8 (lib.strings.stringToCharacters (sourceInfo.rev or sourceInfo.dirtyRev)));
    in {
      version = "/" + shortRev + "/" + toString sourceInfo.lastModified;
    };

    filteredSource = let
      pathsToIgnore = [
        "flake.nix"
        "flake.lock"
        "nix"
        "assets"
        "README.md"
        "CONTRIBUTING.md"
        "CHANGELOG.md"
        "CODE_OF_CONDUCT.md"
        ".github"
        ".gitlab"
      ];
      ignorePaths = path: type: let
        split = lib.splitString "/" path;
        actual = lib.drop 4 split;
        _path = lib.concatStringsSep "/" actual;
      in
        lib.all (n: ! (lib.hasPrefix n _path)) pathsToIgnore;
    in
      builtins.path {
        name = "veloren-source";
        path = toString ./.;
        # filter out unnecessary paths
        filter = ignorePaths;
      };
  in
    inp.parts.lib.mkFlake {inputs = inp;} {
      imports = [inp.nci.flakeModule];
      systems = ["x86_64-linux"];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: let
        checkIfLfsIsSetup = checkFile: ''
          checkFile="${checkFile}"
          result="$(${pkgs.file}/bin/file --mime-type $checkFile)"
          if [ "$result" = "$checkFile: image/jpeg" ]; then
            echo "Git LFS seems to be setup properly."
            true
          else
            echo "
              Git Large File Storage (git-lfs) has not been set up correctly.
              Most common reasons:
                - git-lfs was not installed before cloning this repository.
                - This repository was not cloned from the primary GitLab mirror.
                - The GitHub mirror does not support LFS.
              See the book at https://book.veloren.net/ for details.
              Run 'nix-shell -p git git-lfs --run \"git lfs install --local && git lfs fetch && git lfs checkout\"'
              or 'nix shell nixpkgs#git-lfs nixpkgs#git -c sh -c \"git lfs install --local && git lfs fetch && git lfs checkout\"'.
            "
            false
          fi
        '';
        assets = pkgs.runCommand "veloren-assets" {} ''
          mkdir $out
          ln -sf ${./assets} $out/assets
          ${checkIfLfsIsSetup "$out/assets/voxygen/background/bg_main.jpg"}
        '';
        wrapWithAssets = old:
          pkgs.runCommand
          old.name
          {
            meta = old.meta or {};
            passthru =
              (old.passthru or {})
              // {
                unwrapped = old;
              };
            nativeBuildInputs = [pkgs.makeWrapper];
          }
          ''
            cp -rs --no-preserve=mode,ownership ${old} $out
            wrapProgram $out/bin/* \
              --set VELOREN_ASSETS ${assets} \
              --set VELOREN_GIT_VERSION "${git.version}" \
          '';
        veloren-common-env = {
          # We don't add in any information here because otherwise anything
          # that depends on common will be recompiled. We will set these in
          # our wrapper instead.
          VELOREN_GIT_VERSION = "/0/0";
          VELOREN_USERDATA_STRATEGY = "system";
        };
        voxygenOut = config.nci.outputs."veloren-voxygen";
        serverCliOut = config.nci.outputs."veloren-server-cli";
        harnessOut = config.nci.outputs."bastion-harness";
      in {
        packages.veloren-voxygen = wrapWithAssets voxygenOut.packages.release;
        packages.veloren-voxygen-dev = wrapWithAssets voxygenOut.packages.dev;
        packages.veloren-voxygen-tlto = wrapWithAssets voxygenOut.packages.release-thinlto;
        packages.veloren-server-cli = wrapWithAssets serverCliOut.packages.release;
        packages.veloren-server-cli-dev = wrapWithAssets serverCliOut.packages.dev;
        packages.veloren-server-cli-tlto = wrapWithAssets serverCliOut.packages.release-thinlto;
        packages.default = config.packages."veloren-voxygen";
        # APEX-T1.1.03/.06 (DET-BLD-019/029/032): first-class UNWRAPPED harness
        # package in the tracked `verify` profile. Deliberately NOT wrapWithAssets
        # — T1.1 exports the bare build artifact and permits metadata-only
        # execution; asset/LFS completeness is APEX-T1.2's closure (claiming it
        # here would be T1.1-BLOCK-ASSET-CLAIM). The same derivation doubles as
        # the flake check so `nix flake check` proves the package still builds.
        packages.bastion-harness = harnessOut.packages.verify;
        checks.bastion-harness-package = harnessOut.packages.verify;

        # APEX-T1.3.02: the LOCAL-REPRO variant — same package, but the
        # FINAL derivation must execute locally and can never be satisfied
        # by substitution (packet policy 3: immutable dependency store
        # reuse stays allowed; only the harness derivation itself is
        # forced local). Locale/TZ frozen; sccache/incremental are already
        # neutralized in the base derivation env (T1.1.03-.05).
        packages.bastion-harness-repro = harnessOut.packages.verify.overrideAttrs (old: {
          allowSubstitutes = false;
          preferLocalBuild = true;
          TZ = "UTC";
          LC_ALL = "C";
        });

        # APEX-T1.3.11: known-good/known-bad reproducibility canary
        # derivations. NEVER dependencies of any production output — they
        # exist so the smoke's comparator provably detects representative
        # nondeterminism (stable must pass `--rebuild`; the other three
        # must each fail it for their own mechanism).
        packages.apex-repro-canary-stable = (import ./nix/apex/repro-canaries.nix { inherit pkgs; }).stable;
        packages.apex-repro-canary-time = (import ./nix/apex/repro-canaries.nix { inherit pkgs; }).time;
        packages.apex-repro-canary-random = (import ./nix/apex/repro-canaries.nix { inherit pkgs; }).random;
        packages.apex-repro-canary-tmppath = (import ./nix/apex/repro-canaries.nix { inherit pkgs; }).tmppath;

        devShells.default = config.nci.outputs."veloren".devShell.overrideAttrs (old: {
          VELOREN_ASSETS = "";
          shellHook = ''
            ${checkIfLfsIsSetup "$PWD/assets/voxygen/background/bg_main.jpg"}
            if [ $? -ne 0 ]; then
              exit 1
            fi
            export VELOREN_ASSETS="$PWD/assets"
            export VELOREN_GIT_VERSION="${git.version}"
          '';
        });

        nci.projects."veloren" = {
          export = false;
          path = filteredSource;
        };
        # APEX-T1.1.03-.05 (DET-BLD-019/029): bastion-harness as a locked NCI
        # crate in the tracked `verify` profile (root Cargo.toml [profile.verify],
        # DET-BLD-031(a) semantics: overflow-checks + debug-assertions ON in the
        # cert lane). The derivation env makes the build self-identifying and
        # ambient-free (APEX-T1.1 packet §6.4):
        #  - BASTION_SOURCE_REVISION/SOURCE_DATE_EPOCH from flake sourceInfo →
        #    build.rs DeclaredCertified stamping (no .git, no wall clock; a
        #    DIRTY checkout yields dirtyRev ("<hex>-dirty"), which build.rs
        #    REJECTS as non-40-hex — the packet's dirty-rejection, fail-closed
        #    at the stamping layer);
        #  - BASTION_BUILD_LANE=apex-nix-v1 makes missing identity a BUILD
        #    ERROR (T1.1-BLOCK-UNKNOWN-REVISION), not a silent fallback;
        #  - RUSTC_WRAPPER="" neutralizes the repo-wide sccache wrapper (cargo:
        #    empty string resets a configured wrapper) and CARGO_INCREMENTAL=0
        #    removes incremental cache state (T1.1-BLOCK-AMBIENT-WRAPPER);
        #  - mold comes from the pinned closure, not the host.
        nci.crates."bastion-harness" = rec {
          profiles = {
            verify.runTests = false;
          };
          depsDrvConfig = {
            mkDerivation.nativeBuildInputs = [pkgs.mold];
            env =
              veloren-common-env
              // {
                RUSTC_WRAPPER = "";
                CARGO_INCREMENTAL = "0";
              };
          };
          drvConfig = {
            mkDerivation = depsDrvConfig.mkDerivation;
            env =
              depsDrvConfig.env
              // {
                BASTION_SOURCE_REVISION =
                  inp.self.sourceInfo.rev or inp.self.sourceInfo.dirtyRev or "missing-source-revision";
                SOURCE_DATE_EPOCH = toString inp.self.sourceInfo.lastModified;
                BASTION_BUILD_LANE = "apex-nix-v1";
              };
          };
        };
        nci.crates."veloren-server-cli" = rec {
          profiles = {
            release.features = ["default-publish"];
            release.runTests = false;
            dev.features = ["default-publish"];
            dev.runTests = false;
            release-thinlto.features = ["default-publish"];
            release-thinlto.runTests = false;
          };
          depsDrvConfig.mkDerivation.nativeBuildInputs = [pkgs.mold];
          drvConfig = {
            mkDerivation = depsDrvConfig.mkDerivation;
            env = veloren-common-env;
          };
        };
        nci.crates."veloren-voxygen" = rec {
          profiles = {
            release.features = ["default-publish"];
            release.runTests = false;
            dev.features = ["default-publish"];
            dev.runTests = false;
            release-thinlto.features = ["default-publish"];
            release-thinlto.runTests = false;
          };
          runtimeLibs = with pkgs; [
            wayland
            wayland-protocols
            libX11
            libXi
            libxcb
            libXcursor
            libXrandr
            libxkbcommon
            shaderc.lib
            udev
            alsa-lib
            vulkan-loader
            stdenv.cc.cc.lib
          ];
          depsDrvConfig = {
            env =
              veloren-common-env
              // {
                SHADERC_LIB_DIR = "${pkgs.shaderc.lib}/lib";
              };
            mkDerivation = {
              buildInputs = with pkgs; [
                alsa-lib
                libxkbcommon
                udev
                libxcb

                fontconfig
              ];
              nativeBuildInputs = with pkgs; [
                python3
                pkg-config
                cmake
                gnumake
                mold
              ];
            };
          };
          drvConfig = {
            env =
              depsDrvConfig.env
              // {
                dontUseCmakeConfigure = true;
                VOXYGEN_NULL_SOUND_PATH = ./assets/voxygen/audio/null.ogg;
              };
            mkDerivation =
              depsDrvConfig.mkDerivation
              // {
                prePatch = ''
                                sed -i 's:"../../../assets/voxygen/audio/null.ogg":env!("VOXYGEN_NULL_SOUND_PATH"):' \
                  voxygen/src/audio/soundcache.rs
                '';
              };
            rust-crane.buildFlags = ["--bin=veloren-voxygen"];
          };
        };
      };
    };
}
