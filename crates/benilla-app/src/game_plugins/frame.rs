//! The recorded frame: every schedule [`headless_client`] builds, written out as the build sees
//! it, in `frame.txt` beside this file. Recorded on stock Bevy 0.18.1 before the trimmed Bevy
//! replaced it (A2 of `c-port-plan.md`); a scheduler that runs benilla must produce the same
//! file, the trimmed Bevy's and later the C one's.
//!
//! Per schedule: the systems in the order the build sorts them (what the single-threaded executor
//! runs, and one order the multi-threaded executor may run), with the `ApplyDeferred` sync points
//! the build inserts; the sets each system is directly in and its own run conditions; every set
//! with its parents and conditions; and every declared order, `a -> b` for "a runs before b",
//! between systems or sets. The set bevy gives each system for `.after(system)` is written
//! `type(system)` and left out of the system's own `in` list. A system whose name is not unique in
//! its schedule is named `name@index`, its place in the order. Above the schedules, `MainScheduleOrder` and
//! `FixedMainScheduleOrder` say which schedules a frame runs, in order.
//!
//! What `run()` adds around the headless client is not here: `FpsJournalPlugin`, the dev
//! instruments, the build banner, the single-threaded executor on `Update` and `PostUpdate`.
//! Observers and hooks are not schedules and are not here either.
//!
//! `WOW_FRAME_RECORD=1` rewrites the file; the names need Bevy's `debug` feature (`dev`), so the
//! test skips in the player build.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

use bevy::app::{FixedMainScheduleOrder, MainScheduleOrder};
use bevy::ecs::schedule::graph::Direction;
use bevy::ecs::schedule::{InternedScheduleLabel, NodeId, Schedule, Schedules};
use bevy::prelude::*;

use super::schedule_tests::headless_client;

/// The recording, relative to this crate.
const RECORDED: &str = "src/game_plugins/frame.txt";

fn labels(labels: &[InternedScheduleLabel]) -> String {
    labels
        .iter()
        .map(|l| format!("{l:?}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn list(items: &[String]) -> String {
    items.join(", ")
}

/// The whole frame as text.
fn record(app: &mut App) -> String {
    let world = app.world_mut();
    let mut out = String::new();
    writeln!(
        out,
        "# benilla's frame: game_plugins::frame, recorded from headless_client() on Bevy 0.18.1."
    )
    .unwrap();
    writeln!(
        out,
        "# WOW_FRAME_RECORD=1 cargo test -p benilla-app game_plugins::frame rewrites it."
    )
    .unwrap();
    let main = world.resource::<MainScheduleOrder>();
    writeln!(out, "startup {}", labels(&main.startup_labels)).unwrap();
    writeln!(out, "main {}", labels(&main.labels)).unwrap();
    let fixed = world.resource::<FixedMainScheduleOrder>();
    writeln!(out, "fixed {}", labels(&fixed.labels)).unwrap();
    let mut schedules: Vec<(String, InternedScheduleLabel)> = world
        .resource::<Schedules>()
        .iter()
        .map(|(label, schedule)| (format!("{label:?}"), schedule.label()))
        .collect();
    schedules.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, label) in schedules {
        writeln!(out, "\nschedule {name}").unwrap();
        world.schedule_scope(label, |world, schedule| {
            write_schedule(world, schedule, &mut out)
        });
    }
    out
}

/// One schedule. Pass 1 reads the declared graph, whose systems and conditions move into the
/// executable when the schedule builds; pass 2 builds it and reads the order.
fn write_schedule(world: &mut World, schedule: &mut Schedule, out: &mut String) {
    let graph = schedule.graph();
    let mut conditions: HashMap<NodeId, Vec<String>> = HashMap::new();
    for (key, _, conds) in graph.systems.iter() {
        conditions.insert(
            NodeId::System(key),
            conds
                .iter()
                .map(|c| c.condition.name().to_string())
                .collect(),
        );
    }
    // Every set with its debug name; a `SystemTypeSet` (the set `.after(some_system)` names) also
    // with its members, as its debug name spells out the whole `FunctionSystem` type.
    let mut sets: Vec<(NodeId, String, Option<Vec<NodeId>>)> = Vec::new();
    for (key, set, conds) in graph.system_sets.iter() {
        let node = NodeId::Set(key);
        let members = set.system_type().map(|_| {
            graph
                .hierarchy()
                .graph()
                .neighbors_directed(node, Direction::Outgoing)
                .collect()
        });
        sets.push((node, format!("{set:?}"), members));
        conditions.insert(
            node,
            conds
                .iter()
                .map(|c| c.condition.name().to_string())
                .collect(),
        );
    }
    let parents: HashMap<NodeId, Vec<NodeId>> = graph
        .hierarchy()
        .graph()
        .nodes()
        .map(|node| {
            let up = graph
                .hierarchy()
                .graph()
                .neighbors_directed(node, Direction::Incoming)
                .collect();
            (node, up)
        })
        .collect();
    let edges: Vec<(NodeId, NodeId)> = graph.dependency().graph().all_edges().collect();

    schedule.initialize(world).expect("the schedule builds");
    let order: Vec<(NodeId, String)> = schedule
        .systems()
        .expect("initialized")
        .map(|(key, system)| (NodeId::System(key), system.name().to_string()))
        .collect();

    // Display names: a type set is `type(system)`, after the system it stands for, and a system
    // or set whose name repeats is told apart by its place (`@index` in the order for a system,
    // `@n` among the sorted sets).
    let mut names: HashMap<NodeId, String> = HashMap::new();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (_, name) in &order {
        *seen.entry(name).or_default() += 1;
    }
    for (i, (node, name)) in order.iter().enumerate() {
        let shown = if seen[name.as_str()] > 1 {
            format!("{name}@{i}")
        } else {
            name.clone()
        };
        names.insert(*node, shown);
    }
    let system_name: HashMap<NodeId, &str> = order
        .iter()
        .map(|(node, name)| (*node, name.as_str()))
        .collect();
    let is_type_set: HashSet<NodeId> = sets
        .iter()
        .filter(|(_, _, members)| members.is_some())
        .map(|(node, _, _)| *node)
        .collect();
    let mut set_names: Vec<(String, NodeId)> = sets
        .iter()
        .map(|(node, debug, members)| {
            let member = members
                .iter()
                .flatten()
                .find_map(|m| system_name.get(m).copied());
            let shown = match member {
                Some(system) => format!("type({system})"),
                None => debug.clone(),
            };
            (shown, *node)
        })
        .collect();
    set_names.sort_by(|a, b| a.0.cmp(&b.0));
    let mut set_seen: HashMap<String, usize> = HashMap::new();
    for (name, _) in &set_names {
        *set_seen.entry(name.clone()).or_default() += 1;
    }
    let mut set_index: HashMap<String, usize> = HashMap::new();
    for (name, node) in &set_names {
        let shown = if set_seen[name] > 1 {
            let k = set_index.entry(name.clone()).or_default();
            *k += 1;
            format!("{name}@{k}")
        } else {
            name.clone()
        };
        names.insert(*node, shown);
    }
    let name = |node: &NodeId| names.get(node).cloned().unwrap_or_else(|| "?".into());
    // A system's own type set is implied (bevy puts every system in it, and nothing else can
    // join one), so an `in` list leaves type sets out.
    let sorted = |nodes: Option<&Vec<NodeId>>| -> Vec<String> {
        let mut v: Vec<String> = nodes
            .into_iter()
            .flatten()
            .filter(|n| !is_type_set.contains(n))
            .map(name)
            .collect();
        v.sort();
        v
    };
    let conds = |node: &NodeId| conditions.get(node).cloned().unwrap_or_default();

    for (i, (node, _)) in order.iter().enumerate() {
        writeln!(out, "  {i} {}", name(node)).unwrap();
        let sets = sorted(parents.get(node));
        if !sets.is_empty() {
            writeln!(out, "    in {}", list(&sets)).unwrap();
        }
        let c = conds(node);
        if !c.is_empty() {
            writeln!(out, "    if {}", list(&c)).unwrap();
        }
    }
    // A type set with nothing of its own is only its system's name, and is not listed.
    for (_, node) in &set_names {
        let sets = sorted(parents.get(node));
        let c = conds(node);
        if is_type_set.contains(node) && sets.is_empty() && c.is_empty() {
            continue;
        }
        writeln!(out, "  set {}", name(node)).unwrap();
        if !sets.is_empty() {
            writeln!(out, "    in {}", list(&sets)).unwrap();
        }
        if !c.is_empty() {
            writeln!(out, "    if {}", list(&c)).unwrap();
        }
    }
    let edges: BTreeSet<String> = edges
        .iter()
        .map(|(a, b)| format!("  {} -> {}", name(a), name(b)))
        .collect();
    for e in edges {
        writeln!(out, "{e}").unwrap();
    }
}

/// The frame the build produces is the recorded one: the same schedules, the same order, the same
/// sets, conditions and declared orders. On a difference the new recording is written to
/// `$TMPDIR/frame.actual.txt` and the first differing line is named; a change to benilla's own
/// systems re-records with `WOW_FRAME_RECORD=1`, a change to the scheduler never does.
#[test]
fn the_frame_is_the_recorded_one() {
    let mut app = headless_client();
    let actual = record(&mut app);
    if !actual.contains("benilla_app::") {
        eprintln!("no type names in this build (Bevy's `debug` feature is off); skipped");
        return;
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(RECORDED);
    if std::env::var_os("WOW_FRAME_RECORD").is_some() {
        std::fs::write(&path, &actual).expect("write the recording");
        eprintln!("recorded {}", path.display());
        return;
    }
    let recorded = std::fs::read_to_string(&path).expect("the recording exists");
    if actual != recorded {
        let dump = std::env::temp_dir().join("frame.actual.txt");
        std::fs::write(&dump, &actual).expect("write the actual frame");
        let line = actual
            .lines()
            .zip(recorded.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| actual.lines().count().min(recorded.lines().count()));
        panic!(
            "the frame differs from {RECORDED} from line {}: recorded {:?}, built {:?}; the whole \
             build is in {}",
            line + 1,
            recorded.lines().nth(line),
            actual.lines().nth(line),
            dump.display()
        );
    }
}
