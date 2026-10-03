use std::time::{Duration, Instant};

use gpui::{
    canvas, div, fill, point, px, size, App, Bounds, DispatchPhase, ElementId, HitboxBehavior,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseExitEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, ScrollHandle, ScrollWheelEvent, Styled, Task, Window,
};

use crate::Theme;

const HIDE_DELAY: Duration = Duration::from_millis(1200);
const FADE_DURATION: Duration = Duration::from_millis(180);

#[derive(Default)]
struct ScrollbarState {
    offset: Pixels,
    drag: Option<Pixels>,
    last_activity: Option<Instant>,
    hide_task: Option<Task<()>>,
}

impl ScrollbarState {
    fn reveal(&mut self, window: &Window, cx: &App) {
        self.last_activity = Some(Instant::now());
        self.hide_task = Some(window.spawn(cx, async move |cx| {
            cx.background_executor().timer(HIDE_DELAY).await;
            let _ = cx.update(|window, _| window.refresh());
        }));
    }

    fn opacity(&self, now: Instant, auto_hide: bool, hovered: bool, reduce_motion: bool) -> f32 {
        if !auto_hide || hovered || self.drag.is_some() {
            return 1.0;
        }
        let Some(activity) = self.last_activity else {
            return 0.0;
        };
        let elapsed = now.saturating_duration_since(activity);
        if elapsed < HIDE_DELAY {
            return 1.0;
        }
        if reduce_motion {
            return 0.0;
        }
        1.0 - ((elapsed - HIDE_DELAY).as_secs_f32() / FADE_DURATION.as_secs_f32()).min(1.0)
    }
}

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
                    let state =
                        window.use_keyed_state("scrollbar-state", cx, |_, _| ScrollbarState {
                            offset: handle.offset().y,
                            ..Default::default()
                        });
                    state.update(cx, |state, cx| {
                        if state.offset != handle.offset().y {
                            state.offset = handle.offset().y;
                            state.reveal(window, cx);
                        }
                    });
                    Some((hitbox, thumb, top, travel, state))
                },
                move |bounds, state, window, cx| {
                    let Some((hitbox, thumb, top, travel, state)) = state else {
                        return;
                    };
                    let hovered = hitbox.is_hovered(window);
                    let opacity = state.read(cx).opacity(
                        Instant::now(),
                        crate::platform::auto_hide_scrollbars(cx),
                        hovered,
                        cx.reduce_motion(),
                    );
                    if hovered || state.read(cx).drag.is_some() {
                        window.paint_quad(fill(bounds, theme.surface_hover));
                    }
                    if opacity > 0.0 {
                        let mut color = theme.muted_dark;
                        color.a *= opacity;
                        window.paint_quad(fill(thumb, color).corner_radii(px(3.0)));
                    }
                    if opacity > 0.0 && opacity < 1.0 {
                        window.request_animation_frame();
                    }

                    let handle = paint_handle.clone();
                    let down_hitbox = hitbox.clone();
                    let down_state = state.clone();
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
                        down_state.update(cx, |state, cx| {
                            state.drag = Some(grab);
                            state.reveal(window, cx);
                        });
                        scroll_to_pointer(&handle, event.position.y - grab - top, travel);
                        window.refresh();
                        cx.stop_propagation();
                    });

                    let handle = paint_handle.clone();
                    let move_hitbox = hitbox.clone();
                    let move_state = state.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture {
                            return;
                        }
                        if hovered != move_hitbox.is_hovered(window) {
                            move_state.update(cx, |state, cx| state.reveal(window, cx));
                            window.refresh();
                        }
                        let Some(grab) = move_state.read(cx).drag else {
                            return;
                        };
                        if event.dragging() {
                            scroll_to_pointer(&handle, event.position.y - grab - top, travel);
                            cx.stop_propagation();
                        } else {
                            move_state.update(cx, |state, cx| {
                                state.drag = None;
                                state.reveal(window, cx);
                            });
                        }
                        window.refresh();
                    });

                    let up_state = state.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture
                            && event.button == MouseButton::Left
                            && up_state.read(cx).drag.is_some()
                        {
                            up_state.update(cx, |state, cx| {
                                state.drag = None;
                                state.reveal(window, cx);
                            });
                            window.refresh();
                            cx.stop_propagation();
                        }
                    });

                    let exit_state = state.clone();
                    window.on_mouse_event(move |_: &MouseExitEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture && hovered {
                            exit_state.update(cx, |state, cx| state.reveal(window, cx));
                            window.refresh();
                        }
                    });

                    // The track is a sibling of the viewport, so forward its wheel events.
                    let line_height = window.line_height();
                    window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
                            state.update(cx, |state, cx| state.reveal(window, cx));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_hide_waits_then_fades_and_stays_hidden() {
        let now = Instant::now();
        let mut state = ScrollbarState::default();
        assert_eq!(state.opacity(now, true, false, false), 0.0);
        state.last_activity = Some(now);
        assert_eq!(state.opacity(now + HIDE_DELAY / 2, true, false, false), 1.0);
        assert_eq!(
            state.opacity(now + HIDE_DELAY + FADE_DURATION / 2, true, false, false),
            0.5
        );
        assert_eq!(
            state.opacity(now + HIDE_DELAY + FADE_DURATION, true, false, false),
            0.0
        );
        assert_eq!(
            state.opacity(now + Duration::from_secs(60), true, false, false),
            0.0
        );
    }

    #[test]
    fn system_preference_and_interaction_keep_the_thumb_visible() {
        let now = Instant::now();
        let mut state = ScrollbarState::default();
        assert_eq!(state.opacity(now, false, false, false), 1.0);
        assert_eq!(state.opacity(now, true, true, false), 1.0);
        state.drag = Some(px(8.0));
        assert_eq!(state.opacity(now, true, false, false), 1.0);
    }

    #[test]
    fn reduced_motion_hides_without_fading() {
        let now = Instant::now();
        let state = ScrollbarState {
            last_activity: Some(now),
            ..Default::default()
        };
        assert_eq!(state.opacity(now + HIDE_DELAY / 2, true, false, true), 1.0);
        assert_eq!(state.opacity(now + HIDE_DELAY, true, false, true), 0.0);
    }
}
