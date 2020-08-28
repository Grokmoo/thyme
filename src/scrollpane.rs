use crate::{Frame, widget::WidgetBuilder, Rect, Point};

/// A [`WidgetBuilder`](struct.WidgetBuilder.html) specifically for creating scrollpanes.
///
/// Scrollpanes can have fairly complex behavior, and can include optional horizontal and vertical scrollbars.
/// Scrollbars are, by default, only shown when the content size exceeds the pane's inner size.
/// There is also a [`scrollpane method`](struct.Frame.html#method.scrollpane) on `Frame` as a convenience for simple cases.
///
/// Once you are finished setting up the scrollpane, you call [`children`](#method.children) to add children to the scrollpane
/// content and add the widget to the frame.  Note that the children are added to the scrollpane's content, *not* directly to
/// the scrollpane itself.

// TODO add theme yaml sample here
pub struct ScrollpaneBuilder<'a> {
    builder: WidgetBuilder<'a>,
    state: ScrollpaneState,
}

struct ScrollpaneState {
    content_id: String,
}

impl<'a> ScrollpaneBuilder<'a> {
    pub(crate) fn new(builder: WidgetBuilder<'a>, content_id: &str) -> ScrollpaneBuilder<'a> {
        ScrollpaneBuilder {
            builder,
            state: ScrollpaneState {
                content_id: content_id.to_string(),
            }
        }
    }

    /// Consumes this builder to create a scrollpane.  Calls the specified `children` closure
    /// to add children to the scrollpane.
    pub fn children<F: FnOnce(&mut Frame)>(self, children: F) {
        let mut min_scroll = Point::default();
        let mut max_scroll = Point::default();
        let mut delta = Point::default();

        let content_id = self.state.content_id;

        let (ui, result) = self.builder.finish_with(
            Some(scrollpane_content(&content_id, children, &mut min_scroll, &mut max_scroll, &mut delta))
        );

        // set the scroll every frame to bound it, in case it was modified externally
        ui.modify(&content_id, |state| {
            let min = min_scroll + state.scroll;
            let max = max_scroll + state.scroll;

            state.scroll = (state.scroll + delta + result.moved).max(min).min(max);
        });
    }
}

fn scrollpane_content<'a, F: FnOnce(&mut Frame) + 'a>(
    content_id: &'a str,
    children: F,
    min_scroll: &'a mut Point,
    max_scroll: &'a mut Point,
    delta: &'a mut Point,
) -> impl FnOnce(&mut Frame) + 'a {
    move |ui| {
        let mut content_bounds = Rect::default();

        // TODO if horizontal and/or vertical scrollbars aren't present,
        // change the scrollpane content size to fill up the available space

        ui.start("content")
        .id(content_id)
        .trigger_layout(&mut content_bounds)
        .clip(content_bounds)
        .children(children);

        let content_min = content_bounds.pos;
        let content_max = content_bounds.pos + content_bounds.size;

        let pane_bounds = ui.parent_max_child_bounds();
        let pane_min = pane_bounds.pos;
        let pane_max = pane_bounds.pos + pane_bounds.size;

        let mut delta_scroll = Point::default();
        // check whether to show horizontal scrollbar
        if pane_min.x < content_min.x || pane_max.x > content_max.x {
            ui.start("scrollbar_horizontal")
            .children(|ui| {
                let mut right_rect = Rect::default();
                let result = ui.start("right")
                .enabled(pane_max.x > content_max.x)
                .trigger_layout(&mut right_rect).finish();
                if result.clicked {
                    delta_scroll.x -= 10.0;
                }

                let mut left_rect = Rect::default();
                let result = ui.start("left")
                .enabled(pane_min.x < content_min.x)
                .trigger_layout(&mut left_rect).finish();
                if result.clicked {
                    delta_scroll.x += 10.0;
                }

                // compute size and position for main scroll button
                let start_frac = ((content_min.x - pane_min.x) / pane_bounds.size.x).max(0.0);
                let width_frac = content_bounds.size.x / pane_bounds.size.x;

                // assume left button starts at 0,0 within the parent widget
                let min_x = left_rect.size.x;
                let max_x = right_rect.pos.x - left_rect.pos.x;
                let pos_x = min_x + start_frac * (max_x - min_x);
                let pos_y = 0.0;
                let size_x = width_frac * (max_x - min_x);
                let size_y = left_rect.size.y;

                let result = ui.start("scroll")
                .size(size_x, size_y)
                .pos(pos_x, pos_y)
                .finish();

                if result.pressed {
                    delta_scroll.x -= result.moved.x / width_frac;
                }
            });
        }

        // check whether to show vertical scrollbar
        if pane_min.y < content_min.y || pane_max.y > content_max.y {
            ui.start("scrollbar_vertical")
            .children(|ui| {
                let mut top_rect = Rect::default();
                let result = ui.start("up")
                .enabled(pane_min.y < content_min.y)
                .trigger_layout(&mut top_rect).finish();
                if result.clicked {
                    delta_scroll.y += 10.0;
                }

                let mut bot_rect = Rect::default();
                let result = ui.start("down")
                .enabled(pane_max.y > content_max.y)
                .trigger_layout(&mut bot_rect).finish();
                if result.clicked {
                    delta_scroll.y -= 10.0;
                }

                // compute size and position for main scroll button
                let start_frac = ((content_min.y - pane_min.y) / pane_bounds.size.y).max(0.0);
                let height_frac = content_bounds.size.y / pane_bounds.size.y;

                // assume top button starts at 0,0 within the parent widget
                let min_y = top_rect.size.y;
                let max_y = bot_rect.pos.y - top_rect.pos.y;
                let pos_y = min_y + start_frac * (max_y - min_y);
                let pos_x = 0.0;
                let size_y = height_frac * (max_y - min_y);
                let size_x = top_rect.size.x;

                let result = ui.start("scroll")
                .size(size_x, size_y)
                .pos(pos_x, pos_y)
                .finish();

                if result.pressed {
                    delta_scroll.y -= result.moved.y / height_frac;
                }
            });
        }

        *min_scroll = content_max - pane_max;
        *max_scroll = content_min - pane_min;
        *delta = delta_scroll;
    }
}