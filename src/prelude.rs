pub use crate::debugger::Debugger;
pub use crate::gpu::{Gpu, Mesh};
pub use glam::{
    f32::{Mat3, Mat4, Vec2, Vec3, Vec4},
    i32::IVec3,
    usize::USizeVec3 as UVec3,
    Vec2Swizzles, Vec3Swizzles, Vec4Swizzles,
};
pub use rand::prelude::*;
pub use std::collections::{HashMap, HashSet, VecDeque};
pub use std::f32::consts::{PI, SQRT_2, TAU};
pub use std::time::Instant;

pub enum Event {
    LeftClickPressed(Vec2),
    LeftClickReleased(Vec2),
    RightClickPressed(Vec2),
    RightClickReleased(Vec2),
    MousePos(Vec2),
    Scroll(Vec2),
}
