//! Bootstrap actions for fetching, linking, building, and verifying Gerbil packages.

use super::{
    GERBIL_CELLAR_ENV, GERBIL_GCC_ENV, GERBIL_GSC_ENV, GERBIL_MACOS_SDK_ENV, GerbilDepsConfig,
    GerbilDepsError,
    fs::{ensure_symlink, require_dir, require_file},
    process::{BootstrapCommand, prepend_library_path, prepend_path},
};
use std::{fs, path::PathBuf, process::Command};

impl GerbilDepsConfig {
    pub(super) fn repair_homebrew_layout(&self) -> Result<(), GerbilDepsError> {
        if !self.requires_homebrew_repair() {
            println!(
                "skipping Homebrew Gerbil repair on platform {}",
                self.platform
            );
            return Ok(());
        }

        let cellar = self.gerbil_cellar.as_ref().ok_or_else(|| {
            GerbilDepsError::message(format!(
                "cannot infer {GERBIL_CELLAR_ENV}; pass --gerbil-cellar"
            ))
        })?;
        let prefix = self.gerbil_prefix.as_ref().ok_or_else(|| {
            GerbilDepsError::message("cannot infer GERBIL_PREFIX; pass --gerbil-prefix")
        })?;
        let sdk = self.macos_sdk.as_ref().ok_or_else(|| {
            GerbilDepsError::message(format!(
                "cannot infer {GERBIL_MACOS_SDK_ENV}; pass --macos-sdk"
            ))
        })?;
        let gsc = self.gerbil_gsc.as_ref().ok_or_else(|| {
            GerbilDepsError::message(format!("cannot infer {GERBIL_GSC_ENV}; pass --gerbil-gsc"))
        })?;
        let gcc = self.gerbil_gcc.as_ref().ok_or_else(|| {
            GerbilDepsError::message(format!("cannot infer {GERBIL_GCC_ENV}; pass --gerbil-gcc"))
        })?;

        require_dir(cellar, GERBIL_CELLAR_ENV)?;
        require_file(&self.gerbil_bin.join("gxi"), "gxi")?;
        require_file(&self.gerbil_bin.join("gxpkg"), "gxpkg")?;
        require_file(gcc, GERBIL_GCC_ENV)?;
        require_file(
            &sdk.join("usr").join("include").join("_bounds.h"),
            "_bounds.h",
        )?;
        require_file(&sdk.join("usr").join("lib").join("libz.tbd"), "libz.tbd")?;

        let prefix_bin = prefix.join("bin");
        fs::create_dir_all(&prefix_bin).map_err(|error| {
            GerbilDepsError::message(format!(
                "failed to create {}: {error}",
                prefix_bin.display()
            ))
        })?;
        ensure_symlink(&cellar.join("bin").join("gsc"), &prefix_bin.join("gsc"))?;
        ensure_symlink(
            &cellar.join("bin").join("gambuild-C"),
            &prefix_bin.join("gambuild-C"),
        )?;
        ensure_symlink(&cellar.join("include"), &prefix.join("include"))?;
        require_file(gsc, GERBIL_GSC_ENV)?;

        Ok(())
    }

    pub(super) fn fetch_packages(&self) -> Result<(), GerbilDepsError> {
        require_dir(&self.package_root, "Gerbil package root")?;
        require_file(&self.package_root.join("gerbil.pkg"), "gerbil.pkg")?;
        self.gxpkg_command()
            .args(["deps", "-i"])
            .run("install gerbil.pkg dependencies")
    }

    pub(super) fn link_packages(&self) -> Result<(), GerbilDepsError> {
        Ok(())
    }

    pub(super) fn build_packages(&self) -> Result<(), GerbilDepsError> {
        self.gxpkg_command()
            .args(["build", "-d"])
            .run("build gerbil.pkg dependencies and package")
    }

    pub(super) fn verify_packages(&self) -> Result<(), GerbilDepsError> {
        self.gxi_command()
            .args([
                "-e",
                "(import :clan/base :clan/building :clan/json :clan/cli)",
                "-e",
                "(display \"clan-deps-ok\")",
                "-e",
                "(newline)",
            ])
            .run("verify clan imports")?;
        self.gxi_command()
            .args([
                "-e",
                "(import :clan/poo/object :clan/poo/mop :clan/poo/proto)",
                "-e",
                "(display \"poo-import-ok\")",
                "-e",
                "(newline)",
            ])
            .run("verify clan/poo imports")?;
        self.gxi_command()
            .args([
                "-e",
                "(import :poo-flow/src/module-system/facade :poo-flow/src/loops/agent)",
                "-e",
                "(display \"poo-flow-import-ok\")",
                "-e",
                "(newline)",
            ])
            .run("verify poo-flow imports")?;
        self.verify_package_files()
    }

    fn verify_package_files(&self) -> Result<(), GerbilDepsError> {
        require_file(
            &self.home_dir.join(".gerbil").join("bin").join("random-run"),
            "random-run",
        )?;
        for module in ["object", "mop", "proto"] {
            require_file(
                &self
                    .home_dir
                    .join(".gerbil")
                    .join("lib")
                    .join("clan")
                    .join("poo")
                    .join(format!("{module}.o1")),
                format!("clan/poo/{module}.o1"),
            )?;
        }
        require_file(
            &self
                .home_dir
                .join(".gerbil")
                .join("lib")
                .join("poo-flow")
                .join("src")
                .join("module-system")
                .join("facade.o1"),
            "poo-flow module-system facade",
        )?;
        require_file(
            &self
                .home_dir
                .join(".gerbil")
                .join("lib")
                .join("poo-flow")
                .join("src")
                .join("loops")
                .join("agent.o1"),
            "poo-flow loops/agent",
        )?;
        Ok(())
    }

    fn gxpkg_command(&self) -> BootstrapCommand {
        let mut command = self.gerbil_command(self.gerbil_bin.join("gxpkg"));
        command.current_dir(&self.package_root);
        command
    }

    fn gxi_command(&self) -> BootstrapCommand {
        self.gerbil_command(self.gerbil_bin.join("gxi"))
    }

    fn gerbil_command(&self, program: PathBuf) -> BootstrapCommand {
        let mut command = Command::new(program);
        command.env("PATH", prepend_path(&self.gerbil_bin));
        if let Some(gsc) = &self.gerbil_gsc {
            command.env(GERBIL_GSC_ENV, gsc);
        }
        if self.requires_homebrew_repair() {
            self.configure_macos_build_env(&mut command);
        }
        BootstrapCommand::new(command)
    }

    fn configure_macos_build_env(&self, command: &mut Command) {
        if let Some(gcc) = &self.gerbil_gcc {
            command.env(GERBIL_GCC_ENV, gcc);
        }
        if let Some(sdk) = &self.macos_sdk {
            command.env(
                "CORCXXFLAGS_GAMBUILD",
                format!("-isysroot {} -foptimize-sibling-calls", sdk.display()),
            );
            command.env(
                "LDFLAGS_GAMBUILD",
                format!("-isysroot {} -Wl,-undefined,dynamic_lookup", sdk.display()),
            );
            command.env(
                "LIBRARY_PATH",
                prepend_library_path(&sdk.join("usr").join("lib")),
            );
        }
    }
}
