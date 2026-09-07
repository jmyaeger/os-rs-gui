//! Editor state and the results history. Providers live in `Layout` so the
//! state survives navigation; the history and draft also survive reloads.

use super::metrics::{CombatMetrics, TtkSummary, calculate_against, ttk_distribution};
use super::simulation::{SimOptions, SingleWayInput, SingleWayOutput, ThrallChoice};
use super::spec::{BaseStats, LoadoutSpec};
use super::storage;
use super::strategy::SpecPlan;
use super::target::TargetConfig;
use crate::worker::{CANCELLED, Job, JobOutput};
use dioxus::prelude::*;
use osrs::types::player::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const RESULTS_KEY: &str = "runesim.dps.results.v1";
const DRAFT_KEY: &str = "runesim.dps.draft.v1";
/// Oldest results are dropped beyond this; each carries a TTK distribution.
pub const MAX_RESULTS: usize = 12;

/// Facts about the loadout that need the engine, captured when a result is added.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LoadoutSummary {
    pub boosted: BaseStats,
    pub style: String,
    pub combat_type: String,
    pub speed: i32,
}

impl LoadoutSummary {
    pub fn from_player(player: &Player) -> Self {
        let combat_type = player
            .gear
            .weapon
            .combat_styles
            .get(&player.attrs.active_style)
            .map(|option| option.combat_type.to_string())
            .unwrap_or_default();
        Self {
            boosted: BaseStats::from_current(&player.stats),
            style: player.attrs.active_style.to_string(),
            combat_type,
            speed: player.gear.weapon.speed,
        }
    }
}

/// One entry in the results history: a full setup plus what the engine said about it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultEntry {
    pub id: u32,
    pub name: String,
    pub loadout: LoadoutSpec,
    pub target: TargetConfig,
    pub plan: SpecPlan,
    #[serde(default)]
    pub sim_options: SimOptions,
    #[serde(default)]
    pub summary: LoadoutSummary,
    /// Main-weapon calculator output.
    pub metrics: Option<CombatMetrics>,
    /// Main-weapon analytical TTK distribution.
    pub ttk: Option<TtkSummary>,
    /// Monte Carlo output with special attacks and thralls, once simulated.
    #[serde(default)]
    pub sim: Option<SingleWayOutput>,
    /// Why the metrics or distribution are missing.
    #[serde(default)]
    pub note: Option<String>,
}

impl ResultEntry {
    fn compute(
        id: u32,
        name: String,
        player: &Player,
        target: TargetConfig,
        plan: SpecPlan,
        sim_options: SimOptions,
    ) -> Self {
        let outcome = target
            .combat_monster()
            .ok_or_else(|| {
                "The target is invalid or its starting HP exceeds its maximum".to_string()
            })
            .and_then(|monster| {
                let metrics = calculate_against(
                    player,
                    &monster,
                    sim_options.thrall.map(ThrallChoice::engine),
                )?;
                Ok((metrics, ttk_distribution(player, &monster)))
            });
        let (metrics, ttk, note) = match outcome {
            Ok((metrics, Ok(ttk))) => (Some(metrics), Some(ttk), None),
            Ok((metrics, Err(note))) => (Some(metrics), None, Some(note)),
            Err(note) => (None, None, Some(note)),
        };
        Self {
            id,
            name,
            loadout: LoadoutSpec::from_player(player),
            target,
            plan,
            sim_options,
            summary: LoadoutSummary::from_player(player),
            metrics,
            ttk,
            sim: None,
            note,
        }
    }

    /// Main-weapon expected TTK in seconds.
    pub fn expected_ttk(&self) -> Option<f64> {
        self.ttk
            .as_ref()
            .map(|ttk| ttk.mean)
            .or_else(|| self.metrics.and_then(|metrics| metrics.expected_ttk))
    }

    /// The headline number: simulated when available, else the main-weapon expectation.
    pub fn primary_ttk(&self) -> Option<f64> {
        self.sim
            .as_ref()
            .map(|sim| sim.mean)
            .or_else(|| self.expected_ttk())
    }

    pub fn simulation_input(&self) -> SingleWayInput {
        SingleWayInput {
            loadout: self.loadout.clone(),
            target: self.target.clone(),
            plan: self.plan.clone(),
            options: self.sim_options,
        }
    }
}

/// Transient progress of a simulation for one result. Not persisted.
#[derive(Clone, Debug, PartialEq)]
pub enum SimStatus {
    Queued,
    Running(f64),
    Failed(String),
}

/// Everything the editor holds, in serializable form.
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Draft {
    pub name: String,
    pub loadout: LoadoutSpec,
    pub target: TargetConfig,
    pub plan: SpecPlan,
    pub sim: SimOptions,
    pub loaded_from: Option<u32>,
}

/// The player the editor starts with: the stored draft's loadout, else defaults.
pub fn initial_player() -> Player {
    storage::load::<Draft>(DRAFT_KEY)
        .map(|draft| draft.loadout.to_player().player)
        .unwrap_or_default()
}

pub fn result_letter(index: usize) -> String {
    let letter = (b'A' + (index % 26) as u8) as char;
    if index < 26 {
        letter.to_string()
    } else {
        format!("{letter}{}", index / 26 + 1)
    }
}

#[derive(Clone, Copy)]
pub struct HomeState {
    pub name: Signal<String>,
    pub target: Signal<TargetConfig>,
    pub plan: Signal<SpecPlan>,
    pub sim: Signal<SimOptions>,
    pub results: Signal<Vec<ResultEntry>>,
    /// The result the editor was last loaded from, if any.
    pub loaded_from: Signal<Option<u32>>,
    /// Items a restored setup referenced that the engine no longer knows.
    pub warnings: Signal<Vec<String>>,
    /// Simulations in flight, by result id.
    pub sim_status: Signal<HashMap<u32, SimStatus>>,
    /// Result the comparison deltas are measured against. Falls back to the
    /// first result sharing a monster when unset.
    pub baseline: Signal<Option<u32>>,
}

impl HomeState {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let draft = storage::load::<Draft>(DRAFT_KEY).unwrap_or_default();
        let results: Vec<ResultEntry> = storage::load::<Vec<ResultEntry>>(RESULTS_KEY)
            .unwrap_or_default()
            .into_iter()
            .map(|mut entry| {
                entry.target = entry.target.after_load();
                entry
            })
            .collect();
        Self {
            name: Signal::new(draft.name),
            target: Signal::new(draft.target.after_load()),
            plan: Signal::new(draft.plan),
            sim: Signal::new(draft.sim),
            results: Signal::new(results),
            loaded_from: Signal::new(draft.loaded_from),
            warnings: Signal::new(Vec::new()),
            sim_status: Signal::new(HashMap::new()),
            baseline: Signal::new(None),
        }
    }

    pub fn persist_results(&self) {
        storage::save(RESULTS_KEY, &*self.results.read());
    }

    pub fn persist_draft(&self, player: &Player) {
        storage::save(DRAFT_KEY, &self.draft(player));
    }

    pub fn draft(&self, player: &Player) -> Draft {
        Draft {
            name: self.name.read().clone(),
            loadout: LoadoutSpec::from_player(player),
            target: self.target.read().clone(),
            plan: self.plan.read().clone(),
            sim: *self.sim.read(),
            loaded_from: *self.loaded_from.read(),
        }
    }

    /// The result the editor currently mirrors, and whether it has been edited since.
    pub fn origin(&self, player: &Player) -> Option<(usize, bool)> {
        let id = (*self.loaded_from.read())?;
        let results = self.results.read();
        let index = results.iter().position(|entry| entry.id == id)?;
        let entry = &results[index];
        let unchanged = entry.loadout == LoadoutSpec::from_player(player)
            && entry.target == *self.target.read()
            && entry.plan == *self.plan.read()
            && entry.sim_options == *self.sim.read();
        Some((index, unchanged))
    }

    /// Snapshot the editor into a new result. Returns its id.
    pub fn add_result(&mut self, player: &Player) -> u32 {
        let id = self
            .results
            .peek()
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0)
            + 1;
        let mut name = self.name.peek().trim().to_string();
        if name.is_empty() {
            name = player.gear.weapon.name.clone();
            self.name.set(name.clone());
        }
        let entry = ResultEntry::compute(
            id,
            name,
            player,
            self.target.peek().clone(),
            self.plan.peek().clone(),
            *self.sim.peek(),
        );
        let mut results = self.results.write();
        results.push(entry);
        while results.len() > MAX_RESULTS {
            let dropped = results.remove(0);
            self.sim_status.write().remove(&dropped.id);
        }
        drop(results);
        self.loaded_from.set(Some(id));
        id
    }

    /// Rename a result in place, without loading it into the editor.
    pub fn rename_result(&mut self, id: u32, name: String) {
        if let Some(entry) = self.results.write().iter_mut().find(|entry| entry.id == id) {
            entry.name = name;
        }
    }

    pub fn remove_result(&mut self, id: u32) {
        self.results.write().retain(|entry| entry.id != id);
        self.sim_status.write().remove(&id);
        if *self.baseline.peek() == Some(id) {
            self.baseline.set(None);
        }
        if *self.loaded_from.peek() == Some(id) {
            self.loaded_from.set(None);
        }
    }

    pub fn clear_results(&mut self) {
        crate::worker::cancel_jobs();
        self.results.set(Vec::new());
        self.sim_status.set(HashMap::new());
        self.baseline.set(None);
        self.loaded_from.set(None);
    }

    /// Run the Monte Carlo simulation for one result in the shared worker.
    /// The card shows progress; the entry gains `sim` when it finishes.
    pub fn simulate_entry(&mut self, id: u32) {
        let Some(entry) = self
            .results
            .peek()
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
        else {
            return;
        };
        let input = entry.simulation_input();
        let mut status = self.sim_status;
        let mut results = self.results;
        status.write().insert(id, SimStatus::Queued);
        spawn(async move {
            let outcome = crate::worker::run_job(Job::SingleWay(input), move |value| {
                // Signals are Copy; taking a local copy keeps the callback `Fn`.
                let mut progress = status;
                progress.write().insert(id, SimStatus::Running(value));
            })
            .await;
            match outcome {
                Ok(JobOutput::SingleWay(output)) => {
                    if let Some(entry) = results.write().iter_mut().find(|entry| entry.id == id) {
                        entry.sim = Some(output);
                    }
                    status.write().remove(&id);
                }
                Ok(_) => {
                    status
                        .write()
                        .insert(id, SimStatus::Failed("Unexpected worker response".into()));
                }
                Err(error) if error == CANCELLED => {
                    status.write().remove(&id);
                }
                Err(error) => {
                    status.write().insert(id, SimStatus::Failed(error));
                }
            }
        });
    }

    /// Stop every running simulation. Their cards return to the unsimulated state.
    pub fn cancel_simulations(&mut self) {
        crate::worker::cancel_jobs();
        self.sim_status
            .write()
            .retain(|_, status| matches!(status, SimStatus::Failed(_)));
    }

    /// Load a result back into the editor.
    pub fn restore(&mut self, entry: &ResultEntry, player: &mut Signal<Player>) {
        let restored = entry.loadout.to_player();
        player.set(restored.player);
        self.warnings.set(restored.warnings);
        self.name.set(entry.name.clone());
        self.target.set(entry.target.clone());
        self.plan.set(entry.plan.clone());
        self.sim.set(entry.sim_options);
        self.loaded_from.set(Some(entry.id));
    }

    pub fn reset(&mut self, player: &mut Signal<Player>) {
        player.set(Player::default());
        self.warnings.set(Vec::new());
        self.name.set(String::new());
        self.target.set(TargetConfig::default());
        self.plan.set(SpecPlan::default());
        let trials = self.sim.peek().trials;
        self.sim.set(SimOptions {
            trials,
            thrall: None,
        });
        self.loaded_from.set(None);
    }
}
