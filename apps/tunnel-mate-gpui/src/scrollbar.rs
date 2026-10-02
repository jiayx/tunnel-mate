use gpui::{
    canvas, div, fill, point, px, size, Bounds, DispatchPhase, ElementId, HitboxBehavior,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, ScrollHandle, ScrollWheelEvent, Styled,
};

use crate::Theme;

/// Place after the scrollable content in a relative container with the same height.
pub(crate) fn scrollbar(
    id: impl Into<ElementId>,
    theme: Theme,
    handle: ScrollHandle,
) -> impl IntoElement {
    let paint_handle = handle.clone();
    div()
        .id(id)
        .absolute()
        .top_0()
        .right_0()
        .w(px(12.0))
        .h_full()
        .child(
            canvas(
                move |bounds, window, cx| {
                    let max = handle.max_offset().y;
                    let height = bounds.size.height - px(4.0);
                    if max <= px(0.0) || height <= px(0.0) {
                        return None;
                    }
                    let viewport = handle.bounds().size.height;
                    let thumb_height = (height * (viewport / (viewport + max)))
                        .max(px(28.0))
                        .min(height);
                    let travel = height - thumb_height;
                    let top = bounds.top() + px(2.0);
                    let offset = (-handle.offset().y / max).clamp(0.0, 1.0);
                    let thumb = Bounds::new(
                        point(bounds.left() + px(3.0), top + travel * offset),
                        size(px(6.0), thumb_height),
                    );
                    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
                    let drag = window.use_keyed_state("scrollbar-drag", cx, |_, _| None::<Pixels>);
                    Some((hitbox, thumb, top, travel, drag))
                },
                move |bounds, state, window, cx| {
                    let Some((hitbox, thumb, top, travel, drag)) = state else {
                        return;
                    };
                    let hovered = hitbox.is_hovered(window);
                    if hovered || drag.read(cx).is_some() {
                        window.paint_quad(fill(bounds, theme.surface_hover));
                    }
                    window.paint_quad(fill(thumb, theme.muted_dark).corner_radii(px(3.0)));

                    let handle = paint_handle.clone();
                    let down_hitbox = hitbox.clone();
                    let down_drag = drag.clone();
                    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble
                            || event.button != MouseButton::Left
                            || !down_hitbox.is_hovered(window)
                        {
                            return;
                        }
                        let grab = if thumb.contains(&event.position) {
                            event.position.y - thumb.top()
                        } else {
                            thumb.size.height / 2.0
                        };
                        down_drag.update(cx, |drag, _| *drag = Some(grab));
                        scroll_to_pointer(&handle, event.position.y - grab - top, travel);
                        window.refresh();
                        cx.stop_propagation();
                    });

                    let handle = paint_handle.clone();
                    let move_hitbox = hitbox.clone();
                    let move_drag = drag.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture {
                            return;
                        }
                        if hovered != move_hitbox.is_hovered(window) {
                            window.refresh();
                        }
                        let Some(grab) = *move_drag.read(cx) else {
                            return;
                        };
                        if event.dragging() {
                            scroll_to_pointer(&handle, event.position.y - grab - top, travel);
                            cx.stop_propagation();
                        } else {
                            move_drag.update(cx, |drag, _| *drag = None);
                        }
                        window.refresh();
                    });

                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture
                            && event.button == MouseButton::Left
                            && drag.read(cx).is_some()
                        {
                            drag.update(cx, |drag, _| *drag = None);
                            window.refresh();
                            cx.stop_propagation();
                        }
                    });

                    // The track is a sibling of the viewport, so forward its wheel events.
                    let line_height = window.line_height();
                    window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
                            let mut offset = paint_handle.offset();
                            offset.y = (offset.y + event.delta.pixel_delta(line_height).y)
                                .clamp(-paint_handle.max_offset().y, px(0.0));
                            paint_handle.set_offset(offset);
                            window.refresh();
                            cx.stop_propagation();
                        }
                    });
                },
            )
            .size_full(),
        )
}

fn scroll_to_pointer(handle: &ScrollHandle, position: Pixels, travel: Pixels) {
    if travel > px(0.0) {
        let mut offset = handle.offset();
        offset.y = -handle.max_offset().y * (position / travel).clamp(0.0, 1.0);
        handle.set_offset(offset);
    }
}
