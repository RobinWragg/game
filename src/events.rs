use crate::prelude::*;

pub enum Event {
    LeftClickPressed(Vec2),
    LeftClickReleased(Vec2),
    MousePos(Vec2),
}

#[derive(Default)]
pub struct EventMgr {
    events_for_this_frame: VecDeque<Event>,
    events_for_next_frame: VecDeque<Event>,
}

impl EventMgr {
    pub fn push(&mut self, event: Event) {
        self.events_for_next_frame.push_back(event)
    }

    pub fn begin_frame(&mut self) {
        debug_assert!(self.events_for_this_frame.is_empty());
        std::mem::swap(
            &mut self.events_for_this_frame,
            &mut self.events_for_next_frame,
        );
        debug_assert!(self.events_for_next_frame.is_empty());
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.events_for_this_frame.pop_front()
    }
}
