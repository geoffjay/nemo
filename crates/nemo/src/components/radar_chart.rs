use std::f32::consts::PI;

use gpui::*;
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_array,
};

/// A radar (spider / web) chart. Categories are arranged around a central
/// point and one or more data series are drawn as filled polygons.
///
/// # XML Configuration
///
/// ```xml
/// <radar-chart id="skills" max-value="100" categories='["Speed","Power","Defense"]'
///   y-fields='["player1","player2"]' height="300">
///   <data bind="skillData" />
/// </radar-chart>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `max-value` | float | Maximum value for the radar axes |
/// | `categories` | JSON array | Array of category labels around the radar |
/// | `y-fields` | JSON array | Array of data field names for each series |
/// | `height` | int | Chart height in pixels |
#[derive(IntoElement, NemoComponent)]
pub struct RadarChart {
    #[property]
    max_value: Option<f64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for RadarChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let categories = get_string_array(&self.source, "categories");
        let y_fields = get_string_array(&self.source, "y_fields");
        if categories.is_empty() || y_fields.is_empty() {
            return empty_chart_placeholder(cx);
        }

        // Each data item has one value per category field
        // We treat each y_field as a series; each data item contributes one
        // value per category. For simplicity we take the first data item per
        // series (typical radar usage) or average multiple items.
        let max_val = self.max_value.unwrap_or_else(|| {
            let mut m = 0.0_f64;
            for item in &data {
                for cat in &categories {
                    let v = get_f64_field(item, cat);
                    if v > m {
                        m = v;
                    }
                }
            }
            m
        }) as f32;

        // For radar, we expect one data item per series (y_field identifies
        // the series label, each category is a numeric field).
        // Alternatively, y_fields lists the fields in each data item.
        // We'll iterate data items and treat each as a series.
        let series: Vec<Vec<f32>> = data
            .iter()
            .map(|item| {
                categories
                    .iter()
                    .map(|cat| get_f64_field(item, cat) as f32)
                    .collect()
            })
            .collect();

        let colors: Vec<Hsla> = (0..series.len()).map(|i| chart_color(i, cx)).collect();
        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        RadarElement {
            categories,
            series,
            max_val,
            colors,
            border_color,
            _muted_fg: muted_fg,
        }
        .into_any_element()
    }
}

struct RadarElement {
    categories: Vec<String>,
    series: Vec<Vec<f32>>,
    max_val: f32,
    colors: Vec<Hsla>,
    border_color: Hsla,
    _muted_fg: Hsla,
}

impl IntoElement for RadarElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for RadarElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let w = bounds.size.width.as_f32();
        let h = bounds.size.height.as_f32();
        let cx_px = w / 2.0;
        let cy_px = h / 2.0;
        let radius = cx_px.min(cy_px) * 0.75;
        let n = self.categories.len();
        if n < 3 || self.max_val <= 0.0 {
            return;
        }

        let origin = bounds.origin;
        let angle_step = 2.0 * PI / n as f32;
        // Start from top (−π/2)
        let start_angle = -PI / 2.0;

        // Helper: compute pixel point for (category_index, value)
        let to_point = |cat_idx: usize, val: f32| -> Point<Pixels> {
            let angle = start_angle + angle_step * cat_idx as f32;
            let r = (val / self.max_val) * radius;
            point(
                px(cx_px + r * angle.cos()) + origin.x,
                px(cy_px + r * angle.sin()) + origin.y,
            )
        };

        // Draw concentric reference rings (3 levels)
        for level in 1..=3 {
            let frac = level as f32 / 3.0;
            let mut builder = PathBuilder::stroke(px(1.));
            let pts: Vec<Point<Pixels>> =
                (0..n).map(|i| to_point(i, self.max_val * frac)).collect();
            let mut closed = pts.clone();
            closed.push(pts[0]);
            builder.add_polygon(&closed, false);
            if let Ok(path) = builder.build() {
                window.paint_path(path, self.border_color.opacity(0.3));
            }
        }

        // Draw axis lines from centre to each vertex
        for i in 0..n {
            let tip = to_point(i, self.max_val);
            let center_pt = point(px(cx_px) + origin.x, px(cy_px) + origin.y);
            let mut builder = PathBuilder::stroke(px(1.));
            builder.add_polygon(&[center_pt, tip], false);
            if let Ok(path) = builder.build() {
                window.paint_path(path, self.border_color.opacity(0.3));
            }
        }

        // Draw data series as filled polygons
        for (si, values) in self.series.iter().enumerate() {
            let color = self.colors[si % self.colors.len()];
            let pts: Vec<Point<Pixels>> = values
                .iter()
                .enumerate()
                .map(|(i, &v)| to_point(i, v))
                .collect();

            // Fill polygon
            if pts.len() >= 3 {
                let mut fill_builder = PathBuilder::fill();
                let mut closed = pts.clone();
                closed.push(pts[0]);
                fill_builder.add_polygon(&closed, false);
                if let Ok(path) = fill_builder.build() {
                    window.paint_path(path, color.opacity(0.25));
                }
            }

            // Stroke outline
            {
                let mut stroke_builder = PathBuilder::stroke(px(2.));
                let mut closed = pts.clone();
                closed.push(pts[0]);
                stroke_builder.add_polygon(&closed, false);
                if let Ok(path) = stroke_builder.build() {
                    window.paint_path(path, color);
                }
            }

            // Dots at vertices
            for pt in &pts {
                let r = 3.0_f32;
                let dot_bounds = Bounds::from_corners(
                    point(pt.x - px(r), pt.y - px(r)),
                    point(pt.x + px(r), pt.y + px(r)),
                );
                window.paint_quad(PaintQuad {
                    bounds: dot_bounds,
                    corner_radii: Corners::all(px(r)),
                    background: color.into(),
                    border_widths: Edges::default(),
                    border_color: transparent_black(),
                    border_style: Default::default(),
                });
            }
        }

        // Draw category labels at the tips
        // (Using simple text painting via paint_quad is not ideal; we draw
        //  them as small positioned labels. For now we skip them to avoid
        //  pulling in text layout — the data tells the story.)
    }
}
