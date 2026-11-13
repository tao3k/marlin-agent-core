// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Generic layout, extent resolution and hit testing.

use super::core::resolve_choice;
use marlin_agent_ui::{UiComponentId, UiElement, UiScrollTarget, UiSize};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
};
use std::cell::RefCell;

/// Geometry emitted by the same traversal that paints a declarative tree.
#[derive(Debug)]
pub struct HitRegion {
    pub id: UiComponentId,
    pub area: Rect,
    scroll_target: Option<HitScrollTarget>,
}

#[derive(Debug)]
enum HitScrollTarget {
    Region,
    Named(UiScrollTarget),
}

impl HitRegion {
    pub(super) fn new(id: &UiComponentId, area: Rect, target: Option<&UiScrollTarget>) -> Self {
        let scroll_target = target.map(|target| {
            if target.as_str() == id.as_str() {
                HitScrollTarget::Region
            } else {
                HitScrollTarget::Named(target.clone())
            }
        });
        Self {
            id: id.clone(),
            area,
            scroll_target,
        }
    }

    /// Return the Scheme-selected destination, which may differ from the region identity.
    pub fn scroll_target(&self) -> Option<&str> {
        match self.scroll_target.as_ref()? {
            HitScrollTarget::Region => Some(self.id.as_str()),
            HitScrollTarget::Named(target) => Some(target.as_str()),
        }
    }
}

/// Space requested by an externally bound terminal component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extent {
    Cells(u16),
    Flex(u16),
}

#[derive(Clone, Copy, Default)]
pub(super) struct Demand {
    pub(super) cells: u32,
    flex: u32,
}

/// Resolve Scheme-declared size choices against host facts and cell metrics.
pub fn resolve_size(
    size: &UiSize,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> Extent {
    match size {
        UiSize::Cells(value) => Extent::Cells(*value),
        UiSize::Flex(value) => Extent::Flex(*value),
        UiSize::Binding(name) => extent(name),
        UiSize::Bounded { value, limit } => match (extent(value), extent(limit)) {
            (Extent::Cells(value), Extent::Cells(limit)) => Extent::Cells(value.min(limit)),
            _ => Extent::Cells(0),
        },
        UiSize::Min(left, right)
        | UiSize::Max(left, right)
        | UiSize::Add(left, right)
        | UiSize::Subtract(left, right) => {
            match (
                resolve_size(left, extent, facts),
                resolve_size(right, extent, facts),
            ) {
                (Extent::Cells(left), Extent::Cells(right)) => Extent::Cells(match size {
                    UiSize::Min(..) => left.min(right),
                    UiSize::Max(..) => left.max(right),
                    UiSize::Add(..) => left.saturating_add(right),
                    UiSize::Subtract(..) => left.saturating_sub(right),
                    _ => unreachable!(),
                }),
                _ => Extent::Cells(0),
            }
        }
        UiSize::Choice { cases, fallback } => {
            let selected = cases
                .iter()
                .find_map(|(fact, size)| facts(fact).then_some(size))
                .unwrap_or(fallback);
            resolve_size(selected, extent, facts)
        }
    }
}

/// Locate a region's enclosing size without giving the host knowledge of its policy.
pub fn region_extent(
    element: &UiElement,
    name: &str,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> Option<Extent> {
    match element {
        UiElement::Sized { size, child } => match child.as_ref() {
            UiElement::Region { id, .. } if id.as_str() == name => {
                Some(resolve_size(size, extent, facts))
            }
            _ => region_extent(child, name, extent, facts),
        },
        UiElement::Region { child, .. } | UiElement::Panel { child, .. } => {
            region_extent(child, name, extent, facts)
        }
        UiElement::Rows(children) | UiElement::Columns(children) => children
            .iter()
            .find_map(|child| region_extent(child, name, extent, facts)),
        UiElement::Choice { .. } => {
            region_extent(resolve_choice(element, facts), name, extent, facts)
        }
        _ => None,
    }
}

/// Resolve `region/<id>` bindings against the same selected tree as layout.
/// A frame-local cache avoids repeatedly traversing the tree; cycles and
/// excessive dependency chains resolve to zero instead of recursing forever.
pub struct RegionExtents<'a> {
    tree: &'a UiElement,
    raw: &'a dyn Fn(&UiComponentId) -> Extent,
    facts: &'a dyn Fn(&UiComponentId) -> bool,
    cache: RefCell<Vec<(UiComponentId, Extent)>>,
    active: RefCell<Vec<UiComponentId>>,
}

impl<'a> RegionExtents<'a> {
    pub fn new(
        tree: &'a UiElement,
        raw: &'a dyn Fn(&UiComponentId) -> Extent,
        facts: &'a dyn Fn(&UiComponentId) -> bool,
    ) -> Self {
        Self {
            tree,
            raw,
            facts,
            cache: RefCell::new(Vec::new()),
            active: RefCell::new(Vec::new()),
        }
    }

    pub fn extent(&self, id: &UiComponentId) -> Extent {
        let Some(name) = id.as_str().strip_prefix("region/") else {
            return (self.raw)(id);
        };
        if let Some((_, value)) = self.cache.borrow().iter().find(|(key, _)| key == id) {
            return *value;
        }
        {
            let mut active = self.active.borrow_mut();
            if active.len() >= 32 || active.iter().any(|key| key == id) {
                return Extent::Cells(0);
            }
            active.push(id.clone());
        }
        let value = region_extent(self.tree, name, &|key| self.extent(key), self.facts)
            .unwrap_or(Extent::Cells(0));
        self.active.borrow_mut().pop();
        self.cache.borrow_mut().push((id.clone(), value));
        value
    }
}

/// Preserve fixed and flexible demand when Scheme nests groups on one axis.
pub(super) fn group_demand(
    element: &UiElement,
    vertical: bool,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
) -> Demand {
    match element {
        UiElement::Sized { size, .. } => {
            let size = resolve_size(size, extent, facts);
            match size {
                Extent::Cells(value) => Demand {
                    cells: value.into(),
                    flex: 0,
                },
                Extent::Flex(value) => Demand {
                    cells: 0,
                    flex: value.into(),
                },
            }
        }
        UiElement::Choice { .. } => {
            group_demand(resolve_choice(element, facts), vertical, extent, facts)
        }
        UiElement::Region { child, .. } => group_demand(child, vertical, extent, facts),
        UiElement::Panel { child, .. } => group_demand(child, vertical, extent, facts),
        UiElement::Rows(children) if vertical => {
            children.iter().fold(Demand::default(), |sum, child| {
                let next = group_demand(child, vertical, extent, facts);
                Demand {
                    cells: sum.cells.saturating_add(next.cells),
                    flex: sum.flex.saturating_add(next.flex),
                }
            })
        }
        UiElement::Columns(children) if !vertical => {
            children.iter().fold(Demand::default(), |sum, child| {
                let next = group_demand(child, vertical, extent, facts);
                Demand {
                    cells: sum.cells.saturating_add(next.cells),
                    flex: sum.flex.saturating_add(next.flex),
                }
            })
        }
        _ if vertical => Demand {
            cells: element.intrinsic_rows_with(facts).min(u16::MAX as usize) as u32,
            flex: 0,
        },
        _ => Demand { cells: 0, flex: 1 },
    }
}

pub(super) fn layout_children(
    children: &[UiElement],
    vertical: bool,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    facts: &dyn Fn(&UiComponentId) -> bool,
    mut visit: impl FnMut(&UiElement, Rect),
) {
    let limit = if vertical { area.height } else { area.width };
    // A single child is common for compositional wrappers; no sizing vector is needed.
    if let [child] = children {
        let demand = group_demand(child, vertical, extent, facts);
        let extra = if demand.flex > 0 {
            u32::from(limit).saturating_sub(demand.cells)
        } else {
            0
        };
        let length = demand.cells.saturating_add(extra).min(u32::from(limit)) as u16;
        let rect = if vertical {
            Rect::new(area.x, area.y, area.width, length)
        } else {
            Rect::new(area.x, area.y, length, area.height)
        };
        visit(child, rect);
        return;
    }
    let sizing: Vec<_> = children
        .iter()
        .map(|child| group_demand(child, vertical, extent, facts))
        .collect();
    let fixed = sizing
        .iter()
        .fold(0u32, |sum, size| sum.saturating_add(size.cells));
    let mut available = u32::from(limit).saturating_sub(fixed);
    let mut weight = sizing
        .iter()
        .fold(0u32, |sum, size| sum.saturating_add(size.flex));
    let mut cursor = 0u16;
    for (child, size) in children.iter().zip(sizing) {
        let extra = if size.flex > 0 && weight > 0 {
            let portion = (u64::from(available) * u64::from(size.flex) / u64::from(weight)) as u32;
            available -= portion;
            weight -= size.flex;
            portion
        } else {
            0
        };
        let length = size
            .cells
            .saturating_add(extra)
            .min(u32::from(limit.saturating_sub(cursor))) as u16;
        let rect = if vertical {
            Rect::new(area.x, area.y.saturating_add(cursor), area.width, length)
        } else {
            Rect::new(area.x.saturating_add(cursor), area.y, length, area.height)
        };
        cursor = cursor.saturating_add(length);
        visit(child, rect);
    }
}

/// Map a declarative region tree to terminal rectangles without painting it.
///
/// `regions` is appended in traversal order; callers clear it for each layout.
pub fn layout_regions(
    element: &UiElement,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    regions: &mut Vec<HitRegion>,
) {
    layout_regions_with_facts(element, area, extent, regions, &|_| false);
}

/// Lay out the selected branches using the same facts as painting.
pub fn layout_regions_with_facts(
    element: &UiElement,
    area: Rect,
    extent: &dyn Fn(&UiComponentId) -> Extent,
    regions: &mut Vec<HitRegion>,
    facts: &dyn Fn(&UiComponentId) -> bool,
) {
    if area.is_empty() {
        return;
    }
    match element {
        UiElement::Region {
            id,
            scroll_target,
            child,
        } => {
            regions.push(HitRegion::new(id, area, scroll_target.as_ref()));
            layout_regions_with_facts(child, area, extent, regions, facts);
        }
        UiElement::Text(_)
        | UiElement::Slot(_)
        | UiElement::Template { .. }
        | UiElement::Input { .. } => {}
        UiElement::Choice { .. } => {
            layout_regions_with_facts(resolve_choice(element, facts), area, extent, regions, facts);
        }
        UiElement::Sized { child, .. } => {
            layout_regions_with_facts(child, area, extent, regions, facts);
        }
        UiElement::Panel { child, .. } => {
            let inner = Block::default().borders(Borders::ALL).inner(area);
            layout_regions_with_facts(child, inner, extent, regions, facts);
        }
        UiElement::Rows(children) | UiElement::Columns(children) => {
            let vertical = matches!(element, UiElement::Rows(_));
            layout_children(children, vertical, area, extent, facts, |child, rect| {
                layout_regions_with_facts(child, rect, extent, regions, facts);
            });
        }
    }
}

/// Find the innermost painted region containing a terminal cell.
pub fn hit_test(regions: &[HitRegion], x: u16, y: u16) -> Option<&HitRegion> {
    regions.iter().rev().find(|region| {
        x >= region.area.x
            && x < region.area.right()
            && y >= region.area.y
            && y < region.area.bottom()
    })
}

/// Find the innermost scrollable ancestor at a terminal cell.
pub fn hit_test_scroll(regions: &[HitRegion], x: u16, y: u16) -> Option<&HitRegion> {
    regions.iter().rev().find(|region| {
        region.scroll_target.is_some()
            && x >= region.area.x
            && x < region.area.right()
            && y >= region.area.y
            && y < region.area.bottom()
    })
}
