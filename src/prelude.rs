pub use crate::debugger::Debugger;
pub use crate::events::{Event, EventMgr};
pub use crate::gpu::{Gpu, Mesh};
pub use glam::{
    f32::{Mat4, Vec2, Vec3, Vec4},
    Vec2Swizzles, Vec3Swizzles, Vec4Swizzles,
};
pub use rand::prelude::*;
pub use std::collections::{HashMap, HashSet, VecDeque};
pub use std::f32::consts::SQRT_2;
pub use std::time::{Duration, Instant};

pub fn transform_2d(ndc: &Vec2, ndc_to_your_gui: &Mat4) -> Vec2 {
    let mouse_ndc = Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
    (*ndc_to_your_gui * mouse_ndc).xy()
}
