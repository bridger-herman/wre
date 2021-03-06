// state.rs
//
// Copyright (c) 2019, Univerisity of Minnesota
//
// Author: Bridger Herman (herma582@umn.edu)

//! Global state for the WebAssembly Rendering Engine
//! - Current frame information
//! - Entity manager

use std::sync::Mutex;

use crate::camera::Camera;
use crate::entity_manager::EntityManager;
use crate::render_system::{RenderSystem, WebGlContextWrapper};
use crate::script_manager::ScriptManager;
use crate::time::WreTime;
use crate::window::WindowState;

lazy_static! {
    /// Current frame information (timing, key presses, etc.)
    pub static ref WRE_TIME: Mutex<WreTime> = Mutex::new(WreTime::default());

    /// The single FPS camera
    pub static ref WRE_CAMERA: Mutex<Camera> = Mutex::new(Camera::default());

    /// Entity manager for the game
    pub static ref WRE_ENTITIES: Mutex<EntityManager> = Mutex::new(EntityManager::default());

    /// Script manager for the game
    pub static ref WRE_SCRIPTS: Mutex<ScriptManager> = Mutex::new(ScriptManager::default());

    /// Rendering system
    pub static ref WRE_RENDER_SYSTEM: Mutex<RenderSystem> = Mutex::new(RenderSystem::default());

    /// WebGL state
    pub static ref WRE_GL: Mutex<WebGlContextWrapper> = Mutex::new(WebGlContextWrapper::default());

    // The window state (paused/not paused, etc.)
    pub static ref WRE_WINDOW: Mutex<WindowState> = Mutex::new(WindowState::default());
}
