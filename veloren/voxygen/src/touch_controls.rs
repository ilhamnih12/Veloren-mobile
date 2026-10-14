use crate::{
    game_input::GameInput,
    window::{AnalogGameInput, Event},
};
use vek::*;
use winit::event::{Touch, TouchPhase};

#[derive(Clone, Copy, Debug)]
pub struct TouchButtonRect {
    pub label: &'static str,
    pub input: GameInput,
    pub center: Vec2<f32>,
    pub radius: f32,
    pub active_touch_id: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct TouchOverlayQuad {
    pub min: Vec2<f32>,
    pub max: Vec2<f32>,
    pub color: Rgba<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualJoystick {
    pub origin: Vec2<f32>,
    pub current: Vec2<f32>,
    pub radius: f32,
    pub deadzone: f32,
    pub touch_id: Option<u64>,
}

impl VirtualJoystick {
    pub fn new(origin: Vec2<f32>, radius: f32, deadzone: f32) -> Self {
        Self {
            origin,
            current: origin,
            radius,
            deadzone,
            touch_id: None,
        }
    }

    pub fn axis_value(&self) -> Vec2<f32> {
        if self.touch_id.is_none() {
            return Vec2::zero();
        }
        let delta = self.current - self.origin;
        let dist = delta.magnitude();
        if dist < self.deadzone * self.radius || self.radius <= f32::EPSILON {
            Vec2::zero()
        } else {
            let clamped = (dist / self.radius).min(1.0);
            delta.normalized() * clamped
        }
    }
}

pub struct TouchControls {
    pub enabled: bool,
    pub screen_size: Vec2<f32>,
    pub left_stick: VirtualJoystick,
    pub right_stick: VirtualJoystick,
    pub buttons: Vec<TouchButtonRect>,
    last_digital_move: [bool; 4],
}

impl Default for TouchControls {
    fn default() -> Self { Self::new(Vec2::new(1920.0, 1080.0)) }
}

impl TouchControls {
    pub fn new(screen_size: Vec2<f32>) -> Self {
        let buttons = vec![
            TouchButtonRect { label: "ATK", input: GameInput::Primary, center: Vec2::new(0.88, 0.76), radius: 0.065, active_touch_id: None },
            TouchButtonRect { label: "ALT", input: GameInput::Secondary, center: Vec2::new(0.78, 0.84), radius: 0.052, active_touch_id: None },
            TouchButtonRect { label: "JUMP", input: GameInput::Jump, center: Vec2::new(0.92, 0.58), radius: 0.055, active_touch_id: None },
            TouchButtonRect { label: "DODGE", input: GameInput::Roll, center: Vec2::new(0.79, 0.66), radius: 0.052, active_touch_id: None },
            TouchButtonRect { label: "GLIDE", input: GameInput::Glide, center: Vec2::new(0.84, 0.48), radius: 0.045, active_touch_id: None },
            TouchButtonRect { label: "USE", input: GameInput::Interact, center: Vec2::new(0.72, 0.54), radius: 0.048, active_touch_id: None },
            TouchButtonRect { label: "BAG", input: GameInput::Inventory, center: Vec2::new(0.82, 0.10), radius: 0.042, active_touch_id: None },
            TouchButtonRect { label: "MAP", input: GameInput::Map, center: Vec2::new(0.89, 0.10), radius: 0.042, active_touch_id: None },
            TouchButtonRect { label: "ESC", input: GameInput::Escape, center: Vec2::new(0.95, 0.10), radius: 0.040, active_touch_id: None },
        ];
        Self {
            enabled: true,
            screen_size,
            left_stick: VirtualJoystick::new(Vec2::new(0.16, 0.74), 0.11, 0.15),
            right_stick: VirtualJoystick::new(Vec2::new(0.62, 0.74), 0.11, 0.10),
            buttons,
            last_digital_move: [false; 4],
        }
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.screen_size = Vec2::new((width.max(1)) as f32, (height.max(1)) as f32);
    }

    pub fn handle_touch(&mut self, touch: Touch, out_events: &mut Vec<Event>) {
        if !self.enabled { return; }
        let norm_pos = Vec2::new(
            (touch.location.x as f32 / self.screen_size.x).clamp(0.0, 1.0),
            (touch.location.y as f32 / self.screen_size.y).clamp(0.0, 1.0),
        );
        match touch.phase {
            TouchPhase::Started => {
                for btn in &mut self.buttons {
                    if btn.active_touch_id.is_none() && norm_pos.distance(btn.center) <= btn.radius * 1.15 {
                        btn.active_touch_id = Some(touch.id);
                        out_events.push(Event::InputUpdate(btn.input, true));
                        return;
                    }
                }
                if norm_pos.x < 0.45 && self.left_stick.touch_id.is_none() {
                    self.left_stick.touch_id = Some(touch.id);
                    self.left_stick.origin = norm_pos;
                    self.left_stick.current = norm_pos;
                } else if norm_pos.x >= 0.45 && self.right_stick.touch_id.is_none() {
                    self.right_stick.touch_id = Some(touch.id);
                    self.right_stick.origin = norm_pos;
                    self.right_stick.current = norm_pos;
                }
            },
            TouchPhase::Moved => {
                if self.left_stick.touch_id == Some(touch.id) {
                    self.left_stick.current = norm_pos;
                    let axis = self.left_stick.axis_value();
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementX(axis.x)));
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementY(-axis.y)));
                    self.emit_digital_movement(axis, out_events);
                } else if self.right_stick.touch_id == Some(touch.id) {
                    let prev = self.right_stick.current;
                    self.right_stick.current = norm_pos;
                    let delta = (norm_pos - prev) * self.screen_size;
                    out_events.push(Event::CursorPan(delta * 0.35));
                    let axis = self.right_stick.axis_value();
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraX(axis.x)));
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraY(axis.y)));
                }
            },
            TouchPhase::Ended | TouchPhase::Cancelled => {
                for btn in &mut self.buttons {
                    if btn.active_touch_id == Some(touch.id) {
                        btn.active_touch_id = None;
                        out_events.push(Event::InputUpdate(btn.input, false));
                        return;
                    }
                }
                if self.left_stick.touch_id == Some(touch.id) {
                    self.left_stick.touch_id = None;
                    self.left_stick.current = self.left_stick.origin;
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementX(0.0)));
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::MovementY(0.0)));
                    self.emit_digital_movement(Vec2::zero(), out_events);
                } else if self.right_stick.touch_id == Some(touch.id) {
                    self.right_stick.touch_id = None;
                    self.right_stick.current = self.right_stick.origin;
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraX(0.0)));
                    out_events.push(Event::AnalogGameInput(AnalogGameInput::CameraY(0.0)));
                }
            },
        }
    }

    fn emit_digital_movement(&mut self, axis: Vec2<f32>, out_events: &mut Vec<Event>) {
        let next = [axis.y < -0.35, axis.y > 0.35, axis.x < -0.35, axis.x > 0.35];
        let inputs = [GameInput::MoveForward, GameInput::MoveBack, GameInput::MoveLeft, GameInput::MoveRight];
        for i in 0..4 {
            if next[i] != self.last_digital_move[i] {
                self.last_digital_move[i] = next[i];
                out_events.push(Event::InputUpdate(inputs[i], next[i]));
            }
        }
    }
}
