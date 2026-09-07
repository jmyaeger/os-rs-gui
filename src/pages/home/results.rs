//! The results history: one card per added setup, and the TTK chart beneath.

use super::examples::examples;
use super::metrics::format_seconds;
use super::spec::GearItem;
use super::state::{HomeState, MAX_RESULTS, ResultEntry, SimStatus, result_letter};
use crate::components::plots::{TRACE_COLORS, TimeUnit, TtkCdf, TtkHistogram};
use crate::{BONUSES_ASSETS, PLACEHOLDERS_ASSETS, POTIONS_ASSETS, PRAYERS_ASSETS};
use dioxus::prelude::*;
use osrs::types::equipment::GearSlot;
use osrs::types::player::Player;

/// Paperdoll positions: (slot, row, column) in a 3×5 grid.
const GEAR_GRID: [(GearSlot, u8, u8); 11] = [
    (GearSlot::Head, 1, 2),
    (GearSlot::Cape, 2, 1),
    (GearSlot::Neck, 2, 2),
    (GearSlot::Ammo, 2, 3),
    (GearSlot::Weapon, 3, 1),
    (GearSlot::Body, 3, 2),
    (GearSlot::Shield, 3, 3),
    (GearSlot::Legs, 4, 2),
    (GearSlot::Hands, 5, 1),
    (GearSlot::Feet, 5, 2),
    (GearSlot::Ring, 5, 3),
];

pub fn result_color(index: usize) -> &'static str {
    TRACE_COLORS[index % TRACE_COLORS.len()]
}

#[derive(Clone, Copy, PartialEq)]
enum ChartMode {
    Distribution,
    Cumulative,
}

/// The entry deltas are measured against: the chosen baseline, else the first
/// result sharing this one's monster.
fn baseline_index(entries: &[ResultEntry], index: usize, chosen: Option<u32>) -> Option<usize> {
    if let Some(id) = chosen
        && let Some(position) = entries.iter().position(|entry| entry.id == id)
        && entries[position].primary_ttk().is_some()
    {
        return Some(position);
    }
    let entry = &entries[index];
    entries.iter().position(|other| {
        other.target.label() == entry.target.label() && other.primary_ttk().is_some()
    })
}

fn delta_label(entries: &[ResultEntry], index: usize, chosen: Option<u32>) -> Option<String> {
    let mean = entries[index].primary_ttk()?;
    let baseline = baseline_index(entries, index, chosen)?;
    if baseline == index {
        // Only meaningful when another result actually compares against this one.
        let peers = entries.iter().enumerate().any(|(other, _)| {
            other != index && baseline_index(entries, other, chosen) == Some(index)
        });
        return peers.then(|| "Baseline".to_string());
    }
    let base = entries[baseline].primary_ttk()?;
    let change = (mean / base - 1.0) * 100.0;
    Some(format!("{change:+.1}% vs {}", result_letter(baseline)))
}

#[component]
pub fn ResultsPanel() -> Element {
    let mut state = use_context::<HomeState>();
    let mut player = use_context::<Signal<Player>>();
    let mut compact = use_signal(|| false);
    let mut mode = use_signal(|| ChartMode::Distribution);
    let mut unit = use_signal(|| TimeUnit::Seconds);
    let entries = state.results.read().clone();
    let statuses = state.sim_status.read().clone();
    let loaded = *state.loaded_from.read();
    let chosen_baseline = *state.baseline.read();
    let any_simulated = entries.iter().any(|entry| entry.sim.is_some());
    let plotted: Vec<(usize, &ResultEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.sim.is_some() || entry.ttk.is_some())
        .collect();
    let distributions: Vec<Vec<f64>> = plotted
        .iter()
        .filter_map(|(_, entry)| {
            entry
                .sim
                .as_ref()
                .map(|sim| sim.ticks.clone())
                .or_else(|| entry.ttk.as_ref().map(|ttk| ttk.ticks.clone()))
        })
        .collect();
    let labels: Vec<String> = plotted
        .iter()
        .map(|(index, entry)| {
            let mut label = format!("{} · {}", result_letter(*index), entry.name);
            if any_simulated && entry.sim.is_none() {
                label.push_str(" (main weapon only)");
            }
            label
        })
        .collect();
    let colors: Vec<String> = plotted
        .iter()
        .map(|(index, _)| result_color(*index).to_string())
        .collect();
    let running = statuses
        .values()
        .any(|status| !matches!(status, SimStatus::Failed(_)));

    rsx! {
        section { class: "results-panel", aria_labelledby: "results-heading",
            header { class: "results-header",
                div { class: "results-heading-line",
                    h2 { id: "results-heading", class: "card-title", "Results" }
                    span { class: "results-count num", "{entries.len()}/{MAX_RESULTS}" }
                }
                div { class: "results-toolbar",
                    if running {
                        button {
                            class: "home-text-button",
                            onclick: move |_| state.cancel_simulations(),
                            "Cancel simulations"
                        }
                    }
                    if !entries.is_empty() {
                        div {
                            class: "home-segmented",
                            role: "group",
                            aria_label: "Card detail",
                            button {
                                class: if !compact() { "is-active" } else { "" },
                                aria_pressed: !compact(),
                                onclick: move |_| compact.set(false),
                                "Detailed"
                            }
                            button {
                                class: if compact() { "is-active" } else { "" },
                                aria_pressed: compact(),
                                onclick: move |_| compact.set(true),
                                "Compact"
                            }
                        }
                        button {
                            class: "home-text-button",
                            onclick: move |_| state.clear_results(),
                            "Clear all"
                        }
                    }
                }
            }
            if entries.is_empty() {
                div { class: "card results-empty",
                    p {
                        "No results yet. Configure a loadout and a target above, then add it to compare."
                    }
                    button {
                        class: "home-button",
                        onclick: move |_| {
                            let mut ids = Vec::new();
                            for example in examples() {
                                let restored = example.loadout.to_player();
                                state.name.set(example.name.to_string());
                                state.target.set(example.target);
                                state.plan.set(example.plan);
                                ids.push(state.add_result(&restored.player));
                            }
                            let last = state.results.peek().last().cloned();
                            if let Some(last) = last {
                                state.restore(&last, &mut player);
                            }
                            for id in ids {
                                state.simulate_entry(id);
                            }
                        },
                        "Load example results"
                    }
                }
            } else {
                div { class: "results-grid",
                    for (index , entry) in entries.iter().enumerate() {
                        ResultCard {
                            key: "{entry.id}",
                            entry: entry.clone(),
                            index,
                            delta: delta_label(&entries, index, chosen_baseline),
                            status: statuses.get(&entry.id).cloned(),
                            selected: loaded == Some(entry.id),
                            is_baseline: chosen_baseline == Some(entry.id),
                            compact: compact(),
                            on_load: {
                                let entry = entry.clone();
                                move |_| state.restore(&entry, &mut player)
                            },
                            on_remove: {
                                let id = entry.id;
                                move |_| state.remove_result(id)
                            },
                            on_simulate: {
                                let id = entry.id;
                                move |_| state.simulate_entry(id)
                            },
                            on_cancel: move |_| state.cancel_simulations(),
                            on_rename: {
                                let id = entry.id;
                                move |name| state.rename_result(id, name)
                            },
                            on_baseline: {
                                let id = entry.id;
                                move |_| {
                                    let mut baseline = state.baseline;
                                    let next = if *baseline.peek() == Some(id) { None } else { Some(id) };
                                    baseline.set(next);
                                }
                            },
                        }
                    }
                }
                div { class: "card results-chart",
                    div { class: "results-chart-header",
                        h3 { class: "card-title", "Time to kill distribution" }
                        div { class: "results-chart-controls",
                            div {
                                class: "home-segmented",
                                role: "group",
                                aria_label: "Chart type",
                                button {
                                    class: if mode() == ChartMode::Distribution { "is-active" } else { "" },
                                    onclick: move |_| mode.set(ChartMode::Distribution),
                                    "Distribution"
                                }
                                button {
                                    class: if mode() == ChartMode::Cumulative { "is-active" } else { "" },
                                    onclick: move |_| mode.set(ChartMode::Cumulative),
                                    "Cumulative"
                                }
                            }
                            div {
                                class: "home-segmented",
                                role: "group",
                                aria_label: "Time unit",
                                button {
                                    class: if unit() == TimeUnit::Seconds { "is-active" } else { "" },
                                    onclick: move |_| unit.set(TimeUnit::Seconds),
                                    "Seconds"
                                }
                                button {
                                    class: if unit() == TimeUnit::Ticks { "is-active" } else { "" },
                                    onclick: move |_| unit.set(TimeUnit::Ticks),
                                    "Ticks"
                                }
                            }
                        }
                    }
                    if distributions.is_empty() {
                        p { class: "home-muted results-chart-empty",
                            "None of the results has a distribution yet."
                        }
                    } else {
                        p { class: "home-muted",
                            if any_simulated {
                                "Simulated results include special attacks. "
                            }
                            "Click a legend entry to hide or show it."
                        }
                        match mode() {
                            ChartMode::Distribution => rsx! {
                                TtkHistogram {
                                    distributions,
                                    time_unit: unit(),
                                    labels: Some(labels),
                                    colors: Some(colors),
                                }
                            },
                            ChartMode::Cumulative => rsx! {
                                TtkCdf {
                                    distributions,
                                    time_unit: unit,
                                    labels: Some(labels),
                                    colors: Some(colors),
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn ResultCard(
    entry: ResultEntry,
    index: usize,
    delta: Option<String>,
    status: Option<SimStatus>,
    selected: bool,
    is_baseline: bool,
    compact: bool,
    on_load: EventHandler<()>,
    on_remove: EventHandler<()>,
    on_simulate: EventHandler<()>,
    on_cancel: EventHandler<()>,
    on_rename: EventHandler<String>,
    on_baseline: EventHandler<()>,
) -> Element {
    let letter = result_letter(index);
    let color = result_color(index);
    let stats = entry.loadout.stats.rows(&entry.summary.boosted);
    let overrides = entry.target.overrides();
    let mut conditions = entry.loadout.conditions.active_labels();
    if let Some(thrall) = entry.sim_options.thrall {
        conditions.push(format!("{} thrall", thrall.label()));
    }
    let has_specs = !entry.plan.steps.is_empty();
    let spell = entry.loadout.spell.clone();
    let sim = entry.sim.clone();
    let busy = matches!(status, Some(SimStatus::Queued | SimStatus::Running(_)));
    rsx! {
        article {
            class: if selected { "card result-card is-selected" } else { "card result-card" },
            style: "--result-color: {color}",
            aria_label: "Result {letter}",
            header { class: "result-card-header",
                span { class: "result-badge num", "{letter}" }
                div { class: "result-title",
                    input {
                        class: "result-name-input",
                        aria_label: "Rename result {letter}",
                        value: "{entry.name}",
                        maxlength: "80",
                        oninput: move |event| on_rename.call(event.value()),
                    }
                    div { class: "result-target",
                        span { "{entry.target.label()}" }
                        for detail in overrides {
                            span { class: "home-chip is-note", "{detail}" }
                        }
                    }
                }
                div { class: "result-actions",
                    button {
                        class: "home-text-button",
                        title: "Load this setup into the editor",
                        aria_label: "Load result {letter} into the editor",
                        onclick: move |_| on_load.call(()),
                        if selected {
                            "In editor"
                        } else {
                            "Load"
                        }
                    }
                    button {
                        class: "home-icon-button",
                        title: "Remove",
                        aria_label: "Remove result {letter}",
                        onclick: move |_| on_remove.call(()),
                        "×"
                    }
                }
            }
            div { class: "result-headline",
                div { class: "result-number is-primary",
                    span {
                        if sim.is_some() {
                            "Simulated TTK"
                        } else {
                            "Expected TTK"
                        }
                    }
                    strong { class: "num",
                        match entry.primary_ttk() {
                            Some(mean) => rsx! {
                                "{format_seconds(mean)}"
                                small { "s" }
                            },
                            None => rsx! { "—" },
                        }
                    }
                }
                div { class: "result-compare",
                    if let Some(delta) = delta {
                        span { class: "result-delta", "{delta}" }
                    }
                    button {
                        class: if is_baseline { "home-text-button is-on" } else { "home-text-button" },
                        aria_pressed: is_baseline,
                        title: "Compare the other results against this one",
                        onclick: move |_| on_baseline.call(()),
                        if is_baseline {
                            "Baseline ✓"
                        } else {
                            "Set baseline"
                        }
                    }
                }
            }
            if let Some(note) = &entry.note {
                p { class: "home-error", "{note}" }
            }
            div { class: "result-sim",
                match status {
                    Some(SimStatus::Queued) => rsx! {
                        span { "Waiting for the simulation worker…" }
                        button { class: "home-text-button", onclick: move |_| on_cancel.call(()), "Cancel" }
                    },
                    Some(SimStatus::Running(progress)) => rsx! {
                        span { "Simulating" }
                        div {
                            class: "result-progress",
                            role: "progressbar",
                            aria_valuenow: "{(progress * 100.0) as u32}",
                            aria_valuemin: "0",
                            aria_valuemax: "100",
                            span { style: "width: {progress * 100.0:.0}%" }
                        }
                        span { class: "num", "{progress * 100.0:.0}%" }
                        button { class: "home-text-button", onclick: move |_| on_cancel.call(()), "Cancel" }
                    },
                    Some(SimStatus::Failed(error)) => rsx! {
                        span { class: "home-error", "{error}" }
                        button { class: "home-text-button", onclick: move |_| on_simulate.call(()), "Retry" }
                    },
                    None => {
                        match &sim {
                            Some(sim) => rsx! {
                                span {
                                    "{sim.trials} trials · "
                                    if has_specs {
                                        "specs applied · "
                                    } else {
                                        "no specs · "
                                    }
                                    "{sim.attacks_per_kill:.1} attacks per kill"
                                }
                                button {
                                    class: "home-text-button",
                                    disabled: busy,
                                    onclick: move |_| on_simulate.call(()),
                                    "Re-simulate"
                                }
                            },
                            None => rsx! {
                                span { class: "home-muted", "Not simulated" }
                                if entry.metrics.is_some() {
                                    button {
                                        class: "home-text-button",
                                        disabled: busy,
                                        onclick: move |_| on_simulate.call(()),
                                        "Simulate"
                                    }
                                }
                            },
                        }
                    }
                }
            }
            if let Some(metrics) = entry.metrics {
                div { class: "result-calculated",
                    span {
                        class: "home-eyebrow",
                        title: "Calculated from the main weapon against the target's starting state",
                        "Metrics · excludes specs, includes thralls and burn"
                    }
                    div { class: "result-calculated-grid",
                        div {
                            span { "DPS" }
                            strong { class: "num", "{metrics.dps:.2}" }
                        }
                        div {
                            span { "Expected hit" }
                            strong { class: "num", "{metrics.expected_hit:.2}" }
                        }
                        div {
                            span { "Max hit" }
                            strong { class: "num", "{metrics.max_hit}" }
                        }
                        div {
                            span { "Accuracy" }
                            strong { class: "num", "{metrics.accuracy * 100.0:.2}%" }
                        }
                    }
                }
            }
            if !compact {
                div { class: "result-body",
                    div { class: "result-gear",
                        div {
                            class: "result-paperdoll",
                            aria_label: "Equipment",
                            for (slot , row , column) in GEAR_GRID {
                                {
                                    let item = entry.loadout.item(slot);
                                    let label = item
                                        .map(|item| format!("{slot}: {}", item.label()))
                                        .unwrap_or_else(|| format!("{slot}: empty"));
                                    let src = item
                                        .map(GearItem::image_src)
                                        .unwrap_or_else(|| format!("{PLACEHOLDERS_ASSETS}/{slot}.png"));
                                    rsx! {
                                        div {
                                            class: "result-slot",
                                            style: "grid-row: {row}; grid-column: {column}",
                                            title: "{label}",
                                            img { src, alt: "{label}", class: if item.is_some() { "" } else { "is-empty" } }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "result-side",
                            div { class: "result-style",
                                span { "{entry.summary.style} · {entry.summary.combat_type}" }
                                span { class: "num", "{entry.summary.speed}t" }
                            }
                            if let Some(spell) = spell {
                                div { class: "result-spell", "{spell}" }
                            }
                            div { class: "result-chips",
                                for prayer in &entry.loadout.prayers {
                                    span { class: "home-chip",
                                        img {
                                            src: "{PRAYERS_ASSETS}/{prayer}.png",
                                            alt: "",
                                        }
                                        "{prayer}"
                                    }
                                }
                                for potion in &entry.loadout.potions {
                                    span { class: "home-chip",
                                        img {
                                            src: "{POTIONS_ASSETS}/{potion.replace(\" (-)\", \"\").replace(\" (+)\", \"\")}.png",
                                            alt: "",
                                        }
                                        "{potion}"
                                    }
                                }
                                if entry.loadout.prayers.is_empty() && entry.loadout.potions.is_empty() {
                                    span { class: "home-muted", "No prayers or potions" }
                                }
                            }
                        }
                    }
                    div { class: "result-stats",
                        for (name , base , current) in stats {
                            div { key: "{name}",
                                img {
                                    src: "{BONUSES_ASSETS}/{name.to_lowercase()}.png",
                                    alt: "",
                                }
                                span { class: "result-stat-name", "{name}" }
                                span { class: "num", "{base}" }
                                if current != base {
                                    span { class: "result-stat-boost num", "{current}" }
                                }
                            }
                        }
                    }
                    p { class: "result-conditions home-muted",
                        if conditions.is_empty() {
                            "No situational boosts"
                        } else {
                            "{conditions.join(\" · \")}"
                        }
                    }
                    if has_specs {
                        div { class: "result-specs",
                            div { class: "result-specs-heading",
                                h4 { "Special attacks" }
                            }
                            for (step_index , step) in entry.plan.steps.iter().enumerate() {
                                div { class: "result-spec",
                                    div { class: "result-spec-name",
                                        span { class: "home-muted num", "{step_index + 1}" }
                                        img {
                                            src: step.weapon.image_src(),
                                            alt: "",
                                        }
                                        strong { "{step.weapon.name}" }
                                        span { class: "home-muted", "{step.limits_summary()}" }
                                    }
                                    div { class: "result-spec-detail",
                                        if let Some(style) = step.style_label() {
                                            "{style} · "
                                        }
                                        "{step.condition_summary()}"
                                        if !step.prayers.is_empty() {
                                            " · {step.prayers.iter().map(ToString::to_string).collect::<Vec<_>>().join(\", \")}"
                                        }
                                        if step.switches.is_empty() {
                                            " · main gear"
                                        }
                                        if step.is_two_handed() {
                                            " · shield removed"
                                        }
                                    }
                                    if !step.switches.is_empty() {
                                        div {
                                            class: "result-spec-switches",
                                            aria_label: "Gear switches for {step.weapon.name}",
                                            for item in &step.switches {
                                                img {
                                                    src: item.image_src(),
                                                    alt: "{item.slot}: {item.label()}",
                                                    title: "{item.slot}: {item.label()}",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "home-muted result-spec-settings",
                                "{entry.plan.settings_summary().join(\" · \")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
