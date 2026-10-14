use crate::{
    game_input::GameInput,
    window::{AnalogGameInput, Event},
};

pub mod android_keycodes {
    pub const AKEYCODE_DPAD_UP: i32 = 19;
    pub const AKEYCODE_DPAD_DOWN: i32 = 20;
    pub const AKEYCODE_DPAD_LEFT: i32 = 21;
    pub const AKEYCODE_DPAD_RIGHT: i32 = 22;
    pub const AKEYCODE_BUTTON_A: i32 = 96;
    pub const AKEYCODE_BUTTON_B: i32 = 97;
    pub const AKEYCODE_BUTTON_X: i32 = 99;
    pub const AKEYCODE_BUTTON_Y: i32 = 100;
    pub const AKEYCODE_BUTTON_L1: i32 = 102;
    pub const AKEYCODE_BUTTON_R1: i32 = 103;
    pub const AKEYCODE_BUTTON_L2: i32 = 104;
    pub const AKEYCODE_BUTTON_R2: i32 = 105;
    pub const AKEYCODE_BUTTON_THUMBL: i32 = 106;
    pub const AKEYCODE_BUTTON_THUMBR: i32 = 107;
    pub const AKEYCODE_BUTTON_START: i32 = 108;
    pub const AKEYCODE_BUTTON_SELECT: i32 = 109;
    pub const AKEYCODE_BUTTON_MODE: i32 = 110;
}

pub mod android_axes {
    pub const AMOTION_EVENT_AXIS_X: i32 = 0;
    pub const AMOTION_EVENT_AXIS_Y: i32 = 1;
    pub const AMOTION_EVENT_AXIS_Z: i32 = 11;
    pub const AMOTION_EVENT_AXIS_RZ: i32 = 14;
    pub const AMOTION_EVENT_AXIS_HAT_X: i32 = 15;
    pub const AMOTION_EVENT_AXIS_HAT_Y: i32 = 16;
    pub const AMOTION_EVENT_AXIS_LTRIGGER: i32 = 17;
    pub const AMOTION_EVENT_AXIS_RTRIGGER: i32 = 18;
}

#[derive(Debug, Default)]
pub struct AndroidGamepadMapper {
    pub deadzone: f32,
}

impl AndroidGamepadMapper {
    pub fn new() -> Self { Self { deadzone: 0.12 } }

    pub fn map_keycode(keycode: i32) -> Option<GameInput> {
        use android_keycodes::*;
        match keycode {
            AKEYCODE_BUTTON_A => Some(GameInput::Jump),
            AKEYCODE_BUTTON_B => Some(GameInput::Roll),
            AKEYCODE_BUTTON_X => Some(GameInput::Interact),
            AKEYCODE_BUTTON_Y => Some(GameInput::Glide),
            AKEYCODE_BUTTON_R1 | AKEYCODE_BUTTON_R2 => Some(GameInput::Primary),
            AKEYCODE_BUTTON_L1 => Some(GameInput::Secondary),
            AKEYCODE_BUTTON_L2 => Some(GameInput::Block),
            AKEYCODE_BUTTON_THUMBL => Some(GameInput::ToggleWalk),
            AKEYCODE_BUTTON_THUMBR => Some(GameInput::FreeLook),
            AKEYCODE_BUTTON_START => Some(GameInput::Escape),
            AKEYCODE_BUTTON_SELECT => Some(GameInput::Inventory),
            AKEYCODE_BUTTON_MODE => Some(GameInput::Map),
            AKEYCODE_DPAD_UP => Some(GameInput::SwapLoadout),
            AKEYCODE_DPAD_DOWN => Some(GameInput::Sit),
            AKEYCODE_DPAD_LEFT => Some(GameInput::PreviousSlot),
            AKEYCODE_DPAD_RIGHT => Some(GameInput::NextSlot),
            _ => None,
        }
    }

    pub fn handle_axis_event(&self, axis: i32, raw_value: f32, out_events: &mut Vec<Event>) {
        use android_axes::*;
        let val = if raw_value.abs() < self.deadzone { 0.0 } else { raw_value.clamp(-1.0, 1.0) };
        match axis {
            AMOTION_EVENT_AXIS_X => out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementX(val))),
            AMOTION_EVENT_AXIS_Y => out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementY(-val))),
            AMOTION_EVENT_AXIS_Z => out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraX(val))),
            AMOTION_EVENT_AXIS_RZ => out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraY(val))),
            _ => {},
        }
    }
}
