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

pub mod artwork_session_guard;
pub mod icon_presentation_proof;
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

pub mod device_info_reads;

pub mod app_artwork;

pub mod durable_icon_cache;
pub mod cache_producer_profile;
pub mod artwork_cache_preparation;
pub mod artwork_preparation_events;
pub mod artwork_cache_preparation_runner;
pub mod artwork_cache_preparation_selection;
pub mod artwork_cache_preparation_proof;
pub mod packaged_icon_reader;
pub mod packaged_icons;

pub mod artwork_cache;
pub mod artwork_gl;
pub mod artwork_installed_proof;
pub mod artwork_installed_provider;
pub mod artwork_live_proof;
pub mod artwork_probe;
pub mod artwork_runtime;
pub mod artwork_startup_probe;
pub mod artwork_textures;
pub mod artwork_transfer;
pub mod artwork_transition;
pub mod artwork_worker;
pub mod broker_bootstrap;
pub mod broker_context;
pub mod broker_installation;
pub mod broker_negative_proof;
pub mod broker_probe;
pub mod broker_protocol;
pub mod broker_server;
pub mod broker_startup_proof;
pub mod broker_transport;
pub mod startup_capability_proof;

pub mod artwork_guard_relaunch_proof;

pub mod preview_artwork_admission;

pub mod preferences_migration;
pub mod preferences_migration_proof;

pub mod artwork_closed_negative_proof;

pub mod caption_reads;
