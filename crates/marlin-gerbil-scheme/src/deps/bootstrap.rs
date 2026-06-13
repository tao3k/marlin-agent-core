//! Bootstrap actions for fetching, linking, building, and verifying Gerbil packages.

use super::{
    GERBIL_CELLAR_ENV, GERBIL_GCC_ENV, GERBIL_GSC_ENV, GERBIL_MACOS_SDK_ENV, GERBIL_POO_PACKAGE,
    GERBIL_POO_PROVIDER_URL, GERBIL_UTILS_MODULE_PACKAGE, GERBIL_UTILS_PROVIDER_PACKAGE,
    GERBIL_UTILS_PROVIDER_URL, GerbilDepsConfig, GerbilDepsError,
    fs::{ensure_symlink, package_destination, remove_existing_symlink, require_dir, require_file},
    process::{BootstrapCommand, clone_or_update, prepend_library_path, prepend_path},
};
use std::{fs, path::Path, path::PathBuf, process::Command};

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
        fs::create_dir_all(&self.cache_dir).map_err(|error| {
            GerbilDepsError::message(format!(
                "failed to create {}: {error}",
                self.cache_dir.display()
            ))
        })?;
        clone_or_update(
            GERBIL_UTILS_PROVIDER_URL,
            &self.cache_dir.join("gerbil-utils"),
        )?;
        clone_or_update(GERBIL_POO_PROVIDER_URL, &self.cache_dir.join("gerbil-poo"))?;
        Ok(())
    }

    pub(super) fn link_packages(&self) -> Result<(), GerbilDepsError> {
        let gerbil_utils = self.cache_dir.join("gerbil-utils");
        let gerbil_poo = self.cache_dir.join("gerbil-poo");
        require_dir(&gerbil_utils, "gerbil-utils checkout")?;
        require_dir(&gerbil_poo, "gerbil-poo checkout")?;

        self.link_package(GERBIL_UTILS_MODULE_PACKAGE, &gerbil_utils)?;
        self.link_package(GERBIL_UTILS_PROVIDER_PACKAGE, &gerbil_utils)?;
        self.link_package(GERBIL_POO_PACKAGE, &gerbil_poo)?;
        Ok(())
    }

    pub(super) fn build_packages(&self) -> Result<(), GerbilDepsError> {
        self.gxpkg_command()
            .args(["build", "--global-env", GERBIL_UTILS_PROVIDER_PACKAGE])
            .run("build gerbil-utils")?;
        self.gxpkg_command()
            .args(["build", "--global-env", GERBIL_POO_PACKAGE])
            .run("build clan/poo")
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
        self.verify_package_files()
    }

    fn link_package(&self, package: &str, source: &Path) -> Result<(), GerbilDepsError> {
        let destination = package_destination(&self.home_dir, package);
        remove_existing_symlink(&destination)?;
        self.gxpkg_command()
            .args(["link", "-g", package])
            .arg(source)
            .run(format!("link {package}"))
    }

    fn verify_package_files(&self) -> Result<(), GerbilDepsError> {
        require_file(
            &package_destination(
                &self.home_dir,
                &format!("{GERBIL_UTILS_PROVIDER_PACKAGE}.manifest"),
            ),
            "gerbil-utils manifest",
        )?;
        require_file(
            &self
                .home_dir
                .join(".gerbil")
                .join("pkg")
                .join("clan")
                .join("poo.manifest"),
            "clan/poo manifest",
        )?;
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
        Ok(())
    }

    fn gxpkg_command(&self) -> BootstrapCommand {
        self.gerbil_command(self.gerbil_bin.join("gxpkg"))
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
