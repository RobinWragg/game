use crate::prelude::*;

#[derive(Default)]
pub struct User {
    mouse_ndc: Vec2,
    pub left_button_down: bool,
}

impl User {
    pub fn set_mouse_ndc(&mut self, mouse_ndc: &Vec2) {
        self.mouse_ndc = *mouse_ndc;
    }

    pub fn mouse(&self, ndc_to_your_gui: &Mat4) -> Vec2 {
        let mouse_ndc = Vec4::new(self.mouse_ndc.x, self.mouse_ndc.y, 0.0, 1.0);
        (*ndc_to_your_gui * mouse_ndc).xy()
    }
}
