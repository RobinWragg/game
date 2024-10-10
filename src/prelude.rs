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
    LeftClickPressed(Vec2),
    LeftClickReleased(Vec2),
    MousePos(Vec2),
}

pub fn transform_2d(pos: &Vec2, mat: &Mat4) -> Vec2 {
    let pos4 = Vec4::new(pos.x, pos.y, 0.0, 1.0);
    (*mat * pos4).xy()
}
