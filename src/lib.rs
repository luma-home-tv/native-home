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

pub mod control_reads;

pub mod control_capability;

pub mod control_row_presentation;

pub mod pointer_speed;

pub mod pointer_size;

pub mod description_level;

pub mod keyboard;

pub mod keyboard_observer;

pub mod sound_reads;

pub mod locale_reads;

pub mod app_artwork;

pub mod broker_protocol;
pub mod broker_transport;
pub mod broker_probe;
pub mod broker_server;
pub mod broker_context;
