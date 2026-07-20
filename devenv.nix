{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  bazelDevTestPath = pkgs.lib.makeBinPath [
    pkgs.bash
    pkgs.coreutils
    pkgs.diffutils
    pkgs.findutils
    pkgs.gawk
    pkgs.gnugrep
    pkgs.gnused
    pkgs.lean4
  ];
  bazelNixRc = pkgs.writeTextFile {
    name = "marlin-agent-core-bazelrc";
    destination = "/share/marlin-agent-core/bazelrc";
    text = ''
      build --@rules_rust//rust/settings:extra_exec_rustc_flag=-Lnative=${pkgs.libiconv}/lib
      build --@rules_rust//rust/settings:extra_rustc_flag=-Lnative=${pkgs.libiconv}/lib
      ${lib.optionalString pkgs.stdenv.isDarwin "build --macos_minimum_os=10.12"}
      test:dev --test_env=PATH=${bazelDevTestPath}
    '';
  };
in
{
  # https://devenv.sh/basics/
  env.GREET = "devenv";
  env.USE_BAZEL_VERSION = "9.2.0";

  # https://devenv.sh/packages/
  tasks."bazel:deps-prepare" = {
    description = "Network-enabled refresh of the pinned Bazel dependency vendor tree";
    cwd = config.devenv.root;
    exec = ''
      export CARGO_HOME="$PWD/.data/cargo-home"
      mkdir -p "$CARGO_HOME"
      bazelisk mod deps \
        --config=deps_prepare \
        --repo_env=CARGO_BAZEL_REPIN=1 \
        --repo_env=CARGO_BAZEL_REPIN_ONLY=marlin_crates
      bazelisk vendor \
        --config=deps_prepare \
        --vendor_dir=bazel/vendor \
        --lockfile_mode=error
    '';
  };

  tasks."bazel:offline-test" = {
    description = "Run the full Bazel suite with fail-closed offline dependency policy";
    cwd = config.devenv.root;
    exec = "bazelisk test --config=dev //...";
  };

  packages = [
    bazelNixRc
    pkgs.bazelisk
    pkgs.buildifier
    pkgs.libiconv
    pkgs.lean4
    pkgs.pkg-config
    pkgs.protobuf
    pkgs.just
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    # Ensure rust can link python library
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
    ];
  };

  # https://devenv.sh/languages/
  # languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;
  dotenv.enable = true;
  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  # https://devenv.sh/basics/
  enterShell = "";

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = "";

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
    shellcheck.enable = true;
    nixfmt.enable = true;
  };
  # See full reference at https://devenv.sh/reference/options/
}
