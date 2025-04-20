pub use crate::debugger::Debugger;
pub use crate::gpu::{Gpu, Mesh};
pub use glam::{
    f32::{Mat4, Vec2, Vec3, Vec4},
    Vec2Swizzles, Vec3Swizzles, Vec4Swizzles,
};
pub use rand::prelude::*;
pub use std::collections::{HashMap, HashSet, VecDeque};
pub use std::f32::consts::SQRT_2;
pub use std::time::{Duration, Instant};

pub enum Event {
    TotalTime(f64),
    LeftClickPressed(Vec2),
    LeftClickReleased(Vec2),
    RightClickPressed(Vec2),
    RightClickReleased(Vec2),
    MousePos(Vec2),
}
