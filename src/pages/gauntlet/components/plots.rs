use dioxus::prelude::*;
use plotly::Bar;
use plotly::common::{Font, Label, Line, Mode, Title};
use plotly::configuration::{Configuration, DisplayModeBar};
use plotly::layout::{Legend, Margin};
use plotly::{Layout, Plot, Scatter, layout::Axis};

const FONT_FAMILY: &str = "'Jost', ui-sans-serif, system-ui, sans-serif";
const AXIS_TITLE_FONT_SIZE: usize = 13;
const TICK_FONT_SIZE: usize = 11;
const LEGEND_FONT_SIZE: usize = 11;
const HOVER_FONT_SIZE: usize = 12;
const PLOT_BG_COLOR: &str = "rgba(17, 24, 39, 0)"; // transparent
const PAPER_BG_COLOR: &str = "rgba(17, 24, 39, 0)"; // transparent
const GRID_COLOR: &str = "rgba(75, 85, 99, 0.3)"; // gray-600 with opacity
const AXIS_LINE_COLOR: &str = "rgba(107, 114, 128, 0.5)"; // gray-500 with opacity
const TEXT_COLOR: &str = "#e5e7eb"; // gray-200
const LEGEND_BG_COLOR: &str = "rgba(31, 41, 55, 0.95)"; // gray-800 with opacity
const HOVER_BG_COLOR: &str = "rgba(31, 41, 55, 0.95)"; // gray-800 near-opaque
const HOVER_BORDER_COLOR: &str = "rgba(75, 85, 99, 0.5)"; // gray-600
const TRACE_COLORS: [&str; 8] = [
    "#a78bfa", // soft violet
    "#3ec9a7", // teal
    "#e07a5f", // terracotta/coral
    "#f2cc8f", // warm gold
    "#4cc9f0", // sky blue
    "#f472b6", // pink
    "#34d399", // mint green
    "#fb923c", // orange
];

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimeUnit {
    Ticks,
    Seconds,
}

fn rand_id() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Math::random() * 1_000_000.0) as u32
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0 // Server-side placeholder
    }
}

#[allow(dead_code)]
/// Creates a base layout with consistent dark theme styling
fn create_base_layout() -> Layout {
    Layout::new()
        .paper_background_color(PAPER_BG_COLOR)
        .plot_background_color(PLOT_BG_COLOR)
        .font(
            Font::new()
                .family(FONT_FAMILY)
                .color(TEXT_COLOR)
                .size(TICK_FONT_SIZE),
        )
        .margin(Margin::new().left(50).right(20).top(30).bottom(50).pad(4))
        .auto_size(true)
        .hover_mode(plotly::layout::HoverMode::XUnified)
        .hover_label(
            Label::new()
                .background_color(HOVER_BG_COLOR)
                .border_color(HOVER_BORDER_COLOR)
                .font(
                    Font::new()
                        .family(FONT_FAMILY)
                        .size(HOVER_FONT_SIZE)
                        .color(TEXT_COLOR),
                )
                .name_length(-1),
        )
        .legend(
            Legend::new()
                .background_color(LEGEND_BG_COLOR)
                .border_width(0)
                .font(
                    Font::new()
                        .family(FONT_FAMILY)
                        .size(LEGEND_FONT_SIZE)
                        .color(TEXT_COLOR),
                )
                .orientation(plotly::common::Orientation::Horizontal)
                .x(0.5)
                .x_anchor(plotly::common::Anchor::Center)
                .y(1.02)
                .y_anchor(plotly::common::Anchor::Bottom),
        )
}

#[allow(dead_code)]
/// Creates a styled X axis
fn create_x_axis(title: &str) -> Axis {
    Axis::new()
        .title(
            Title::from(title).font(
                Font::new()
                    .family(FONT_FAMILY)
                    .size(AXIS_TITLE_FONT_SIZE)
                    .color(TEXT_COLOR),
            ),
        )
        .grid_color(GRID_COLOR)
        .line_color(AXIS_LINE_COLOR)
        .zero_line(false)
        .show_spikes(false)
        .tick_font(
            Font::new()
                .family(FONT_FAMILY)
                .size(TICK_FONT_SIZE)
                .color(TEXT_COLOR),
        )
        .show_grid(true)
}

#[allow(dead_code)]
/// Creates a styled Y axis
fn create_y_axis(title: &str) -> Axis {
    Axis::new()
        .title(
            Title::from(title).font(
                Font::new()
                    .family(FONT_FAMILY)
                    .size(AXIS_TITLE_FONT_SIZE)
                    .color(TEXT_COLOR),
            ),
        )
        .grid_color(GRID_COLOR)
        .line_color(AXIS_LINE_COLOR)
        .zero_line(false)
        .tick_font(
            Font::new()
                .family(FONT_FAMILY)
                .size(TICK_FONT_SIZE)
                .color(TEXT_COLOR),
        )
        .show_grid(true)
}

#[allow(dead_code)]
/// Creates a base configuration for all plots
fn create_base_configuration() -> Configuration {
    Configuration::new()
        .responsive(true)
        .display_mode_bar(DisplayModeBar::False)
        .display_logo(false)
        .scroll_zoom(false)
}

/// Renders a plot using plotly's wasm bindings
#[cfg(target_arch = "wasm32")]
fn render_plot(plot_id: &str, plot: &Plot) {
    let id = plot_id.to_string();
    let plot = plot.clone();

    spawn(async move {
        plotly::bindings::react(&id, &plot).await;
    });
}

#[allow(dead_code)]
fn create_ttk_cdf(
    distributions: Vec<Vec<f64>>,
    time_unit: TimeUnit,
    labels: Option<Vec<String>>,
) -> Plot {
    let mut plot = Plot::new();

    for (i, dist) in distributions.iter().enumerate() {
        let mut cumulative_sum = 0.0;
        let cdf_data: Vec<(f64, f64)> = dist
            .iter()
            .enumerate()
            .map(|(x, prob)| {
                cumulative_sum += prob;
                (x as f64, cumulative_sum)
            })
            .collect();

        let mut ttks: Vec<f64> = cdf_data.iter().map(|(x, _)| *x).collect();
        let cum_probs: Vec<f64> = cdf_data.iter().map(|(_, y)| *y).collect();

        if time_unit == TimeUnit::Seconds {
            ttks = ttks.iter().map(|ttk| ttk * 0.6).collect();
        }

        let name = labels
            .as_ref()
            .and_then(|l| l.get(i))
            .map(|s| s.as_str())
            .unwrap_or("");

        let color = TRACE_COLORS[i % TRACE_COLORS.len()];
        let trace = Scatter::new(ttks, cum_probs)
            .mode(Mode::Lines)
            .name(name)
            .line(Line::new().color(color).width(2.5));
        plot.add_trace(trace);
    }

    let x_axis_label = match time_unit {
        TimeUnit::Seconds => "Time to Kill (seconds)",
        TimeUnit::Ticks => "Time to Kill (ticks)",
    };

    let layout = create_base_layout()
        .x_axis(create_x_axis(x_axis_label))
        .y_axis(create_y_axis("Cumulative Probability"));

    plot.set_layout(layout);
    plot.set_configuration(create_base_configuration());
    plot
}

#[component]
pub fn TtkCdf(
    distributions: Vec<Vec<f64>>,
    time_unit: Signal<TimeUnit>,
    labels: Option<Vec<String>>,
) -> Element {
    let plot_id = use_signal(|| format!("ttk-cdf-{}", rand_id()));

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let id = plot_id();
        let distributions = distributions.clone();
        let labels = labels.clone();

        let plot = create_ttk_cdf(distributions, time_unit(), labels);
        render_plot(&id, &plot);
    });

    rsx! {
        div { class: "plot-container w-full",
            div { id: plot_id(), class: "w-full aspect-4/3 min-h-64 max-h-96" }
        }
    }
}

#[allow(dead_code)]
fn create_ttk_histogram(
    distributions: Vec<Vec<f64>>,
    time_unit: TimeUnit,
    labels: Option<Vec<String>>,
) -> Plot {
    let mut plot = Plot::new();

    for (i, dist) in distributions.iter().enumerate() {
        let (mut ttks, probs): (Vec<f64>, Vec<f64>) = dist
            .iter()
            .enumerate()
            .map(|(t, p)| (t as f64, *p))
            .collect();
        if time_unit == TimeUnit::Seconds {
            ttks = ttks.iter().map(|ttk| ttk * 0.6).collect();
        }

        let color = TRACE_COLORS[i % TRACE_COLORS.len()];
        let hist = Bar::new(ttks, probs)
            .name(
                labels
                    .as_ref()
                    .map_or("", |l| l.get(i).map(|s| s.as_str()).unwrap_or("")),
            )
            .opacity(0.85)
            .marker(plotly::common::Marker::new().color(color));
        plot.add_trace(hist);
    }

    let x_axis_label = if time_unit == TimeUnit::Seconds {
        "Time to Kill (seconds)"
    } else {
        "Time to Kill (ticks)"
    };

    let layout = create_base_layout()
        .x_axis(create_x_axis(x_axis_label))
        .y_axis(create_y_axis("Probability"))
        .bar_gap(0.1);

    plot.set_layout(layout);
    plot.set_configuration(create_base_configuration());
    plot
}

#[component]
pub fn TtkHistogram(
    distributions: Vec<Vec<f64>>,
    time_unit: TimeUnit,
    labels: Option<Vec<String>>,
) -> Element {
    let plot_id = use_signal(|| format!("ttk-histogram-{}", rand_id()));

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let id = plot_id();
        let distributions = distributions.clone();
        let labels = labels.clone();

        let plot = create_ttk_histogram(distributions, time_unit, labels);
        render_plot(&id, &plot);
    });

    rsx! {
        div { class: "plot-container w-full",
            div { id: plot_id(), class: "w-full aspect-4/3 min-h-64 max-h-96" }
        }
    }
}

#[allow(dead_code)]
fn create_food_histogram(distribution: Vec<f64>) -> Plot {
    let food_counts: Vec<f64> = distribution
        .iter()
        .enumerate()
        .map(|(x, _)| x as f64)
        .collect();
    let probs = distribution;

    let hist = Bar::new(food_counts, probs)
        .marker(plotly::common::Marker::new().color(TRACE_COLORS[1])) // emerald-400
        .opacity(0.85);
    let mut plot = Plot::new();
    plot.add_trace(hist);

    let layout = create_base_layout()
        .x_axis(create_x_axis("Food Eaten"))
        .y_axis(create_y_axis("Probability"))
        .bar_gap(0.15)
        .show_legend(false);

    plot.set_layout(layout);
    plot.set_configuration(create_base_configuration());
    plot
}

#[component]
pub fn FoodHistogram(distribution: Vec<f64>) -> Element {
    let plot_id = use_signal(|| format!("food-histogram-{}", rand_id()));

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let id = plot_id();
        let distribution = distribution.clone();

        let plot = create_food_histogram(distribution);
        render_plot(&id, &plot);
    });

    rsx! {
        div { class: "plot-container w-full",
            div { id: plot_id(), class: "w-full aspect-4/3 min-h-64 max-h-96" }
        }
    }
}
