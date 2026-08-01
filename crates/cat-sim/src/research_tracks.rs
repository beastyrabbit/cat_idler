//! Player-facing technology tracks layered over the stable research-node ledger.
//!
//! The raw catalog remains the persistence/effect authority. This module groups its
//! generated family stages into one readable card, exposes finite levels, and owns
//! the repeatable global-modifier contract.

use std::{collections::BTreeMap, sync::OnceLock};

use crate::research_catalog::{ResearchCategory, ResearchNode, research_catalog};

pub const BUILDING_TRACK_IDS: &[&str] = &[
    "den",
    "food_storage",
    "water_bowl",
    "beds",
    "herb_garden",
    "nursery",
    "elder_corner",
    "walls",
    "mouse_farm",
    "shrine",
    "workshop",
    "field",
    "research_hut",
    "school",
    "smithy",
    "barracks",
    "wood_cutter",
    "stone_prep",
    "woodworking",
    "clothier",
    "tannery",
    "smelter",
    "accounting_tent",
    "mill",
    "sawmill",
];

pub const RECIPE_TRACK_IDS: &[&str] = &[
    "hunting",
    "foraging",
    "grain_milling",
    "baking",
    "herbalism",
    "textile_work",
    "leatherworking",
    "carpentry",
    "stonecraft",
    "metallurgy",
    "toolmaking",
    "weaponcraft",
    "armorcraft",
    "food_preservation",
    "brewing",
    "waterworks",
    "trade_goods",
    "animal_husbandry",
    "field_craft",
    "expedition_supplies",
];

pub const GLOBAL_TRACK_IDS: &[&str] = &[
    "logistics",
    "construction",
    "scholarship",
    "governance",
    "welfare",
    "exploration",
    "defense_doctrine",
    "combat_doctrine",
    "storage",
    "agriculture",
    "water_management",
    "craftsmanship",
    "trade",
    "resilience",
];

pub const FINITE_TRACK_LEVELS: u32 = 10;
pub const MAX_RESEARCH_QUEUE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechnologyKind {
    Milestone,
    Building,
    Recipe,
    GlobalModifier,
}

#[derive(Debug)]
pub struct TechnologyTrack {
    pub id: String,
    pub name: String,
    pub kind: TechnologyKind,
    pub category: ResearchCategory,
    pub node_indices: Vec<usize>,
    pub root_prerequisite_indices: Vec<usize>,
}

impl TechnologyTrack {
    #[must_use]
    pub fn is_repeatable(&self) -> bool {
        self.kind == TechnologyKind::GlobalModifier
    }

    #[must_use]
    pub fn finite_level_count(&self) -> u32 {
        match self.kind {
            TechnologyKind::Milestone => 1,
            TechnologyKind::Building | TechnologyKind::Recipe | TechnologyKind::GlobalModifier => {
                FINITE_TRACK_LEVELS
            }
        }
    }

    #[must_use]
    pub fn owned_component_count(&self, owned: &[String]) -> usize {
        self.node_indices
            .iter()
            .filter(|index| {
                owned
                    .iter()
                    .any(|id| id == &research_catalog().nodes()[**index].id)
            })
            .count()
    }

    /// Existing family sizes vary between five and eleven. Present them on the
    /// approved ten-step scale while retaining every component node and effect.
    #[must_use]
    pub fn displayed_finite_level(&self, owned: &[String]) -> u32 {
        let count = self.owned_component_count(owned);
        if count == 0 {
            return 0;
        }
        let total = self.node_indices.len().max(1);
        match self.kind {
            TechnologyKind::Milestone => 1,
            TechnologyKind::Building => 1 + (count * 9).div_ceil(total).min(9) as u32,
            TechnologyKind::Recipe | TechnologyKind::GlobalModifier => {
                (count * 10).div_ceil(total).clamp(1, 10) as u32
            }
        }
    }

    #[must_use]
    pub fn next_component<'a>(&self, owned: &[String]) -> Option<&'a ResearchNode>
    where
        'static: 'a,
    {
        self.node_indices.iter().find_map(|index| {
            let node = &research_catalog().nodes()[*index];
            (!owned.iter().any(|id| id == &node.id)).then_some(node)
        })
    }
}

#[derive(Debug)]
pub struct TechnologyCatalog {
    tracks: Vec<TechnologyTrack>,
    by_id: BTreeMap<String, usize>,
    node_to_track: BTreeMap<String, usize>,
}

impl TechnologyCatalog {
    #[must_use]
    pub fn tracks(&self) -> &[TechnologyTrack] {
        &self.tracks
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&TechnologyTrack> {
        self.by_id.get(id).map(|index| &self.tracks[*index])
    }

    #[must_use]
    pub fn for_node(&self, node_id: &str) -> Option<&TechnologyTrack> {
        self.node_to_track
            .get(node_id)
            .map(|index| &self.tracks[*index])
    }
}

fn family_for_node(id: &str) -> Option<(&'static str, TechnologyKind)> {
    for family in BUILDING_TRACK_IDS {
        if id
            .strip_prefix(family)
            .is_some_and(|tail| tail.starts_with('_'))
        {
            return Some((family, TechnologyKind::Building));
        }
    }
    for family in RECIPE_TRACK_IDS {
        if id
            .strip_prefix(family)
            .is_some_and(|tail| tail.starts_with('_'))
        {
            return Some((family, TechnologyKind::Recipe));
        }
    }
    for family in GLOBAL_TRACK_IDS {
        if id
            .strip_prefix(family)
            .is_some_and(|tail| tail.starts_with('_'))
        {
            return Some((family, TechnologyKind::GlobalModifier));
        }
    }
    None
}

fn track_name(first: &ResearchNode, family: &str) -> String {
    let suffix = first.name.split_whitespace().last().unwrap_or_default();
    first
        .name
        .strip_suffix(suffix)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            family
                .split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    chars.next().map_or_else(String::new, |first| {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    })
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
}

fn build_technology_catalog() -> TechnologyCatalog {
    let nodes = research_catalog().nodes();
    let mut grouped: BTreeMap<String, (TechnologyKind, Vec<usize>)> = BTreeMap::new();
    let mut milestones = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if let Some((family, kind)) = family_for_node(&node.id) {
            grouped
                .entry(family.to_owned())
                .or_insert_with(|| (kind, Vec::new()))
                .1
                .push(index);
        } else {
            milestones.push(index);
        }
    }

    let mut tracks = Vec::with_capacity(grouped.len() + milestones.len());
    for index in milestones {
        let node = &nodes[index];
        let prerequisites = node
            .prerequisites
            .iter()
            .filter_map(|id| nodes.iter().position(|candidate| candidate.id == *id))
            .collect();
        tracks.push(TechnologyTrack {
            id: node.id.clone(),
            name: node.name.clone(),
            kind: TechnologyKind::Milestone,
            category: node.category,
            node_indices: vec![index],
            root_prerequisite_indices: prerequisites,
        });
    }
    for (id, (kind, indices)) in grouped {
        let first = &nodes[indices[0]];
        let prerequisites = first
            .prerequisites
            .iter()
            .filter_map(|node_id| nodes.iter().position(|candidate| candidate.id == *node_id))
            .collect();
        tracks.push(TechnologyTrack {
            name: if id == "research_hut" {
                "Research Hut Improvements".to_owned()
            } else {
                track_name(first, &id)
            },
            // The bootstrap "research_hut" milestone and the physical Research
            // Hut improvement family intentionally share a raw prefix. Give the
            // grouped track a distinct UI/save key while preserving every raw
            // node id beneath it.
            id: if id == "research_hut" {
                "research_hut_building".to_owned()
            } else {
                id
            },
            kind,
            category: first.category,
            node_indices: indices,
            root_prerequisite_indices: prerequisites,
        });
    }
    tracks.sort_by(|left, right| {
        nodes[left.node_indices[0]]
            .layout
            .x
            .cmp(&nodes[right.node_indices[0]].layout.x)
            .then_with(|| left.id.cmp(&right.id))
    });

    let by_id = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| (track.id.clone(), index))
        .collect();
    let node_to_track = tracks
        .iter()
        .enumerate()
        .flat_map(|(track_index, track)| {
            track
                .node_indices
                .iter()
                .map(move |node_index| (nodes[*node_index].id.clone(), track_index))
        })
        .collect();
    TechnologyCatalog {
        tracks,
        by_id,
        node_to_track,
    }
}

static TECHNOLOGY_CATALOG: OnceLock<TechnologyCatalog> = OnceLock::new();

#[must_use]
pub fn technology_catalog() -> &'static TechnologyCatalog {
    TECHNOLOGY_CATALOG.get_or_init(build_technology_catalog)
}

#[must_use]
pub fn repeatable_cost(track_id: &str, next_level: u32) -> Option<f64> {
    let track = technology_catalog().get(track_id)?;
    if !track.is_repeatable() || next_level <= FINITE_TRACK_LEVELS {
        return None;
    }
    let last = track
        .node_indices
        .last()
        .map(|index| research_catalog().nodes()[*index].cost)?;
    let exponent = i32::try_from(next_level.saturating_sub(FINITE_TRACK_LEVELS)).ok()?;
    Some((last * 2_f64.powi(exponent)).min(f64::MAX / 4.0))
}

#[must_use]
pub fn base_research_duration_seconds(cost: f64) -> f64 {
    (cost.max(0.0) * 12.0).max(60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_collapses_generated_families_into_readable_tracks() {
        let catalog = technology_catalog();
        assert_eq!(catalog.tracks().len(), 88);
        assert_eq!(
            catalog
                .tracks()
                .iter()
                .filter(|track| track.kind == TechnologyKind::Building)
                .count(),
            25
        );
        assert_eq!(
            catalog
                .tracks()
                .iter()
                .filter(|track| track.kind == TechnologyKind::Recipe)
                .count(),
            19
        );
        assert_eq!(
            catalog
                .tracks()
                .iter()
                .filter(|track| track.is_repeatable())
                .count(),
            14
        );
        assert_eq!(
            catalog
                .tracks()
                .iter()
                .filter(|track| track.kind == TechnologyKind::Milestone)
                .count(),
            30
        );
        assert_eq!(
            catalog
                .tracks()
                .iter()
                .flat_map(|track| &track.node_indices)
                .count(),
            research_catalog().nodes().len()
        );
    }

    #[test]
    fn infinite_cost_and_time_scale_from_the_finite_track() {
        let eleven = repeatable_cost("scholarship", 11).unwrap();
        let twenty = repeatable_cost("scholarship", 20).unwrap();
        assert!(twenty > eleven * 500.0);
        assert!(base_research_duration_seconds(twenty) > 24.0 * 60.0 * 60.0);
    }
}
