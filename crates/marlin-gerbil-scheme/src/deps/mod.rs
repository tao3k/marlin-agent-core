//! Gerbil package dependency bootstrap for the deck runtime.

mod action;
mod bootstrap;
mod cli;
mod config;
mod constants;
mod error;
mod fs;
mod process;

pub use action::GerbilDepsAction;
pub use cli::{run_gerbil_deps_cli, run_gerbil_deps_from_args};
pub use config::GerbilDepsConfig;
pub use error::GerbilDepsError;

pub(crate) use constants::{
    GERBIL_BIN_ENV, GERBIL_CELLAR_ENV, GERBIL_GCC_ENV, GERBIL_GSC_ENV, GERBIL_MACOS_SDK_ENV,
    GERBIL_POO_PACKAGE, GERBIL_POO_PROVIDER_URL, GERBIL_PREFIX_ENV, GERBIL_UTILS_MODULE_PACKAGE,
    GERBIL_UTILS_PROVIDER_PACKAGE, GERBIL_UTILS_PROVIDER_URL, MARLIN_GERBIL_PKG_CACHE_ENV,
};
