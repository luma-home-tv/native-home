//! A separate Home application, using the shared native UI without Plex/player dependencies.
//! Navigation, read-only controls and capability-gated normal application launching.

pub mod model;
pub mod preview;
pub mod shell;
pub mod strings;

pub mod runtime;

pub mod preferences;

pub mod native_probe;

pub mod live;

pub mod actions;

pub mod drawer;

pub mod session;

pub mod identity;

pub mod clock;

pub mod performance;
