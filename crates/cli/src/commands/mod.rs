mod activate;
mod alias;
mod bin;
pub mod clean;
mod completions;
pub mod debug;
mod diagnose;
mod install;
mod list;
mod list_remote;
mod migrate;
mod outdated;
mod pin;
pub mod plugin;
mod regen;
mod run;
mod setup;
mod status;
mod unalias;
mod uninstall;
mod unpin;
mod upgrade;

pub use activate::*;
pub use alias::*;
pub use bin::*;
pub use clean::*;
pub use completions::*;
pub use diagnose::*;
pub use install::*;
pub use list::*;
pub use list_remote::*;
pub use migrate::*;
pub use outdated::*;
pub use pin::*;
pub use regen::*;
pub use run::*;
pub use setup::*;
pub use status::*;
pub use unalias::*;
pub use uninstall::*;
pub use unpin::*;
pub use upgrade::*;
