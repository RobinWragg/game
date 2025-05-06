pub use crate::debugger::Debugger;
pub use crate::gpu::*;
use crate::grid::Atom;
pub use glam::{
    f32::{Mat3, Mat4, Vec2, Vec3, Vec4},
    i32::IVec3,
    usize::USizeVec3 as UVec3,
    Vec2Swizzles, Vec3Swizzles, Vec4Swizzles,
};
use once_cell::sync::Lazy;
pub use rand::prelude::*;
pub use std::collections::{HashMap, HashSet, VecDeque};
pub use std::f32::consts::{PI, SQRT_2, TAU};
pub use std::mem::discriminant;
use std::mem::Discriminant;
use std::sync::Mutex;
pub use std::time::Instant;

pub struct Global {
    pub selected_atom_type: Atom,
    pub should_step: bool,
    pub is_playing: bool,
}

pub static GLOBAL: Lazy<Mutex<Global>> = Lazy::new(|| {
    Mutex::new(Global {
        selected_atom_type: Atom::Solid(Vec4::ZERO),
        should_step: false,
        is_playing: false,
    })
});

pub enum Event {
    LeftClickPressed(Vec2),
    LeftClickReleased(Vec2),
    RightClickPressed(Vec2),
    RightClickReleased(Vec2),
    MousePos(Vec2),
    Scroll(Vec2),
}
