use crate::{color, glass};
use gpui::{Rgba, WindowAppearance};

/// Colors shared by every surface, including native input and modal states.
#[derive(Clone, Copy)]
pub(crate) struct Theme {
    pub app_bg: Rgba,
    pub sidebar_bg: Rgba,
    pub surface: Rgba,
    pub surface_hover: Rgba,
    pub border: Rgba,
    pub border_soft: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub muted_dark: Rgba,
    pub primary: Rgba,
    pub primary_hover: Rgba,
    pub primary_text: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub danger: Rgba,
    pub selected: Rgba,
    pub selected_border: Rgba,
    pub accent: Rgba,
    pub danger_bg: Rgba,
    pub selection: Rgba,
    pub toggle_off: Rgba,
    pub backdrop: Rgba,
}

impl Theme {
    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        let dark = matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let pick = |dark_color, light_color| color(if dark { dark_color } else { light_color });
        Self {
            app_bg: pick(0x10141c, 0xf6f8fb),
            sidebar_bg: pick(0x151b26, 0xeef2f7),
            surface: pick(0x1a2230, 0xffffff),
            surface_hover: pick(0x253249, 0xe8eef7),
            border: pick(0x334157, 0xc8d1df),
            border_soft: pick(0x252f40, 0xe0e6ef),
            text: pick(0xeaf0f9, 0x182335),
            muted: pick(0xa7b5cb, 0x52627a),
            muted_dark: pick(0x91a1bb, 0x617089),
            primary: color(0x2764e7),
            primary_hover: color(0x1f53c2),
            primary_text: color(0xffffff),
            success: pick(0x63cda7, 0x14734f),
            warning: pick(0xd2a85e, 0x855700),
            danger: pick(0xf08e97, 0xb52c3c),
            selected: pick(0x1c2d46, 0xe7effe),
            selected_border: pick(0x43699b, 0x90b2ec),
            accent: pick(0xa8c8ff, 0x2458b8),
            danger_bg: pick(0x332631, 0xffedf0),
            selection: glass(0x3979f4, if dark { 0.38 } else { 0.22 }),
            toggle_off: pick(0x354154, 0xb7c3d5),
            backdrop: glass(0x080c14, if dark { 0.72 } else { 0.30 }),
        }
    }
}
