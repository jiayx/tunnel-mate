use super::*;
use std::borrow::Cow;

pub(crate) struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let drawing = match path {
            "icons/tunnels" => "<rect x='3' y='4' width='18' height='6' rx='2'/><rect x='3' y='14' width='18' height='6' rx='2'/><path d='M7 7h.01M7 17h.01M12 10v4'/>",
            "icons/activity" => "<path d='M3 12h4l3-8 4 16 3-8h4'/>",
            "icons/folder" => "<path d='M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z'/>",
            "icons/settings" => "<path d='M4 6h16M4 12h16M4 18h16'/><circle cx='8' cy='6' r='2' fill='black'/><circle cx='16' cy='12' r='2' fill='black'/><circle cx='10' cy='18' r='2' fill='black'/>",
            "icons/plus" => "<path d='M12 5v14M5 12h14'/>",
            "icons/close" => "<path d='m6 6 12 12M18 6 6 18'/>",
            "icons/arrow" => "<path d='M4 12h16m-6-6 6 6-6 6'/>",
            "icons/globe" => "<circle cx='12' cy='12' r='9'/><ellipse cx='12' cy='12' rx='4' ry='9'/><path d='M3 12h18'/>",
            "icons/check" => "<path d='m5 12 4 4L19 6'/>",
            "icons/copy" => "<rect x='8' y='8' width='12' height='13' rx='2'/><path d='M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h3'/>",
            "icons/chevron" => "<path d='m9 5 7 7-7 7'/>",
            _ => return Ok(None),
        };
        Ok(Some(Cow::Owned(format!("<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'>{drawing}</svg>").into_bytes())))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

pub(crate) fn icon(path: &'static str) -> gpui::Svg {
    gpui::svg()
        .path(path)
        .size(px(16.0))
        .flex_none()
        .text_color(TEXT)
}

/// Shared button treatment and keyboard activation (Enter / Space).
fn button_base(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    let label = label.into();
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label.clone())
        .key_context("TunnelButton")
        .tab_index(0)
        .flex_none()
        .h(px(34.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(BORDER)
        .bg(SURFACE)
        .text_size(px(12.0))
        .text_color(TEXT)
        .cursor_pointer()
        .child(label)
}

pub(crate) fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    button_base(id, label)
        .hover(|style| style.bg(SURFACE_HOVER).border_color(MUTED_DARK))
        .focus(|style| style.border_color(PRIMARY))
}

pub(crate) fn primary_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    button_base(id, label)
        .bg(PRIMARY)
        .border_color(PRIMARY)
        .text_color(PRIMARY_TEXT)
        .font_weight(FontWeight::MEDIUM)
        .hover(|style| style.bg(PRIMARY_HOVER).border_color(PRIMARY_HOVER))
        .focus(|style| style.border_color(TEXT))
}

pub(crate) fn close_button(id: &'static str) -> gpui::Stateful<gpui::Div> {
    button(id, "")
        .aria_label("Close")
        .tab_index(100)
        .size(px(32.0))
        .px(px(0.0))
        .border_color(rgba(0x00000000))
        .bg(rgba(0x00000000))
        .text_color(MUTED)
        .child(icon("icons/close"))
}

pub(crate) fn section_heading(title: impl Into<SharedString>) -> gpui::Div {
    div()
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(MUTED)
        .child(title.into())
}

pub(crate) fn toggle(id: &'static str, checked: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .key_context("TunnelButton")
        .tab_index(0)
        .w(px(42.0))
        .h(px(26.0))
        .flex_none()
        .p(px(3.0))
        .rounded(px(13.0))
        .border_1()
        .border_color(if checked { PRIMARY } else { BORDER })
        .bg(if checked { PRIMARY } else { color(0x354154) })
        .cursor_pointer()
        .focus(|style| style.border_color(TEXT))
        .child(
            div()
                .size(px(18.0))
                .rounded(px(9.0))
                .bg(PRIMARY_TEXT)
                .when(checked, |dot| dot.ml(px(15.0))),
        )
}

// Bracket IPv6 literals so the port is unambiguous in connection summaries.
pub(crate) fn endpoint_label(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
