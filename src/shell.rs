//! The interactive application shell. Focus, hit resolution, press ownership and card motion
//! come from plx_ui. Only dashboard preferences are writable in this slice; TV actions are
//! typed requests checked by a separate, capability-gated executor.
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;

use plx_machine::machine::{
    Canon, Chrome, Cx, Effects, EntryId, FocusKey, GroupId, Host, InputOwner, LogicalState,
    MachineId, PressArm, PressFrom, PressRead, ScreenArg, ScreenId, Tick,
};
use plx_machine::present::{Present, PresentEvent};
use plx_ui::cards::{CardSource, Grid, GridSpec, RowStyle, SectionFrame, Shelf, Tile, TileLabel};
use plx_ui::form::{Form, FormSection, FormTable, RowKey, RowKind};
use plx_ui::input::{InputMachine, PressEvent};
use plx_ui::label::{HAlign, Label};
use plx_ui::screen::{
    Activate, At, AxisMask, By, Dir, DrawFrame, EdgeRule, ElemKind, FocusTarget, Focusable,
    GroupKind, GroupSpec, Hover, Link, Placed, ScreenEvent, Seat, Step, Stop,
};
use plx_ui::table::Row;
use plx_ui::widgets::{
    Art, LegacyMeasure, RangeControl, RangeLayout, RangeScale, RangeStep, StagedRange, TabPill,
    TextEntry,
};
use plx_ui::{theme, Env, Painter, Rect, View};
use serde::{Deserialize, Serialize};

use crate::artwork_runtime::TransitionReadiness;
use crate::artwork_transition::{Anchor as ArtworkAnchor, PauseReason, PauseReasons};
use crate::drawer::{Drawer, KEYS};
use crate::live::{self, Freshness, Update};
use crate::model::{
    controls_form_with_numeric, Action, Application, Catalogue, Context, Page, Setting,
    SettingsSection,
};

const RANGE_BASE: u32 = 35_000;
const RANGE_FRAME: Rect = Rect::new(838.0, 480.0, 946.0, 184.0);
fn range_rect(index: usize) -> Rect {
    match index {
        0 => RANGE_FRAME,
        1 => Rect::new(838.0, 716.0, 946.0, 64.0),
        _ => Rect::new(838.0, 804.0, 946.0, 64.0),
    }
}
fn range_key(key: u32) -> Option<usize> {
    (RANGE_BASE..RANGE_BASE + 3)
        .contains(&key)
        .then(|| (key - RANGE_BASE) as usize)
}
struct RangeSession {
    label: String,
    state: StagedRange,
    pending: bool,
    return_page: Page,
    description: bool,
}
const MENU_BASE: u32 = 32_000;
const MENU_PANEL: Rect = Rect::new(1360.0, 330.0, 464.0, 598.0);
const MENU_ROWS: Rect = Rect::new(1380.0, 424.0, 424.0, 464.0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    ToggleFavourite,
    Earlier,
    Later,
    Close,
}
impl MenuAction {
    fn key(self) -> u32 {
        match self {
            Self::Open => 0,
            Self::ToggleFavourite => 1,
            Self::Earlier => 2,
            Self::Later => 3,
            Self::Close => 4,
        }
    }
}
struct AppMenu {
    id: String,
    origin: u32,
    index: usize,
}
const TOOLBAR_COUNT: usize = 4;
const TOOLBAR_BASE: u32 = 30_000;
const KEYBOARD_BASE: u32 = 31_000;
const TOOLBAR: Rect = Rect::new(480.0, 248.0, 1344.0, 58.0);
const KEYBOARD: Rect = Rect::new(480.0, 668.0, 1344.0, 290.0);
fn toolbar_rect(index: usize) -> Rect {
    match index {
        0 => Rect::new(480.0, 248.0, 144.0, 58.0),
        1 => Rect::new(648.0, 248.0, 226.0, 58.0),
        2 => Rect::new(898.0, 248.0, 698.0, 58.0),
        _ => Rect::new(1620.0, 248.0, 204.0, 58.0),
    }
}
fn keyboard_rect(index: usize) -> Rect {
    Rect::new(
        480.0 + (index % 8) as f32 * 168.0,
        668.0 + (index / 8) as f32 * 58.0,
        154.0,
        48.0,
    )
}
fn toolbar_key(key: u32) -> Option<usize> {
    (TOOLBAR_BASE..TOOLBAR_BASE + TOOLBAR_COUNT as u32)
        .contains(&key)
        .then(|| (key - TOOLBAR_BASE) as usize)
}
fn keyboard_key(key: u32) -> Option<usize> {
    (KEYBOARD_BASE..KEYBOARD_BASE + 40)
        .contains(&key)
        .then(|| (key - KEYBOARD_BASE) as usize)
}
const HOME_APPS_LINK: u32 = 34_000;
const HOME_INPUTS_LINK: u32 = 34_001;
const APP_GRID: Rect = Rect::new(438.0, 312.0, 1386.0, 652.0);
fn home_link_rect(index: usize) -> Rect {
    Rect::new(1512.0, if index == 0 { 248.0 } else { 650.0 }, 312.0, 48.0)
}
fn home_link_key(key: u32) -> Option<usize> {
    match key {
        HOME_APPS_LINK => Some(0),
        HOME_INPUTS_LINK => Some(1),
        _ => None,
    }
}
const CATEGORY_BASE: u32 = 33_000;
const CATEGORIES: [(Page, &str); 6] = [
    (Page::Picture, "Picture"),
    (Page::Sound, "Sound"),
    (Page::Network, "Network"),
    (Page::Accessibility, "Accessibility"),
    (Page::General, "General"),
    (Page::PanelCare, "Panel care"),
];
const CATEGORY_AREA: Rect = Rect::new(480.0, 326.0, 282.0, 500.0);
const DETAIL_PANEL: Rect = Rect::new(798.0, 326.0, 1026.0, 620.0);
const DETAIL_ROWS: Rect = Rect::new(818.0, 466.0, 986.0, 456.0);
fn category_rect(index: usize) -> Rect {
    Rect::new(480.0, 334.0 + 74.0 * index as f32, 282.0, 62.0)
}
fn category_key(key: u32) -> Option<usize> {
    (CATEGORY_BASE..CATEGORY_BASE + 6)
        .contains(&key)
        .then(|| (key - CATEGORY_BASE) as usize)
}
const RAIL_BASE: u32 = 10_000;
const LG_HOME_KEY: u32 = RAIL_BASE + 4;
const RAIL_COUNT: u32 = 5;
const TABLE_BASE: u32 = 20_000;
const RAIL: Rect = Rect::new(96.0, 238.0, 286.0, 690.0);
const CONTENT: Rect = Rect::new(438.0, 238.0, 1386.0, 690.0);
fn rail_rect(index: usize) -> Rect {
    Rect::new(
        96.0,
        if index == 4 {
            682.0
        } else {
            254.0 + 84.0 * index as f32
        },
        286.0,
        68.0,
    )
}
const CARD_STYLE: RowStyle = RowStyle {
    w: 230.0,
    h: 230.0,
    gap: 48.0,
    margin_x: 96.0,
    radius: 32.0,
    right_reserve: 96.0,
    ..RowStyle::HOME
};
const INPUT_DETAIL: Rect = Rect::new(1326.0, 330.0, 498.0, 598.0);
const INPUT_DETAIL_STYLE: RowStyle = RowStyle {
    w: 372.0,
    h: 230.0,
    gap: 34.0,
    ..CARD_STYLE
};
const INPUT_STYLE: RowStyle = RowStyle {
    w: 310.0,
    h: 190.0,
    gap: 32.0,
    ..CARD_STYLE
};
const HOME_APP_STYLE: RowStyle = RowStyle {
    w: 310.0,
    h: 230.0,
    gap: 34.0,
    margin_x: 480.0,
    focus_scale: 1.05,
    ..CARD_STYLE
};
const HOME_INPUT_STYLE: RowStyle = RowStyle {
    margin_x: 480.0,
    gap: 34.0,
    ..INPUT_STYLE
};
const HOME_FAVOURITES: SectionFrame = SectionFrame {
    y: 330.0,
    clip: Rect::new(438.0, 292.0, 1386.0, 340.0),
};
const HOME_INPUTS: SectionFrame = SectionFrame {
    y: 722.0,
    clip: Rect::new(438.0, 684.0, 1386.0, 276.0),
};
const TABS: [(Page, &str); 4] = [
    (Page::Home, "Home"),
    (Page::Applications, "Apps"),
    (Page::Inputs, "Inputs"),
    (Page::Controls, "Controls"),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub id: String,
    pub label: String,
    pub plugged: bool,
    pub signal: bool,
    pub active: bool, // Legacy EIM device flag; not an authoritative selection indicator.
    #[serde(default)]
    pub selected: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub captured_at: String,
    pub applications: Vec<Application>,
    pub launch_ids: BTreeSet<String>,
    pub inputs: Vec<Input>,
    pub context: Context,
    pub settings: BTreeMap<String, Vec<Setting>>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    #[serde(default)]
    pub initialized: bool,
    pub favourites: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub favourite_order: Vec<String>,
}
impl Preferences {
    /// Explicit ranks first; old records retain catalogue order until the user reorders.
    fn ordered_favourites(&self, catalogue: &Catalogue) -> Vec<String> {
        let mut ids = Vec::new();
        for id in self
            .favourite_order
            .iter()
            .chain(catalogue.applications.iter().map(|a| &a.id))
            .chain(self.favourites.iter())
        {
            if self.favourites.contains(id) && !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }
    fn toggle_favourite(&mut self, id: &str) {
        if self.favourites.remove(id) {
            self.favourite_order.retain(|saved| saved != id);
        } else {
            self.favourites.insert(id.to_owned());
            if !self.favourite_order.is_empty() {
                self.favourite_order.push(id.to_owned());
            }
        }
    }
    fn move_favourite(&mut self, id: &str, earlier: bool, catalogue: &Catalogue) -> bool {
        let mut order = self.ordered_favourites(catalogue);
        let Some(i) = order.iter().position(|saved| saved == id) else {
            return false;
        };
        let next = if earlier {
            i.checked_sub(1)
        } else {
            (i + 1 < order.len()).then_some(i + 1)
        };
        let Some(next) = next else {
            return false;
        };
        order.swap(i, next);
        self.favourite_order = order;
        true
    }
}

pub struct HomeHost;
impl LogicalState for Page {
    fn write(&self, c: &mut Canon) {
        c.u32(*self as u32);
    }
    fn probe(&self, out: &mut String) {
        out.push_str(self.title().unwrap_or("Home"));
    }
}
impl ScreenArg for Page {
    fn chrome(&self) -> Chrome {
        Chrome::TabBar
    }
    fn id(&self) -> ScreenId {
        ScreenId(*self as u32 + 1)
    }
    fn title(&self) -> Option<&str> {
        Some(match self {
            Self::Home => "Home",
            Self::Applications => "Applications",
            Self::Inputs => "Inputs",
            Self::Controls => "Controls",
            Self::Picture => "Picture",
            Self::Sound => "Sound",
            Self::Network => "Network",
            Self::Accessibility => "Accessibility",
            Self::General => "General",
            Self::PanelCare => "Panel care",
            Self::PointerSpeed => "Pointer speed",
            Self::RangeEditor => "Adjust value",
        })
    }
    fn same_instance(&self, other: &Self) -> bool {
        self == other
    }
}
impl Host for HomeHost {
    type Arg = Page;
    type Fx = Action;
    type Msg = ();
    type Elem = u32;
    type Views<'a> = ();
    type Init = ();
    type Memory = ();
}

struct Card {
    key: u32,
    id: String,
    title: CString,
    caption: CString,
    initials: CString,
    texture: (u32, f32, f32),
    loading_icon: bool,
    context_selected: bool,
    compact_labels: bool,
}
impl Card {
    fn has_icon(&self) -> bool {
        input_key(&self.id).is_none()
            && self.texture.0 != 0
            && self.texture.1.is_finite()
            && self.texture.2.is_finite()
            && self.texture.1 > 0.0
            && self.texture.2 > 0.0
    }
}
struct Cards {
    items: Vec<Card>,
}
impl CardSource<HomeHost> for Cards {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn elem(&self, i: usize) -> u32 {
        self.items[i].key
    }
    fn index_of(&self, e: &u32) -> Option<usize> {
        self.items.iter().position(|c| c.key == *e)
    }
    fn art(&self, i: usize) -> Art<'_> {
        let c = &self.items[i];
        if c.has_icon() || c.loading_icon {
            Art::Icon {
                id: c.texture.0,
                width: c.texture.1,
                height: c.texture.2,
                identity: c.key as u64,
            }
        } else {
            // Only missing artwork and source/status cards use the shared matte.
            Art::Texture {
                id: 0,
                width: c.texture.1,
                height: c.texture.2,
                identity: c.key as u64,
            }
        }
    }
    fn label(&self, i: usize) -> TileLabel {
        let c = &self.items[i];
        if input_key(&c.id).is_some() {
            // Every source keeps its status in the tile; a focused long name uses the
            // shared label marquee beneath it, like every other card.
            return TileLabel {
                title: Some(c.title.clone()),
                ..Default::default()
            };
        }
        TileLabel {
            title: Some(c.title.clone()),
            caption: None,
            ..Default::default()
        }
    }
    fn holdable(&self, i: usize) -> bool {
        input_key(&self.items[i].id).is_none()
    }
    fn overlay(
        &self,
        p: Painter,
        i: usize,
        tile: &Tile,
        _measure: &dyn plx_machine::machine::Measure,
    ) {
        let c = &self.items[i];
        if input_key(&c.id).is_some() {
            let inset = tile.rect.w * 0.07;
            let title = cstr(&plx_gfx::text::elide(
                c.title.to_str().unwrap_or(""),
                tile.rect.w - 2.0 * inset,
                theme::size::HEADLINE,
                1,
                false,
            ));
            Label::new(title.as_ptr(), theme::size::HEADLINE, theme::TEXT_PRIMARY)
                .bold()
                .draw(
                    p,
                    Rect::new(
                        tile.rect.x + inset,
                        tile.rect.y + inset,
                        tile.rect.w - 2.0 * inset,
                        tile.rect.h * 0.48,
                    ),
                );
            let (connection, signal) = c
                .caption
                .to_str()
                .unwrap_or("")
                .split_once('\n')
                .unwrap_or(("Unavailable", ""));
            for (text, top) in [(connection, 0.49), (signal, 0.70)] {
                Label::new(
                    cstr(text).as_ptr(),
                    theme::size::CAPTION,
                    theme::TEXT_SECONDARY,
                )
                .draw(
                    p,
                    Rect::new(
                        tile.rect.x + inset,
                        tile.rect.y + tile.rect.h * top,
                        tile.rect.w - 2.0 * inset,
                        tile.rect.h * 0.21,
                    ),
                );
            }
        } else if !c.has_icon() && !c.loading_icon {
            Label::new(
                c.initials.as_ptr(),
                theme::size::DISPLAY,
                theme::TEXT_PRIMARY,
            )
            .bold()
            .h(HAlign::Center)
            .draw(p, tile.rect);
        }
        if c.loading_icon && !c.has_icon() && tile.focused {
            p.rring(
                tile.rect,
                theme::CARD_RING_RAD,
                theme::CARD_SHEEN_W,
                theme::TEXT_SECONDARY,
            );
        }
        if input_key(&c.id).is_none() && !tile.focused {
            let text = cstr(&plx_gfx::text::elide(
                c.title.to_str().unwrap_or(""),
                tile.rect.w,
                28,
                0,
                false,
            ));
            Label::new(text.as_ptr(), 28, theme::TEXT_SECONDARY)
                .h(HAlign::Center)
                .draw(
                    p,
                    Rect::new(
                        tile.rect.x,
                        tile.rect.y + tile.rect.h + if c.compact_labels { 0.0 } else { 14.0 },
                        tile.rect.w,
                        if c.compact_labels { 32.0 } else { 44.0 },
                    ),
                );
        }
        if c.context_selected && !tile.focused {
            p.rring(tile.rect, tile.radius, 2.0, theme::TEXT_SECONDARY);
        }
        if tile.focused && !c.has_icon() {
            p.rring(tile.rect, tile.radius, 3.0, theme::TEXT_PRIMARY);
        }
    }
}

struct Surface {
    page: Page,
    grid: Grid,
    cards: Cards,
    home_favourites: Shelf,
    home_inputs: Shelf,
    input_cards: Cards,
    rail: FormTable<String, Action, Page>,
    table: FormTable<String, Action, Page>,
    table_mode: bool,
    control_page: Option<Page>,
    editing: bool,
    menu: Option<FormTable<MenuAction, MenuAction, Page>>,
}
fn input_key(id: &str) -> Option<u32> {
    match id {
        "HDMI_1" => Some(500),
        "HDMI_2" => Some(501),
        "HDMI_3" => Some(502),
        "HDMI_4" => Some(503),
        _ => None,
    }
}
fn entry(page: Page) -> EntryId {
    EntryId(page as u32 + 1)
}
fn root_tab(page: Page) -> Page {
    match page {
        Page::Picture
        | Page::Sound
        | Page::Network
        | Page::Accessibility
        | Page::General
        | Page::PanelCare
        | Page::PointerSpeed
        | Page::RangeEditor => Page::Controls,
        _ => page,
    }
}
fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap()
}
impl Surface {
    fn grid_area(&self) -> Rect {
        if self.page == Page::Applications {
            Rect::new(
                APP_GRID.x,
                APP_GRID.y,
                if self.menu.is_some() {
                    884.0
                } else {
                    APP_GRID.w
                },
                if self.editing { 320.0 } else { APP_GRID.h },
            )
        } else {
            CONTENT
        }
    }
    fn table_area(&self) -> Rect {
        if self.control_page.is_some() {
            DETAIL_ROWS
        } else {
            CONTENT
        }
    }
    fn selected_category(&self) -> usize {
        CATEGORIES
            .iter()
            .position(|(page, _)| Some(*page) == self.control_page)
            .unwrap_or(0)
    }
    fn keys(&self) -> Vec<u32> {
        if self.page == Page::RangeEditor {
            return (RANGE_BASE..RANGE_BASE + 3).collect();
        }
        if let Some(menu) = &self.menu {
            return (0..menu.table.n_rows().max(0) as usize)
                .filter_map(|i| menu.key_at(i).map(|k| MENU_BASE + k.0))
                .collect();
        }
        if self.table_mode {
            (0..self.table.table.n_rows().max(0) as usize)
                .filter_map(|i| self.table.key_at(i).map(|k| TABLE_BASE + k.0))
                .collect()
        } else {
            self.cards
                .items
                .iter()
                .chain(self.input_cards.items.iter())
                .map(|c| c.key)
                .collect()
        }
    }
    fn key_index(&self, k: u32) -> Option<usize> {
        self.keys().iter().position(|x| *x == k)
    }
}
impl Focusable<HomeHost> for Surface {
    fn groups(&self, _cx: &Cx<'_, HomeHost>, out: &mut Vec<GroupSpec>) {
        if self.page == Page::RangeEditor {
            out.push(GroupSpec {
                id: GroupId(9),
                kind: GroupKind::Column,
                seat: Seat::Remembered,
                reachable: AxisMask::BOTH,
                edge: [EdgeRule::Stop; 4],
                extent: DETAIL_ROWS,
                len: 3,
                elem: ElemKind::Control,
            });
            return;
        }
        if self.menu.is_some() {
            out.push(GroupSpec {
                id: GroupId(5),
                kind: GroupKind::Column,
                seat: Seat::Remembered,
                reachable: AxisMask::BOTH,
                edge: [EdgeRule::Stop; 4],
                extent: MENU_ROWS,
                len: self.keys().len(),
                elem: ElemKind::Control,
            });
            return;
        }
        if self.page == Page::Applications && self.editing {
            out.push(GroupSpec {
                id: GroupId(4),
                kind: GroupKind::Grid {
                    cols: 8,
                    holes: &[],
                },
                seat: Seat::Remembered,
                reachable: AxisMask::BOTH,
                edge: [EdgeRule::Stop; 4],
                extent: KEYBOARD,
                len: 40,
                elem: ElemKind::Control,
            });
            return;
        }
        if self.page == Page::Applications {
            out.push(GroupSpec {
                id: GroupId(3),
                kind: GroupKind::Row { wrap: false },
                seat: Seat::Nearest,
                reachable: AxisMask::BOTH,
                edge: [EdgeRule::Geometric; 4],
                extent: TOOLBAR,
                len: TOOLBAR_COUNT,
                elem: ElemKind::Control,
            });
        }
        out.push(GroupSpec {
            id: GroupId(0),
            kind: GroupKind::Column,
            seat: Seat::Remembered,
            reachable: AxisMask::HORIZONTAL,
            edge: [EdgeRule::Geometric; 4],
            extent: RAIL,
            len: RAIL_COUNT as usize,
            elem: ElemKind::Control,
        });
        if self.page == Page::Home {
            for i in 0..2 {
                out.push(GroupSpec {
                    id: GroupId(7 + i as u32),
                    kind: GroupKind::Row { wrap: false },
                    seat: Seat::Remembered,
                    reachable: AxisMask::BOTH,
                    edge: [EdgeRule::Geometric; 4],
                    extent: home_link_rect(i),
                    len: 1,
                    elem: ElemKind::Control,
                });
            }
            out.push(self.home_favourites.group_spec(
                GroupId(1),
                self.cards.len(),
                HOME_FAVOURITES.clip,
            ));
            out.push(self.home_inputs.group_spec(
                GroupId(2),
                self.input_cards.len(),
                HOME_INPUTS.clip,
            ));
            return;
        }
        if self.control_page.is_some() {
            out.push(GroupSpec {
                id: GroupId(6),
                kind: GroupKind::Column,
                seat: Seat::Remembered,
                reachable: AxisMask::HORIZONTAL,
                edge: [EdgeRule::Stop; 4],
                extent: CATEGORY_AREA,
                len: 6,
                elem: ElemKind::Control,
            });
        }
        out.push(GroupSpec {
            id: GroupId(1),
            kind: if self.table_mode {
                GroupKind::Column
            } else {
                GroupKind::Grid {
                    cols: if self.page == Page::Inputs { 2 } else { 4 },
                    holes: &[],
                }
            },
            seat: Seat::Remembered,
            reachable: AxisMask::BOTH,
            edge: [EdgeRule::Geometric; 4],
            extent: if self.page == Page::Applications {
                self.grid_area()
            } else {
                self.table_area()
            },
            len: self.keys().len(),
            elem: if self.table_mode {
                ElemKind::Control
            } else {
                ElemKind::Card
            },
        });
    }
    fn group_of(&self, key: &u32, _cx: &Cx<'_, HomeHost>) -> Option<GroupId> {
        if self.page == Page::RangeEditor {
            return range_key(*key).map(|_| GroupId(9));
        }
        if self.menu.is_some() {
            return self.key_index(*key).map(|_| GroupId(5));
        }
        if let Some(i) = home_link_key(*key).filter(|_| self.page == Page::Home) {
            return Some(GroupId(7 + i as u32));
        }
        if self.control_page.is_some() && category_key(*key).is_some() {
            return Some(GroupId(6));
        }
        if self.page == Page::Applications && self.editing {
            return keyboard_key(*key).map(|_| GroupId(4));
        }
        if self.page == Page::Applications && toolbar_key(*key).is_some() {
            Some(GroupId(3))
        } else if (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(key) {
            Some(GroupId(0))
        } else if self.input_cards.index_of(key).is_some() {
            Some(GroupId(2))
        } else {
            self.key_index(*key).map(|_| GroupId(1))
        }
    }
    fn neighbour(&self, key: FocusKey<u32>, dir: Dir, _cx: &Cx<'_, HomeHost>) -> Step<u32> {
        if self.page == Page::RangeEditor {
            let next = range_key(key.elem).and_then(|i| match dir {
                Dir::Up => i.checked_sub(1),
                Dir::Down => (i < 2).then_some(i + 1),
                _ => None,
            });
            return next.map_or(Step::Edge, |i| {
                Step::Move(FocusKey {
                    entry: key.entry,
                    elem: RANGE_BASE + i as u32,
                })
            });
        }

        if self.menu.is_some() {
            let keys = self.keys();
            let Some(i) = keys.iter().position(|k| *k == key.elem) else {
                return Step::Edge;
            };
            let next = match dir {
                Dir::Up => i.checked_sub(1),
                Dir::Down => Some(i + 1),
                _ => None,
            };
            return next.and_then(|n| keys.get(n)).map_or(Step::Edge, |k| {
                Step::Move(FocusKey {
                    entry: key.entry,
                    elem: *k,
                })
            });
        }
        if let Some(i) = category_key(key.elem).filter(|_| self.control_page.is_some()) {
            let next = match dir {
                Dir::Up => i.checked_sub(1),
                Dir::Down => Some(i + 1).filter(|n| *n < 6),
                _ => None,
            };
            return next.map_or(Step::Edge, |n| {
                Step::Move(FocusKey {
                    entry: key.entry,
                    elem: CATEGORY_BASE + n as u32,
                })
            });
        }
        if self.page == Page::Applications {
            if let Some(i) = keyboard_key(key.elem).filter(|_| self.editing) {
                let next = match dir {
                    Dir::Up => i.checked_sub(8),
                    Dir::Down => Some(i + 8).filter(|n| *n < 40),
                    Dir::Left => i.checked_sub(1).filter(|_| i % 8 > 0),
                    Dir::Right => (i % 8 < 7).then_some(i + 1),
                };
                return next.map_or(Step::Edge, |n| {
                    Step::Move(FocusKey {
                        entry: key.entry,
                        elem: KEYBOARD_BASE + n as u32,
                    })
                });
            }
            if let Some(i) = toolbar_key(key.elem) {
                let next = match dir {
                    Dir::Left => i.checked_sub(1),
                    Dir::Right => Some(i + 1).filter(|n| *n < TOOLBAR_COUNT),
                    _ => None,
                };
                return next.map_or(Step::Edge, |n| {
                    Step::Move(FocusKey {
                        entry: key.entry,
                        elem: TOOLBAR_BASE + n as u32,
                    })
                });
            }
        }
        if self.page == Page::Home && home_link_key(key.elem).is_some() {
            return Step::Edge;
        }
        if self.page == Page::Home {
            if self.cards.index_of(&key.elem).is_some() {
                return self
                    .home_favourites
                    .neighbour::<HomeHost, _>(&self.cards, key, dir);
            }
            if self.input_cards.index_of(&key.elem).is_some() {
                return self
                    .home_inputs
                    .neighbour::<HomeHost, _>(&self.input_cards, key, dir);
            }
        }
        let rail = (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(&key.elem);
        let keys = if rail {
            (RAIL_BASE..RAIL_BASE + RAIL_COUNT).collect::<Vec<_>>()
        } else {
            self.keys()
        };
        let Some(i) = keys.iter().position(|k| *k == key.elem) else {
            return Step::Edge;
        };
        let cols = if rail || self.table_mode {
            1
        } else if self.page == Page::Inputs {
            2
        } else {
            4
        };
        let next = match dir {
            Dir::Up => i.checked_sub(cols),
            Dir::Down => Some(i + cols),
            Dir::Left if cols > 1 && i % cols > 0 => Some(i - 1),
            Dir::Right if cols > 1 && i % cols + 1 < cols => Some(i + 1),
            _ => None,
        };
        next.and_then(|n| keys.get(n)).map_or(Step::Edge, |k| {
            Step::Move(FocusKey {
                entry: key.entry,
                elem: *k,
            })
        })
    }
    fn place(&self, key: &u32, cx: &Cx<'_, HomeHost>, at: At) -> Option<Placed> {
        if self.page == Page::RangeEditor {
            return range_key(*key).map(|i| {
                let rect = range_rect(i);
                Placed {
                    rect,
                    rest_rect: rect,
                    clip: DETAIL_ROWS,
                    index: Some(i as u32),
                }
            });
        }
        if let Some(menu) = &self.menu {
            let i = menu.index_of_key(RowKey(key.checked_sub(MENU_BASE)?))?;
            let rect = menu.table.row_frame(MENU_ROWS, i as i32)?;
            return Some(Placed {
                rect,
                rest_rect: rect,
                clip: MENU_ROWS,
                index: Some(i as u32),
            });
        }
        if let Some(i) = category_key(*key).filter(|_| self.control_page.is_some()) {
            let rect = category_rect(i);
            return Some(Placed {
                rect,
                rest_rect: rect,
                clip: CATEGORY_AREA,
                index: Some(i as u32),
            });
        }
        if self.page == Page::Applications {
            let special = if self.editing {
                keyboard_key(*key).map(|i| (keyboard_rect(i), i, KEYBOARD))
            } else {
                toolbar_key(*key).map(|i| (toolbar_rect(i), i, TOOLBAR))
            };
            if let Some((rect, index, clip)) = special {
                return Some(Placed {
                    rect,
                    rest_rect: rect,
                    clip,
                    index: Some(index as u32),
                });
            }
        }
        if let Some(i) = home_link_key(*key).filter(|_| self.page == Page::Home) {
            let r = home_link_rect(i);
            return Some(Placed {
                rect: r,
                rest_rect: r,
                clip: r,
                index: Some(0),
            });
        }
        if (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(key) {
            let i = (*key - RAIL_BASE) as usize;
            let rect = rail_rect(i);
            Some(Placed {
                rect,
                rest_rect: rect,
                clip: RAIL,
                index: Some(i as u32),
            })
        } else if self.table_mode {
            let i = self
                .table
                .index_of_key(RowKey(key.checked_sub(TABLE_BASE)?))?;
            let rect = self.table.table.row_frame(self.table_area(), i as i32)?;
            Some(Placed {
                rect,
                rest_rect: rect,
                clip: self.table_area(),
                index: Some(i as u32),
            })
        } else if self.page == Page::Home {
            self.home_favourites
                .place(cx, &self.cards, key, HOME_FAVOURITES, at)
                .or_else(|| {
                    self.home_inputs
                        .place(cx, &self.input_cards, key, HOME_INPUTS, at)
                })
        } else {
            self.grid.place(cx, &self.cards, key, at).map(|mut p| {
                if self.page == Page::Applications {
                    p.clip = self.grid_area();
                }
                p
            })
        }
    }
    fn reconcile(&self, want: FocusKey<u32>, _cx: &Cx<'_, HomeHost>) -> FocusKey<u32> {
        if self.page == Page::Applications && self.editing && keyboard_key(want.elem).is_none() {
            return FocusKey {
                entry: entry(self.page),
                elem: KEYBOARD_BASE,
            };
        }
        if self.group_of(&want.elem, _cx).is_some() {
            want
        } else {
            FocusKey {
                entry: entry(self.page),
                elem: self
                    .keys()
                    .first()
                    .copied()
                    .unwrap_or(if self.control_page.is_some() {
                        CATEGORY_BASE + self.selected_category() as u32
                    } else if self.page == Page::Applications {
                        TOOLBAR_BASE
                    } else {
                        RAIL_BASE
                    }),
            }
        }
    }
    fn seat(&self, group: GroupId, from: Placed, _cx: &Cx<'_, HomeHost>) -> FocusKey<u32> {
        if self.page == Page::RangeEditor {
            return FocusKey {
                entry: entry(self.page),
                elem: RANGE_BASE,
            };
        }

        let key = if self.page == Page::Home && group == GroupId(7) {
            HOME_APPS_LINK
        } else if self.page == Page::Home && group == GroupId(8) {
            HOME_INPUTS_LINK
        } else if group == GroupId(6) {
            CATEGORY_BASE + self.selected_category() as u32
        } else if group == GroupId(5) {
            MENU_BASE + MenuAction::ToggleFavourite.key()
        } else if group == GroupId(4) {
            KEYBOARD_BASE
        } else if group == GroupId(3) {
            (0..TOOLBAR_COUNT)
                .min_by(|a, b| {
                    (toolbar_rect(*a).cx() - from.rect.cx())
                        .abs()
                        .total_cmp(&(toolbar_rect(*b).cx() - from.rect.cx()).abs())
                })
                .map(|i| TOOLBAR_BASE + i as u32)
                .unwrap_or(TOOLBAR_BASE)
        } else if group == GroupId(0) {
            RAIL_BASE
                + TABS
                    .iter()
                    .position(|(p, _)| *p == root_tab(self.page))
                    .unwrap_or(0) as u32
        } else if self.page == Page::Home {
            let seat = if group == GroupId(2) {
                self.home_inputs
                    .seat::<HomeHost, _>(&self.input_cards, from)
            } else {
                self.home_favourites.seat::<HomeHost, _>(&self.cards, from)
            };
            seat.map(|k| k.elem).unwrap_or(RAIL_BASE)
        } else {
            let keys = self.keys();
            let mut best = keys.first().copied().unwrap_or(RAIL_BASE);
            let mut dist = f32::MAX;
            for k in keys {
                if let Some(p) = self.place(&k, _cx, At::SpringTarget) {
                    let d = (p.rect.cx() - from.rect.cx()).powi(2)
                        + (p.rect.cy() - from.rect.cy()).powi(2);
                    if d < dist {
                        dist = d;
                        best = k;
                    }
                }
            }
            best
        };
        FocusKey {
            entry: entry(self.page),
            elem: key,
        }
    }
}

pub struct Shell {
    snapshot: Snapshot,
    catalogue: Catalogue,
    prefs: Preferences,
    input: InputMachine<u32>,
    surface: Surface,
    // Borrowed drawing references only; packaged_icons owns their GL handles.
    textures: BTreeMap<String, (u32, f32, f32)>,
    packaged_bindings: crate::packaged_icons::Bindings,
    current_icon_cache: bool,
    // Independent ordinary association/freshness; never Root Runtime readiness.
    current_icon_sources: Option<crate::app_artwork::SourceMap>,
    current_icon_sources_observed: Option<u32>,
    packaged_icons: Option<crate::packaged_icons::PackagedIcons<crate::artwork_gl::Backend>>,
    packaged_gl: crate::artwork_gl::Diagnostics,
    live_textures: crate::artwork_textures::ArtworkTextures<crate::artwork_gl::Backend>,
    artwork_gl: crate::artwork_gl::Diagnostics,
    artwork: Option<crate::artwork_runtime::Runtime>,
    artwork_ever_enabled: bool,
    artwork_pause: PauseReasons,
    artwork_external_held: bool,
    artwork_queued_anchor: Option<(crate::actions::Request, ArtworkAnchor)>,
    artwork_filter: Option<BTreeSet<String>>,
    artwork_origin: std::time::Instant,
    artwork_interest: (Vec<String>, Vec<String>),
    artwork_retry_at: u64,
    artwork_upload_failures: u64,
    artwork_catalogue_live: bool,
    now: u32,
    status: CString,
    pending: Vec<Action>,
    exit: bool,
    preferences_changed: bool,
    live_state: Option<Freshness>,
    live_now: u32,
    live_updates: u32,
    app_launches_enabled: bool,
    home_launches_enabled: bool,
    input_launches_enabled: bool,
    drawer: Drawer,
    system_keyboard_available: bool,
    control_category: Page,
    category_labels: [CString; 6],
    control_caption_page: Option<Page>,
    control_caption_context: Option<Context>,
    control_context_current: bool,
    control_breadcrumb: CString,
    menu: Option<AppMenu>,
    clock: crate::clock::View,
    clock_time: CString,
    clock_source: CString,
    brand_texture: u32,
    navigation_textures: [u32; 4],
    drawer_scroll: plx_ui::Spring,
    drawer_scroll_target: f32,
    home_return: Option<(Page, usize)>,
    range: Option<RangeSession>,
    range_request: Option<(i64, i64)>,
    description_sample: Option<(u32, crate::description_level::Level)>,
    description_confirmation: Option<(u32, crate::description_level::Level)>,
    pointer_enabled: bool,
    pointer_pending: bool,
    pointer_expected: Option<crate::pointer_speed::Speed>,
    pointer_confirmation: Option<(u32, crate::pointer_speed::Speed)>,
}
impl Shell {
    pub fn new(snapshot: Snapshot, mut prefs: Preferences) -> Self {
        let artwork_gl = crate::artwork_gl::Diagnostics::default();
        let live_textures = crate::artwork_textures::ArtworkTextures::new(
            crate::artwork_gl::Backend::new(artwork_gl.clone()),
            crate::artwork_textures::Config::default(),
        )
        .expect("Default live texture limits");
        let mut catalogue = Catalogue::default();
        catalogue.refresh(&snapshot.applications, &snapshot.launch_ids);
        // Preserve the package's mappings before live catalogue merges. These
        // are packaged snapshots, not live app-version/source authority.
        let packaged_bindings = catalogue
            .applications
            .iter()
            .filter_map(|app| {
                crate::packaged_icon_reader::RelativeIconPath::parse(app.icon.as_deref()?)
                    .ok()
                    .map(|path| (app.id.clone(), path))
            })
            .collect();
        let packaged_gl = crate::artwork_gl::Diagnostics::default();
        #[cfg(test)]
        packaged_gl.reuse_test_handle(1_000_000);
        let seeded = !prefs.initialized;
        if seeded {
            let preferred = [
                "com.beb.plxnative",
                "youtube.leanback.v4",
                "netflix",
                "amazon",
                "com.apple.appletv",
                "com.disney.disneyplus-prod",
                "bbc.iplayer.3.0",
                "com.crunchyroll.webapp",
            ];
            prefs.favourites.extend(
                catalogue
                    .applications
                    .iter()
                    .filter(|a| preferred.contains(&a.id.as_str()))
                    .map(|a| a.id.clone()),
            );
            if prefs.favourites.is_empty() {
                prefs
                    .favourites
                    .extend(catalogue.applications.iter().take(8).map(|a| a.id.clone()));
            }
            prefs.initialized = true;
        }
        let mut shell = Self {
            snapshot,
            catalogue,
            prefs,
            input: InputMachine::new(),
            surface: Self::blank(Page::Home),
            textures: BTreeMap::new(),
            packaged_bindings,
            current_icon_cache: false,
            current_icon_sources: None,
            current_icon_sources_observed: None,
            packaged_icons: None,
            packaged_gl,
            live_textures,
            artwork_gl,
            artwork: None,
            artwork_ever_enabled: false,
            artwork_pause: PauseReasons::default(),
            artwork_external_held: false,
            artwork_queued_anchor: None,
            artwork_filter: None,
            artwork_origin: std::time::Instant::now(),
            artwork_interest: (Vec::new(), Vec::new()),
            artwork_retry_at: 0,
            artwork_upload_failures: 0,
            artwork_catalogue_live: false,
            now: 0,
            status: cstr("Captured TV state • hold OK to pin an application"),
            pending: Vec::new(),
            exit: false,
            preferences_changed: seeded,
            live_state: None,
            live_now: 0,
            live_updates: 0,
            app_launches_enabled: false,
            home_launches_enabled: false,
            input_launches_enabled: false,
            drawer: Drawer::default(),
            system_keyboard_available: false,
            control_category: Page::Picture,
            category_labels: CATEGORIES.map(|(_, label)| cstr(label)),
            control_caption_page: None,
            control_caption_context: None,
            control_context_current: false,
            control_breadcrumb: cstr(""),
            menu: None,
            clock: crate::clock::View::default(),
            clock_time: cstr("--:--"),
            clock_source: cstr("TV time unavailable"),
            brand_texture: 0,
            navigation_textures: [0; 4],
            drawer_scroll: plx_ui::Spring::at(0.0),
            drawer_scroll_target: 0.0,
            home_return: None,
            range: None,
            range_request: None,
            description_sample: None,
            description_confirmation: None,
            pointer_enabled: false,
            pointer_pending: false,
            pointer_expected: None,
            pointer_confirmation: None,
        };
        shell.rebuild(Page::Home);
        shell
    }
    fn blank(page: Page) -> Surface {
        let mut rail = FormSection::new("Luma Home");
        for (i, (p, title)) in TABS.into_iter().enumerate() {
            rail = rail.item_keyed(
                title.to_owned(),
                RowKey(i as u32 + 1),
                RowKind::Nav(p),
                Action::Inspect(title.to_owned()),
                Row::new(title),
            );
        }
        rail = rail.item_keyed(
            "lg-home".into(),
            RowKey(5),
            RowKind::Button,
            Action::OpenLgHome,
            Row::new("LG Home"),
        );
        let mut rt = FormTable::new(2000);
        rt.set(Form::new().section(rail), None);
        Surface {
            page,
            grid: Grid::new(
                entry(page),
                GridSpec::new(282.0, 300.0).columns(4, CARD_STYLE, 480.0),
            ),
            cards: Cards { items: Vec::new() },
            home_favourites: Shelf::new(entry(page), &HOME_APP_STYLE),
            home_inputs: Shelf::new(entry(page), &HOME_INPUT_STYLE),
            input_cards: Cards { items: Vec::new() },
            rail: rt,
            table: FormTable::new(2000),
            table_mode: false,
            control_page: None,
            editing: false,
            menu: None,
        }
    }
    fn context<'a>(&'a self, measure: &'a dyn plx_machine::machine::Measure) -> Cx<'a, HomeHost> {
        let owner = InputOwner::Entry(entry(self.surface.page));
        Cx {
            views: (),
            tick: Tick {
                ms: self.now,
                dt_us: 16_667,
            },
            measure,
            press: PressRead {
                scale: self.input.press.scale(),
                is_long: self.input.press.is_long(self.now),
                held_ms: self.input.press.held_ms(self.now),
                owner: self.input.dip_owner,
            },
            focus: self.input.engine.read(owner),
            owner,
        }
    }
    fn rebuild(&mut self, page: Page) {
        let page = if page == Page::RangeEditor && self.range.is_none() {
            Page::Controls
        } else {
            page
        };
        if page != Page::RangeEditor {
            self.range = None;
            self.range_request = None;
        }

        plx_machine::idle::invalidate();
        if page != Page::Applications {
            self.drawer.finish_editing();
            self.menu = None;
        }
        if page != Page::PointerSpeed {
            self.pointer_expected = None;
        }
        if self.menu.as_ref().is_some_and(|menu| {
            !self
                .catalogue
                .applications
                .iter()
                .any(|app| app.id == menu.id)
        }) {
            self.menu = None;
        }
        self.input.cancel_press();
        self.surface = Self::blank(page);
        if root_tab(page) == Page::Controls {
            if page == Page::RangeEditor {
                self.control_category = Page::Accessibility;
            } else if page == Page::PointerSpeed {
                self.control_category = Page::General;
            } else if page != Page::Controls {
                self.control_category = page;
            }
            self.surface.control_page = Some(self.control_category);
        }
        self.surface.editing = page == Page::Applications && self.drawer.editing;
        if page == Page::Applications {
            self.surface.grid = Grid::new(
                entry(page),
                GridSpec::new(340.0, 330.0)
                    .columns(if self.menu.is_some() { 3 } else { 4 }, CARD_STYLE, 480.0)
                    .external()
                    .whole_tiles(),
            );
        }
        if page == Page::Applications {
            self.surface.grid.set_page(340.0, self.drawer_scroll.pos);
        }
        self.status = cstr(match page {
            Page::Home | Page::Applications => {
                if self.app_launches_enabled {
                    if page == Page::Applications {
                        "OK to open • hold OK for app options"
                    } else {
                        "OK to open • hold OK to add or remove a favourite"
                    }
                } else {
                    if page == Page::Applications {
                        "Hold OK for app options • launches disabled in this preview"
                    } else {
                        "Hold OK to add or remove a favourite • launches disabled in this preview"
                    }
                }
            }
            Page::Inputs if self.input_launches_enabled => "OK open source • Back Home",
            Page::Inputs => "HDMI status • source switching unavailable on this TV",
            Page::RangeEditor => "Left/Right adjust • Down choose Apply • Back cancel",
            Page::PointerSpeed if self.pointer_pending => "Applying pointer speed…",
            Page::PointerSpeed => "OK apply selection • Back cancel",
            Page::General if self.pointer_enabled => {
                "Pointer speed can be adjusted here • more controls in LG Settings"
            }
            Page::Controls if self.control_category == Page::General && self.pointer_enabled => {
                "Pointer speed can be adjusted here • more controls in LG Settings"
            }
            _ => "View only • setting changes remain available in LG Settings",
        });
        match page {
            Page::Home | Page::Applications => {
                for app in &self.catalogue.applications {
                    if page == Page::Applications
                        && !self
                            .drawer
                            .matches(&app.title, self.prefs.favourites.contains(&app.id))
                    {
                        continue;
                    }
                    if page == Page::Home && !self.prefs.favourites.contains(&app.id) {
                        continue;
                    }
                    let initials = app.title.chars().take(2).collect::<String>().to_uppercase();
                    let tex = self
                        .live_textures
                        .texture(&app.id)
                        .map(|t| (t.handle, t.width as f32, t.height as f32))
                        .or_else(|| self.textures.get(&app.id).copied())
                        .unwrap_or_default();
                    self.surface.cards.items.push(Card {
                        key: app.key.0,
                        id: app.id.clone(),
                        title: cstr(&app.title),
                        caption: cstr(if self.prefs.favourites.contains(&app.id) {
                            "Favourite"
                        } else {
                            "Application"
                        }),
                        initials: cstr(&initials),
                        texture: tex,
                        loading_icon: self
                            .packaged_icons
                            .as_ref()
                            .is_some_and(|icons| icons.will_load(&app.id)),
                        compact_labels: page == Page::Applications,
                        context_selected: self.menu.as_ref().is_some_and(|m| m.id == app.id),
                    });
                }
            }
            Page::Inputs => {
                self.surface.grid = Grid::new(
                    entry(page),
                    GridSpec::new(330.0, 320.0).columns(2, INPUT_DETAIL_STYLE, 480.0),
                );
                self.surface.cards = self.input_cards();
            }

            _ => {
                self.surface.table_mode = true;
                let detail = self.surface.control_page.unwrap_or(page);
                let name = detail
                    .title()
                    .unwrap_or("General")
                    .to_lowercase()
                    .replace(' ', "_");
                let settings = self
                    .snapshot
                    .settings
                    .get(&name)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                self.surface.table.set(
                    controls_form_with_numeric(
                        settings,
                        &self.snapshot.context,
                        SettingsSection::from_page(detail),
                        self.pointer_value().is_some() && !self.pointer_pending,
                        self.description_value(),
                    ),
                    None,
                );
                if page == Page::RangeEditor {
                    self.surface.table.set(Form::new(), None);
                }
                if page == Page::PointerSpeed {
                    let mut choices = FormSection::new("POINTER SPEED");
                    for (i, value) in crate::pointer_speed::Speed::ALL.into_iter().enumerate() {
                        let mut row = Row::new(value.label());
                        row.checked = self.pointer_expected == Some(value);
                        choices = choices
                            .item_keyed(
                                value.wire().into(),
                                RowKey(1500 + i as u32),
                                RowKind::Choice,
                                Action::SetPointerSpeed(crate::pointer_speed::Change {
                                    expected: self
                                        .pointer_expected
                                        .unwrap_or(crate::pointer_speed::Speed::Normal),
                                    value,
                                }),
                                row,
                            )
                            .disabled(self.pointer_pending || self.pointer_value().is_none());
                    }
                    choices = choices
                        .item_keyed(
                            "cancel".into(),
                            RowKey(1510),
                            RowKind::Button,
                            Action::ClosePointerSpeed,
                            Row::new("Cancel"),
                        )
                        .disabled(self.pointer_pending);
                    self.surface.table.set(Form::new().section(choices), None);
                }
                // Known read-only state stays legible. The shared form binding remains
                // disabled, so changing ink never enables an LG-managed operation.
                if let Some(section) = self
                    .surface
                    .table
                    .table
                    .sections
                    .first_mut()
                    .filter(|_| page != Page::PointerSpeed)
                {
                    for (row, setting) in section.rows.iter_mut().zip(settings) {
                        if setting.protected
                            && setting.available
                            && setting.still_current(&self.snapshot.context)
                            && matches!(
                                setting.constraint,
                                Some(
                                    crate::model::ControlConstraint::Range { active: true, .. }
                                        | crate::model::ControlConstraint::Choices {
                                            active: true,
                                            ..
                                        }
                                )
                            )
                        {
                            row.dim = false;
                        }
                    }
                }
            }
        }
        if page == Page::Home || (page == Page::Applications && self.drawer.favourites_only) {
            let order = self.prefs.ordered_favourites(&self.catalogue);
            self.surface.cards.items.sort_by_key(|card| {
                order
                    .iter()
                    .position(|id| id == &card.id)
                    .unwrap_or(usize::MAX)
            });
        }
        if page == Page::Home {
            self.surface.input_cards = self.input_cards();
        }
        if let Some(menu) = &self.menu {
            let order = self.prefs.ordered_favourites(&self.catalogue);
            let rank = order.iter().position(|id| id == &menu.id);
            let mut section = FormSection::new("APP OPTIONS");
            for (action, label, disabled) in [
                (
                    MenuAction::Open,
                    "Open app",
                    !self.app_launches_enabled || !self.request_is_current(false),
                ),
                (
                    MenuAction::ToggleFavourite,
                    if rank.is_some() {
                        "Unpin favourite"
                    } else {
                        "Pin favourite"
                    },
                    false,
                ),
                (
                    MenuAction::Earlier,
                    "Move earlier",
                    rank.is_none_or(|i| i == 0),
                ),
                (
                    MenuAction::Later,
                    "Move later",
                    rank.is_none_or(|i| i + 1 >= order.len()),
                ),
                (MenuAction::Close, "Close menu", false),
            ] {
                section = section
                    .item_keyed(
                        action,
                        RowKey(action.key()),
                        RowKind::Button,
                        action,
                        Row::new(label),
                    )
                    .disabled(disabled);
            }
            let mut table = FormTable::new(10);
            table.set(Form::new().section(section), None);
            self.surface.menu = Some(table);
        }
        self.refresh_control_breadcrumb();
        let owner = InputOwner::Entry(entry(page));
        let old = self.input.engine.current(owner);
        let cx = Cx::<HomeHost> {
            views: (),
            tick: Tick::default(),
            measure: &LegacyMeasure,
            press: PressRead::default(),
            focus: self.input.engine.read(owner),
            owner,
        };
        let outcome = self.input.engine.enter(
            owner,
            &self.surface,
            FocusTarget::ContainerGroup(GroupId(if page == Page::RangeEditor {
                9
            } else if self.surface.control_page.is_some()
                && (page == Page::Controls || self.surface.keys().is_empty())
            {
                6
            } else if self.surface.menu.is_some() {
                5
            } else if self.surface.keys().is_empty() {
                if page == Page::Applications {
                    3
                } else {
                    0
                }
            } else if self.surface.editing {
                4
            } else {
                if page == Page::Home && self.surface.cards.items.is_empty() {
                    2
                } else {
                    1
                }
            })),
            old,
            &cx,
        );
        self.moved(outcome);
    }
    fn input_cards(&self) -> Cards {
        Cards {
            items: self
                .snapshot
                .inputs
                .iter()
                .filter_map(|input| {
                    let key = input_key(&input.id)?;
                    let caption = format!(
                        "{}{}\n{}",
                        if input.selected == Some(true) {
                            "Selected • "
                        } else {
                            ""
                        },
                        if input.plugged {
                            "Connected"
                        } else {
                            "No cable"
                        },
                        if input.signal {
                            "Signal present"
                        } else {
                            "No signal"
                        }
                    );
                    Some(Card {
                        key,
                        id: input.id.clone(),
                        title: cstr(&input.label),
                        caption: cstr(&caption),
                        initials: cstr(&input.id.replace('_', " ")),
                        texture: (0, 0.0, 0.0),
                        loading_icon: false,
                        context_selected: false,
                        compact_labels: false,
                    })
                })
                .collect(),
        }
    }
    pub fn apply_live(&mut self, update: &Update, now: u32) {
        let current_sources = match &update.artwork {
            live::Reading::Value(sources)
                if !update.simulated
                    && matches!(update.catalogue, live::Reading::Value(_))
                    && Freshness::current(Some(update.observed_ms), now)
                    && sources.len() <= crate::packaged_icons::MAX_SOURCES =>
            {
                Some(sources)
            }
            _ => None,
        };
        self.update_current_icon_sources(
            current_sources,
            current_sources.map(|_| update.observed_ms),
        );
        if let Some(artwork) = &mut self.artwork {
            let filtered;
            let sources = match &update.artwork {
                live::Reading::Value(sources)
                    if matches!(update.catalogue, live::Reading::Value(_))
                        && Freshness::current(Some(update.observed_ms), now) =>
                {
                    if let Some(ids) = &self.artwork_filter {
                        filtered = sources
                            .iter()
                            .filter(|(id, _)| ids.contains(*id))
                            .map(|(id, s)| (id.clone(), s.clone()))
                            .collect();
                        Some(&filtered)
                    } else {
                        Some(sources)
                    }
                }
                _ => None,
            };
            self.artwork_catalogue_live = sources.is_some();
            if artwork.update_catalogue(sources).is_err() {
                let _ = artwork.update_catalogue(None);
            }
        }
        self.system_keyboard_available = update.system_keyboard;
        let before = self.snapshot.clone();
        let mut fresh = self.live_state.take().unwrap_or_default();
        live::merge(&mut self.snapshot, update, &mut fresh);
        if let Some((stamp, value)) = self.pointer_confirmation {
            if Freshness::current(Some(stamp), now)
                && now.wrapping_sub(update.observed_ms) > now.wrapping_sub(stamp)
            {
                if let Some(row) = self
                    .snapshot
                    .settings
                    .get_mut("general")
                    .and_then(|rows| rows.iter_mut().find(|r| r.id == "pointerSpeed"))
                {
                    row.value = value.wire().into();
                }
            } else {
                self.pointer_confirmation = None;
            }
        }
        self.live_state = Some(fresh);
        self.live_now = now;
        self.live_updates += 1;
        self.catalogue
            .refresh(&self.snapshot.applications, &self.snapshot.launch_ids);
        self.refresh_shown_data(&before);
        self.validate_pointer_picker();
        self.refresh_control_breadcrumb();
        // Refresh validity/status chrome even when values themselves are unchanged.
        plx_machine::idle::invalidate();
    }
    pub fn age_live(&mut self, now: u32) {
        self.live_now = now;
        if self.current_icon_sources.is_some()
            && !Freshness::current(self.current_icon_sources_observed, now)
        {
            self.update_current_icon_sources(None, None);
        }
        if self.artwork_catalogue_live && !self.artwork_catalogue_current() {
            self.artwork_catalogue_live = false;
            if let Some(artwork) = &mut self.artwork {
                let _ = artwork.update_catalogue(None);
            }
        }
        if let Some(fresh) = &self.live_state {
            if live::needs_expiry(&self.snapshot, fresh, now) {
                plx_machine::idle::invalidate();
                let before = self.snapshot.clone();
                live::expire(&mut self.snapshot, fresh, now);
                self.refresh_shown_data(&before);
            }
        }
        self.validate_pointer_picker();
        self.refresh_control_breadcrumb();
    }
    fn refresh_control_breadcrumb(&mut self) {
        let page = if self.surface.page == Page::RangeEditor {
            Some(Page::RangeEditor)
        } else if self.surface.page == Page::PointerSpeed {
            Some(Page::PointerSpeed)
        } else {
            self.surface.control_page
        };
        let current = self
            .live_state
            .as_ref()
            .is_some_and(|f| Freshness::current(f.picture, self.live_now));
        if self.control_caption_page == page
            && self.control_context_current == current
            && self.control_caption_context.as_ref() == Some(&self.snapshot.context)
        {
            return;
        }
        plx_machine::idle::invalidate();
        self.control_breadcrumb = match page {
            Some(Page::Picture) if current => cstr(&format!(
                "{} / {} / {}",
                self.snapshot.context.input,
                self.snapshot.context.dynamic_range.to_uppercase(),
                self.snapshot.context.picture_mode
            )),
            Some(Page::Picture) => cstr("Picture context unavailable"),
            Some(Page::Sound) => cstr("Sound readings • view only"),
            Some(Page::Network) => cstr("Connection status • view only"),
            Some(Page::PanelCare) => cstr("OLED protections stay managed by LG"),
            Some(Page::RangeEditor) => cstr("Staged value • changes require explicit Apply"),
            Some(Page::PointerSpeed) => cstr("Choose a value • press OK to apply"),
            Some(Page::General) if self.pointer_enabled => {
                cstr("Pointer speed can be adjusted here")
            }
            Some(Page::Accessibility) => cstr("Accessibility readings • LG-managed controls"),
            Some(_) => cstr("General readings • LG-managed controls"),
            None => cstr(""),
        };
        self.control_caption_page = page;
        self.control_context_current = current;
        self.control_caption_context = Some(self.snapshot.context.clone());
    }
    fn refresh_shown_data(&mut self, before: &Snapshot) {
        let page = self.surface.page;
        let changed = match page {
            Page::Home => {
                before.applications != self.snapshot.applications
                    || before.launch_ids != self.snapshot.launch_ids
                    || before.inputs != self.snapshot.inputs
            }
            Page::Applications => {
                before.applications != self.snapshot.applications
                    || before.launch_ids != self.snapshot.launch_ids
            }
            Page::Inputs => before.inputs != self.snapshot.inputs,
            Page::Controls => {
                before.context != self.snapshot.context || before.settings != self.snapshot.settings
            }
            _ => {
                before.context != self.snapshot.context || before.settings != self.snapshot.settings
            }
        };
        if !changed {
            return;
        }
        // Keep the shared grid's motion/viewport on a data refresh; the engine owns the key.
        let grid = std::mem::replace(
            &mut self.surface.grid,
            Grid::new(
                entry(page),
                GridSpec::new(282.0, 300.0).columns(4, CARD_STYLE, 480.0),
            ),
        );
        let favourites = std::mem::replace(
            &mut self.surface.home_favourites,
            Shelf::new(entry(page), &HOME_APP_STYLE),
        );
        let inputs = std::mem::replace(
            &mut self.surface.home_inputs,
            Shelf::new(entry(page), &HOME_INPUT_STYLE),
        );
        self.rebuild(page);
        self.surface.grid = grid;
        if page == Page::Applications {
            self.surface.grid.set_columns(
                if self.menu.is_some() { 3 } else { 4 },
                CARD_STYLE,
                480.0,
            );
            self.surface.grid.set_page(340.0, self.drawer_scroll.pos);
        }
        self.surface.home_favourites = favourites;
        self.surface.home_inputs = inputs;
        let owner = InputOwner::Entry(entry(page));
        if let Some(to) = self.input.engine.current(owner) {
            self.moved(plx_ui::focus::Outcome::Moved {
                from: Some(to),
                to,
                by: By::Restore,
            });
        }
    }
    fn request_is_current(&self, input: bool) -> bool {
        let Some(fresh) = &self.live_state else {
            return true;
        };
        Freshness::current(
            if input { fresh.inputs } else { fresh.catalogue },
            self.live_now,
        )
    }
    pub fn enable_pointer_editor(&mut self, enabled: bool) {
        self.pointer_enabled = enabled;
        self.control_caption_page = None;
        self.rebuild(self.surface.page);
    }
    pub fn pointer_value(&self) -> Option<crate::pointer_speed::Speed> {
        if !self.pointer_enabled
            || !self.live_state.as_ref().is_some_and(|f| {
                Freshness::current(
                    f.controls
                        .get(&crate::control_reads::Section::General)
                        .copied(),
                    self.live_now,
                )
            })
        {
            return None;
        }
        let row = self.snapshot.settings.get("general")?.iter().find(|r| {
            r.id == "pointerSpeed" && r.available && r.still_current(&self.snapshot.context)
        })?;
        if !matches!(&row.constraint,Some(crate::model::ControlConstraint::Choices{active:true,values}) if values.len()==3 && crate::pointer_speed::Speed::ALL.iter().all(|s|values.iter().any(|v|v==s.wire())))
        {
            return None;
        }
        crate::pointer_speed::Speed::parse(&serde_json::json!(row.value))
    }
    pub fn focus_pointer_setting(&mut self) -> bool {
        if self.surface.control_page != Some(Page::General)
            || self.surface.page == Page::PointerSpeed
            || self.pointer_value().is_none()
        {
            return false;
        }
        self.focus_special(TABLE_BASE + 141, GroupId(1));
        true
    }
    pub fn focus_pointer_choice(&mut self, value: crate::pointer_speed::Speed) -> bool {
        if self.surface.page != Page::PointerSpeed || self.pointer_pending {
            return false;
        }
        let i = crate::pointer_speed::Speed::ALL
            .iter()
            .position(|s| *s == value)
            .unwrap();
        self.focus_special(TABLE_BASE + 1500 + i as u32, GroupId(1));
        true
    }
    fn open_pointer_picker(&mut self) {
        if let Some(value) = self.pointer_value() {
            self.pointer_expected = Some(value);
            self.rebuild(Page::PointerSpeed);
            self.focus_pointer_choice(value);
        }
    }
    fn close_pointer_picker(&mut self) {
        self.pointer_expected = None;
        self.rebuild(Page::General);
        self.focus_pointer_setting();
    }
    fn validate_pointer_picker(&mut self) {
        if self.surface.page == Page::PointerSpeed
            && !self.pointer_pending
            && self.pointer_expected != self.pointer_value()
        {
            self.close_pointer_picker();
            self.status = cstr("Pointer speed changed • choose again");
        }
    }
    pub fn pointer_reply(&mut self, outcome: &crate::pointer_speed::Outcome, now: u32) {
        self.pointer_pending = false;
        if outcome.verified {
            if let Some(row) = self
                .snapshot
                .settings
                .get_mut("general")
                .and_then(|rows| rows.iter_mut().find(|r| r.id == "pointerSpeed"))
            {
                row.value = outcome.change.value.wire().into();
            }
            self.pointer_confirmation = Some((now, outcome.change.value));
            self.status = cstr("Pointer speed updated");
        } else {
            self.pointer_enabled = false;
        }
        if self.surface.page == Page::PointerSpeed {
            self.close_pointer_picker();
        } else {
            self.rebuild(self.surface.page);
        }
        self.status = cstr(if outcome.verified {
            "Pointer speed updated"
        } else {
            "Pointer speed could not be confirmed • use LG Settings"
        });
    }
    pub fn input_launches_enabled(&self) -> bool {
        self.input_launches_enabled
    }
    pub fn enable_input_launches(&mut self, enabled: bool) {
        self.input_launches_enabled = enabled;
        self.rebuild(self.surface.page);
    }
    pub fn enable_home_launches(&mut self, enabled: bool) {
        self.home_launches_enabled = enabled;
        plx_machine::idle::invalidate();
    }
    pub fn focus_lg_home(&mut self) -> bool {
        if self.menu.is_some() || self.drawer.editing {
            return false;
        }
        self.focus_special(LG_HOME_KEY, GroupId(0));
        true
    }
    pub fn enable_app_launches(&mut self, enabled: bool) {
        self.app_launches_enabled = enabled;
        self.rebuild(self.surface.page);
    }
    /// Diagnostic-only readout receipt; the normal interactive package does not persist this.
    pub fn control_readout_proof(&self) -> serde_json::Value {
        ["picture", "sound", "accessibility", "general", "panel_care"]
            .into_iter()
            .map(|name| {
                (
                    name.to_owned(),
                    serde_json::json!(self.snapshot.settings.get(name)),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into()
    }
    pub fn live_updates(&self) -> u32 {
        self.live_updates
    }
    pub fn set_clock(&mut self, view: &crate::clock::View) {
        if self.clock != *view {
            plx_machine::idle::invalidate();
            self.clock_time = cstr(&view.time);
            self.clock_source = cstr(&view.source);
            self.clock = view.clone();
        }
    }
    /// Empty-result copy reflects the current filter and reading validity.
    /// This changes no focus target, catalogue, action or persistence state.
    fn drawer_empty_copy(&self) -> Option<(&'static std::ffi::CStr, &'static std::ffi::CStr)> {
        if self.surface.page != Page::Applications || !self.surface.cards.items.is_empty() {
            return None;
        }
        if self
            .live_state
            .as_ref()
            .is_some_and(|fresh| !Freshness::current(fresh.catalogue, self.live_now))
        {
            return Some((
                c"Apps unavailable",
                c"Use the sidebar to open Home, Inputs or Controls.",
            ));
        }
        if self.catalogue.applications.is_empty() {
            return Some((c"No apps to show", c"Use the sidebar to return Home."));
        }
        let searching = !self.drawer.query().trim().is_empty();
        match (self.drawer.favourites_only, searching) {
            (true, false) => Some((
                c"No favourites yet",
                c"Choose All apps, then hold OK on an app to add it.",
            )),
            (true, true) => Some((
                c"No matching favourites",
                c"Choose All apps to search the full list.",
            )),
            (false, _) => Some((
                c"No matching apps",
                c"Change your search or clear it with Keypad.",
            )),
        }
    }

    fn reading_label(&self) -> String {
        let shown = self.surface.control_page.unwrap_or(self.surface.page);
        let Some(fresh) = &self.live_state else {
            return "Updating TV state…".into();
        };
        if shown == Page::Sound {
            let core = Freshness::current(fresh.sound, self.live_now);
            let optional = Freshness::current(
                fresh
                    .controls
                    .get(&crate::control_reads::Section::Sound)
                    .copied(),
                self.live_now,
            );
            let current_row = |row: &Setting| {
                let optional_row = crate::sound_reads::Key::ALL
                    .iter()
                    .any(|key| row.id == key.id() && row.focus_key == key.focus_key());
                row.available
                    && row.still_current(&self.snapshot.context)
                    && if optional_row { optional } else { core }
            };
            let rows = self.snapshot.settings.get("sound");
            if !rows.is_some_and(|rows| rows.iter().any(current_row)) {
                return "Reading unavailable • last state kept".into();
            }
            let partial = rows.is_some_and(|rows| rows.iter().any(|row| !current_row(row)));
            if core != optional || partial {
                return if fresh.simulated {
                    "Simulated update • some readings unavailable"
                } else {
                    "Live TV state • some readings unavailable"
                }
                .into();
            }
        }
        let stamp = match shown {
            Page::Home | Page::Applications => fresh.catalogue,
            Page::Inputs => fresh.inputs,
            Page::Sound => fresh.sound,
            Page::Network => fresh.network,
            Page::Picture => fresh.picture,
            Page::Accessibility => fresh
                .controls
                .get(&crate::control_reads::Section::Accessibility)
                .copied(),
            Page::General => fresh
                .controls
                .get(&crate::control_reads::Section::General)
                .copied(),
            Page::PanelCare => fresh
                .controls
                .get(&crate::control_reads::Section::PanelCare)
                .copied(),
            _ => fresh.picture,
        };
        if !Freshness::current(stamp, self.live_now) {
            return "Reading unavailable • last state kept".into();
        }
        let label = if fresh.simulated {
            "Simulated update"
        } else {
            "Live TV state"
        };
        label.into()
    }
    pub fn load_icons(&mut self, assets: &std::path::Path) {
        for (i, svg) in [
            include_str!("../assets/luma-home/navigation/home.svg"),
            include_str!("../assets/luma-home/navigation/apps.svg"),
            include_str!("../assets/luma-home/navigation/inputs.svg"),
            include_str!("../assets/luma-home/navigation/controls.svg"),
        ]
        .into_iter()
        .enumerate()
        {
            if self.navigation_textures[i] == 0 {
                if let Some(pixels) = plx_gfx::svg::rasterize(svg, 32, 32) {
                    self.navigation_textures[i] =
                        plx_gfx::gfx::upload_rgba(0, 32, 32, pixels.as_ptr());
                }
            }
        }
        if self.brand_texture == 0 {
            if let Some((w, h, pixels)) =
                plx_gfx::img::img_decode_owned(include_bytes!("../assets/luma-home/icon-160.png"))
            {
                self.brand_texture =
                    plx_gfx::gfx::upload_rgba(0, w as i32, h as i32, pixels.as_ptr());
            }
        }
        self.configure_packaged_icons(assets);
        self.rebuild(self.surface.page);
    }
    /// Configuration only, before load_icons. No profile/cache/self-ELF I/O.
    /// Main selects this only for actual interactive native Preview.
    pub fn enable_current_icon_cache(&mut self) {
        self.current_icon_cache = true;
    }
    fn update_current_icon_sources(
        &mut self,
        sources: Option<&crate::app_artwork::SourceMap>,
        observed: Option<u32>,
    ) -> bool {
        self.current_icon_sources_observed = if sources.is_some() { observed } else { None };
        if self.current_icon_sources.as_ref() == sources {
            return false; // Renew only the source clock; no refresh/epoch churn.
        }
        self.current_icon_sources = sources.cloned();
        if self.current_icon_cache {
            let failed = self.packaged_icons.as_mut().is_some_and(|icons| {
                icons
                    .update_sources(self.current_icon_sources.as_ref())
                    .is_err()
            });
            if failed {
                self.current_icon_sources = None;
                self.current_icon_sources_observed = None;
                if let Some(icons) = &mut self.packaged_icons {
                    let _ = icons.update_sources(None);
                }
            }
            self.withdraw_stale_icon_references();
        }
        true
    }
    fn configure_packaged_icons(&mut self, assets: &std::path::Path) {
        let _ = self.finish_packaged_icons();
        self.textures.clear();
        let backend = crate::artwork_gl::Backend::new(self.packaged_gl.clone());
        if self.current_icon_cache {
            // Starts even with no packaged bindings: later live catalogue entries
            // can select ordinary cache images. Profile/cache work stays on worker.
            self.packaged_icons = crate::packaged_icons::PackagedIcons::start_current_cache(
                assets.to_owned(),
                self.packaged_bindings.clone(),
                backend,
                crate::durable_icon_cache::Config::default(),
            )
            .ok();
            if let Some(icons) = &mut self.packaged_icons {
                let sources = self.current_icon_sources.as_ref().filter(|_| {
                    Freshness::current(self.current_icon_sources_observed, self.live_now)
                });
                if icons.update_sources(sources).is_err() {
                    let _ = icons.update_sources(None);
                }
            }
        } else if !self.packaged_bindings.is_empty() {
            self.packaged_icons = crate::packaged_icons::PackagedIcons::start(
                assets.to_owned(),
                self.packaged_bindings.clone(),
                backend,
            )
            .ok();
        }
    }
    /// Pure reference withdrawal only: do not poll/dispatch/upload during live
    /// reconciliation before the current frame's keyboard/external intent gate.
    fn withdraw_stale_icon_references(&mut self) {
        let Some(icons) = &self.packaged_icons else {
            return;
        };
        let mut removed = BTreeSet::new();
        self.textures.retain(|id, _| {
            let keep = icons.texture(id).is_some();
            if !keep {
                removed.insert(id.clone());
            }
            keep
        });
        for card in &mut self.surface.cards.items {
            let loading = icons.will_load(&card.id);
            if card.loading_icon != loading {
                removed.insert(card.id.clone());
            }
            card.loading_icon = loading;
        }
        self.refresh_artwork_cards();
        if self.ordinary_icons_allowed()
            && self
                .packaged_visible()
                .iter()
                .any(|id| removed.contains(id))
        {
            plx_machine::idle::invalidate();
        }
    }
    fn packaged_visible(&self) -> Vec<String> {
        if !matches!(self.surface.page, Page::Home | Page::Applications) {
            return Vec::new();
        }
        let cx = self.context(&LegacyMeasure);
        let focused_key = self
            .input
            .engine
            .current(InputOwner::Entry(entry(self.surface.page)))
            .map(|key| key.elem);
        let mut visible = self
            .surface
            .cards
            .items
            .iter()
            .filter_map(|card| {
                let (placed, clip) = if self.surface.page == Page::Home {
                    (
                        self.surface.home_favourites.place(
                            &cx,
                            &self.surface.cards,
                            &card.key,
                            HOME_FAVOURITES,
                            At::Drawn,
                        )?,
                        HOME_FAVOURITES.clip,
                    )
                } else {
                    (
                        self.surface
                            .grid
                            .place(&cx, &self.surface.cards, &card.key, At::Drawn)?,
                        self.surface.grid_area(),
                    )
                };
                let r = placed.rect;
                // Applications uses GridSpec::whole_tiles(): unfocused partial
                // tiles are not painted. Focused motion keeps its clipped image.
                if self.surface.page == Page::Applications
                    && focused_key != Some(card.key)
                    && !(r.x >= clip.x
                        && r.y >= clip.y
                        && r.x + r.w <= clip.x + clip.w
                        && r.y + r.h <= clip.y + clip.h)
                {
                    return None;
                }
                (r.w > 0.0
                    && r.h > 0.0
                    && r.x < clip.x + clip.w
                    && r.x + r.w > clip.x
                    && r.y < clip.y + clip.h
                    && r.y + r.h > clip.y)
                    .then(|| card.id.clone())
            })
            .collect::<Vec<_>>();
        if let Some(focused) = self.focused_item() {
            if let Some(index) = visible.iter().position(|id| id == &focused) {
                visible.swap(0, index);
            }
        }
        visible
    }
    fn ordinary_icons_allowed(&self) -> bool {
        !self.exit
            && ![
                PauseReason::ForegroundLost,
                PauseReason::Idle,
                PauseReason::KeyboardIntent,
                PauseReason::KeyboardActive,
                PauseReason::ExternalIntent,
            ]
            .into_iter()
            .any(|reason| self.artwork_pause.contains(reason))
    }
    fn pump_packaged_icons(&mut self) {
        if self.packaged_icons.is_none() {
            return;
        }
        let allowed = self.ordinary_icons_allowed();
        let visible = if allowed {
            self.packaged_visible()
        } else {
            Vec::new()
        };
        let ids = visible.iter().map(String::as_str).collect::<Vec<_>>();
        let controller = self
            .packaged_icons
            .as_mut()
            .expect("Packaged controller exists");
        let update = controller.frame(&ids, allowed);
        let mut references_changed = false;
        let mut visible_references_changed = false;
        // Check each borrowed reference even on a paused/reset frame. Only
        // current controller-owned handles may reach the next draw.
        self.textures.retain(|id, _| {
            let keep = controller.texture(id).is_some();
            if !keep {
                references_changed = true;
                visible_references_changed |= visible.contains(id);
            }
            keep
        });
        for id in &visible {
            if let Some(view) = controller.texture(id) {
                let next = (view.handle, view.width as f32, view.height as f32);
                if self.textures.get(id) != Some(&next) {
                    self.textures.insert(id.clone(), next);
                    references_changed = true;
                    visible_references_changed = true;
                }
            }
        }
        for card in &mut self.surface.cards.items {
            let loading = controller.will_load(&card.id);
            if card.loading_icon != loading {
                card.loading_icon = loading;
                references_changed = true;
                visible_references_changed |= visible.contains(&card.id);
            }
        }
        if references_changed {
            self.refresh_artwork_cards();
        }
        if allowed && (visible_references_changed || update.visible_pixels_changed) {
            plx_machine::idle::invalidate();
        }
    }
    pub fn finish_packaged_icons(&mut self) -> Option<serde_json::Value> {
        let mut controller = self.packaged_icons.take()?;
        let before = controller.stats();
        let done = controller.finish();
        self.textures.clear();
        for card in &mut self.surface.cards.items {
            card.loading_icon = false;
        }
        self.refresh_artwork_cards();
        Some(serde_json::json!({"worker_joined":done.worker_joined,
            "worker_panicked":done.worker_panicked,"remaining_textures":done.remaining_textures,
            "submitted":before.submitted,"completed":before.completed,
            "discarded":before.discarded,"unavailable_sources":before.unavailable_sources,
            "texture_bytes_before_cleanup":before.texture_bytes_estimate,
            "cache_readiness":format!("{:?}",before.cache_readiness),
            "cache_readiness_is_index_not_images":true,
            "cache_uploads":before.cache_uploads,"packaged_uploads":before.packaged_uploads,
            "cache_adoptions":before.cache_adoptions,
            "gl":self.packaged_gl.receipt()}))
    }
    /// Diagnostic readiness only. Ordinary navigation never waits for artwork.
    pub fn packaged_icons_settled(&self) -> bool {
        self.packaged_icons.as_ref().is_none_or(|icons| {
            self.packaged_visible()
                .iter()
                .all(|id| icons.texture(id).is_some() || !icons.will_load(id))
        })
    }
    pub fn enable_live_artwork(
        &mut self,
        runtime: crate::artwork_runtime::Runtime,
        filter: Option<BTreeSet<String>>,
    ) {
        let _ = self.finish_live_artwork();
        self.artwork_ever_enabled = true;
        self.artwork = Some(runtime);
        self.artwork_filter = filter;
        self.artwork_origin = std::time::Instant::now();
        self.artwork_retry_at = 0;
        self.artwork_interest = (Vec::new(), Vec::new());
        self.reconcile_artwork_pause();
    }
    /// Compatibility/manual ownership only: true cannot clear any other owner.
    pub fn set_artwork_active(&mut self, active: bool) {
        self.set_artwork_pause(PauseReason::Manual, !active);
    }
    pub fn artwork_transition_enabled(&self) -> bool {
        self.artwork_ever_enabled
    }
    pub fn set_artwork_pause(&mut self, reason: PauseReason, paused: bool) {
        if reason == PauseReason::Quarantine && !paused {
            return;
        }
        self.artwork_pause.set(reason, paused);
        self.reconcile_artwork_pause();
    }
    pub fn artwork_pauses(&self) -> PauseReasons {
        self.artwork_pause
    }
    pub fn set_artwork_external_held(&mut self, held: bool) {
        self.artwork_external_held = held;
        self.reconcile_artwork_pause();
    }
    fn reconcile_artwork_pause(&mut self) {
        self.artwork_pause
            .set(PauseReason::KeyboardIntent, self.drawer.system_keyboard());
        self.artwork_pause.set(
            PauseReason::ExternalIntent,
            self.artwork_external_held
                || self
                    .pending
                    .iter()
                    .any(|a| crate::actions::Request::from_action(a.clone()).is_some()),
        );
        if let Some(runtime) = &mut self.artwork {
            if runtime.transition_readiness() == TransitionReadiness::BlockedUnknown {
                self.artwork_pause.set(PauseReason::Quarantine, true);
            }
            runtime.set_active(self.artwork_pause.is_empty());
        }
    }
    pub fn artwork_transition_readiness(&self) -> TransitionReadiness {
        if self.artwork_pause.contains(PauseReason::Quarantine) {
            TransitionReadiness::BlockedUnknown
        } else {
            self.artwork
                .as_ref()
                .map_or(TransitionReadiness::Disabled, |r| r.transition_readiness())
        }
    }
    fn artwork_context_ready(&self) -> bool {
        !self.artwork_pause.contains(PauseReason::ForegroundLost)
            && !self.artwork_pause.contains(PauseReason::Idle)
            && !self.artwork_pause.contains(PauseReason::Quarantine)
    }
    pub fn keyboard_transition_ready(&mut self) -> bool {
        if !self.artwork_ever_enabled {
            return true;
        }
        self.reconcile_artwork_pause();
        self.artwork_context_ready()
            && !self.artwork_pause.contains(PauseReason::ExternalIntent)
            && matches!(
                self.artwork_transition_readiness(),
                TransitionReadiness::Ready | TransitionReadiness::Disabled
            )
    }
    pub fn external_transition_ready(&mut self) -> bool {
        if !self.artwork_ever_enabled {
            return true;
        }
        self.reconcile_artwork_pause();
        self.artwork_context_ready()
            && !self.artwork_pause.contains(PauseReason::KeyboardIntent)
            && !self.artwork_pause.contains(PauseReason::KeyboardActive)
            && matches!(
                self.artwork_transition_readiness(),
                TransitionReadiness::Ready | TransitionReadiness::Disabled
            )
    }
    pub fn artwork_intent_anchor(&self) -> ArtworkAnchor {
        ArtworkAnchor {
            page: self.surface.page,
            focused: self.focused_item(),
            key: self
                .input
                .engine
                .current(InputOwner::Entry(entry(self.surface.page)))
                .map(|k| k.elem),
        }
    }
    pub fn external_request_current(&self, request: &crate::actions::Request) -> bool {
        use crate::actions::Request;
        match request {
            Request::Application { id } => {
                self.app_launches_enabled
                    && self.request_is_current(false)
                    && self.catalogue.applications.iter().any(|app| &app.id == id)
            }
            Request::Input { id } => {
                self.input_launches_enabled
                    && self.request_is_current(true)
                    && input_key(id).is_some()
                    && self.snapshot.inputs.iter().any(|input| &input.id == id)
            }
            Request::LgHome {} => self.home_launches_enabled,
            Request::LgSettings {} | Request::LgSettingsSection { .. } => {
                request.settings_section().is_some_and(|section| {
                    section.page() == self.surface.page
                        && self
                            .surface
                            .table
                            .index_of_key(RowKey(section.focus_key()))
                            .and_then(|i| self.surface.table.binding_at(i))
                            .is_some_and(|binding| !binding.disabled)
                })
            }
        }
    }
    fn artwork_visible(&self) -> Vec<String> {
        if self.surface.page == Page::Home {
            return self
                .surface
                .cards
                .items
                .iter()
                .take(12)
                .map(|c| c.id.clone())
                .collect();
        }
        if self.surface.page != Page::Applications {
            return Vec::new();
        }
        let focused = self.focused_item();
        let index = self
            .surface
            .cards
            .items
            .iter()
            .position(|c| Some(c.id.as_str()) == focused.as_deref())
            .unwrap_or(0);
        let area = self.surface.grid_area();
        let cx = self.context(&LegacyMeasure);
        self.surface
            .cards
            .items
            .iter()
            .skip(index.saturating_sub(16))
            .take(36)
            .filter(|card| {
                self.surface
                    .grid
                    .place(&cx, &self.surface.cards, &card.key, At::SpringTarget)
                    .is_some_and(|p| {
                        p.rect.x < area.x + area.w
                            && p.rect.x + p.rect.w > area.x
                            && p.rect.y < area.y + area.h
                            && p.rect.y + p.rect.h > area.y
                    })
            })
            .map(|c| c.id.clone())
            .collect()
    }
    /// Replace only drawing references. Do not rebuild focus/layout or cancel
    /// an armed press merely because new pixels became available.
    fn refresh_artwork_cards(&mut self) -> bool {
        let mut changed = false;
        for card in &mut self.surface.cards.items {
            let next = self
                .live_textures
                .texture(&card.id)
                .map(|t| (t.handle, t.width as f32, t.height as f32))
                .or_else(|| self.textures.get(&card.id).copied())
                .unwrap_or_default();
            if card.texture != next {
                card.texture = next;
                changed |= self.artwork_interest.0.contains(&card.id);
            }
        }
        changed
    }
    fn pump_artwork(&mut self) {
        self.reconcile_artwork_pause();
        if self.artwork.is_none() {
            return;
        }
        let visible = self.artwork_visible();
        let favourites = self.prefs.favourites.iter().cloned().collect::<Vec<_>>();
        let mut candidates = visible.clone();
        candidates.extend(favourites.iter().cloned());
        let runtime = self.artwork.as_mut().expect("artwork enabled");
        if self.artwork_interest.0 != visible || self.artwork_interest.1 != favourites {
            let _ = runtime.set_interest(&visible, &favourites);
            self.artwork_interest = (visible, favourites);
        }
        let now = self.artwork_origin.elapsed().as_millis() as u64;
        let frame = runtime.frame(now);
        if runtime.transition_readiness() == TransitionReadiness::BlockedUnknown {
            self.artwork_pause.set(PauseReason::Quarantine, true);
        }
        for key in &frame.evicted {
            self.live_textures.evict(key);
        }
        if let Some(key) = frame.changed {
            candidates.insert(0, key.source.app_id);
        }
        let mut visible_pixels_changed = false;
        if runtime.active() && now >= self.artwork_retry_at {
            for id in candidates {
                let Some(view) = runtime.resident(&id) else {
                    continue;
                };
                if self
                    .live_textures
                    .texture(&id)
                    .is_some_and(|t| t.key == view.key)
                {
                    continue;
                }
                // Exactly one backend upload attempt per frame, including a
                // retry after a backend allocation failure.
                match self.live_textures.install(&view) {
                    Ok(crate::artwork_textures::InstallOutcome::Uploaded { .. }) => {
                        visible_pixels_changed = self.artwork_interest.0.contains(&id);
                    }
                    Err(_) => {
                        self.artwork_upload_failures += 1;
                        self.artwork_retry_at = now.saturating_add(5000);
                    }
                    _ => {}
                }
                break;
            }
        }
        if self.refresh_artwork_cards() || visible_pixels_changed {
            plx_machine::idle::invalidate();
        }
    }
    pub fn live_artwork_receipt(&self) -> serde_json::Value {
        let textures = self.live_textures.stats();
        serde_json::json!({"runtime":self.artwork.as_ref().map(|r|r.receipt()),
            "textures":{"entries":textures.entries,"rgba_bytes_estimate":textures.rgba_bytes_estimate},
            "gl":self.artwork_gl.receipt(),"upload_failures":self.artwork_upload_failures,"pause_reasons":self.artwork_pause.bits(),"transition_readiness":format!("{:?}",self.artwork_transition_readiness())})
    }
    pub fn artwork_target(&mut self, id: &str, hash_pixels: bool) -> Option<serde_json::Value> {
        let runtime = self.artwork.as_mut()?;
        let view = runtime.resident(id)?;
        let texture = self.live_textures.texture(id)?;
        if texture.key != view.key {
            return None;
        }
        let hex = |b: &[u8]| b.iter().map(|v| format!("{v:02x}")).collect::<String>();
        Some(
            serde_json::json!({"source_key":view.key.source,"revision":view.key.revision,
            "encoded_sha256":hex(&view.fingerprint.content_sha256),
            "rgba_sha256":hash_pixels.then(||hex(&plx_base::sha256::sha256(view.rgba))),
            "width":view.width,"height":view.height,"texture_handle":texture.handle}),
        )
    }
    pub fn artwork_target_absent(&mut self, id: &str) -> bool {
        self.artwork
            .as_mut()
            .is_some_and(|r| r.source(id).is_none() && r.resident(id).is_none())
            && self.live_textures.texture(id).is_none()
            && !self.catalogue.applications.iter().any(|a| a.id == id)
    }
    pub fn artwork_catalogue_current(&self) -> bool {
        self.live_state
            .as_ref()
            .is_some_and(|f| Freshness::current(f.catalogue, self.live_now))
    }
    pub fn finish_live_artwork(&mut self) -> Option<serde_json::Value> {
        let mut runtime = self.artwork.take()?;
        let finished = runtime.finish();
        if finished.settlement == crate::artwork_worker::SessionSettlement::Unknown {
            self.artwork_pause.set(PauseReason::Quarantine, true);
        }
        for key in &finished.evicted {
            self.live_textures.evict(key);
        }
        self.live_textures.clear();
        if self.refresh_artwork_cards() {
            plx_machine::idle::invalidate();
        }
        let gl = self.artwork_gl.receipt();
        Some(
            serde_json::json!({"joined":finished.joined,"panicked":finished.panicked,
            "outstanding_resolved":finished.outstanding_resolved,"closed":true,"settlement":format!("{:?}",finished.settlement),
            "gpu_cleanup_qualified":gl["deletion_failures"]==0 && gl["observed_gl_errors"]==0,
            "remaining_live_textures":self.live_textures.stats().entries,
            "runtime":runtime.receipt(),"gl":gl}),
        )
    }
    fn moved(&mut self, outcome: plx_ui::focus::Outcome<u32>) {
        if let plx_ui::focus::Outcome::Moved { from, to, by } = outcome {
            plx_machine::idle::invalidate();
            if self.surface.page == Page::RangeEditor {
                self.input.abandon_if_off(entry(self.surface.page), to);
                return;
            }
            if let Some(index) =
                category_key(to.elem).filter(|_| self.surface.control_page.is_some())
            {
                if self.control_category != CATEGORIES[index].0
                    || (self.surface.page != Page::Controls && by != By::Restore)
                {
                    self.select_control_category(index);
                    return;
                }
            }
            self.input.abandon_if_off(entry(self.surface.page), to);
            let cx = self.context(&LegacyMeasure);
            let mut buf = Vec::new();
            let mut present = Present::new();
            let mut fx = Effects::new(&mut buf, MachineId::Input, &mut present);
            // Build an independent projection so the grid receives the move without borrowing Input.
            let read = Cx::<HomeHost> {
                views: (),
                tick: cx.tick,
                measure: &LegacyMeasure,
                press: cx.press,
                focus: cx.focus.clone(),
                owner: cx.owner,
            };
            if self.surface.page == Page::Home {
                self.surface.home_favourites.on(
                    &ScreenEvent::FocusMoved { from, to, by },
                    &read,
                    &self.surface.cards,
                    &mut fx,
                );
                self.surface.home_inputs.on(
                    &ScreenEvent::FocusMoved { from, to, by },
                    &read,
                    &self.surface.input_cards,
                    &mut fx,
                );
            } else {
                self.surface.grid.on(
                    &ScreenEvent::FocusMoved { from, to, by },
                    &read,
                    &self.surface.cards,
                    &mut fx,
                );
            }
        }
    }
    pub fn direction(&mut self, dir: Dir) {
        if self.drawer.system_keyboard() {
            self.input.cancel_press();
            return;
        }
        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return;
        }
        self.input.hit.note_dpad();
        if self.surface.page == Page::RangeEditor
            && self.focused_item().as_deref() == Some("range-value")
            && matches!(dir, Dir::Left | Dir::Right)
        {
            self.input.cancel_press();
            if let Some(r) = self.range.as_mut() {
                if r.state.nudge(if dir == Dir::Left {
                    RangeStep::Decrease
                } else {
                    RangeStep::Increase
                }) {
                    plx_machine::idle::invalidate();
                }
            }
            return;
        }
        let owner = InputOwner::Entry(entry(self.surface.page));
        let cx = Cx::<HomeHost> {
            views: (),
            tick: Tick {
                ms: self.now,
                dt_us: 16_667,
            },
            measure: &LegacyMeasure,
            press: PressRead::default(),
            focus: self.input.engine.read(owner),
            owner,
        };
        let links = if self.surface.control_page.is_some() {
            vec![
                Link {
                    from: GroupId(0),
                    dir: Dir::Right,
                    to: GroupId(6),
                },
                Link {
                    from: GroupId(6),
                    dir: Dir::Left,
                    to: GroupId(0),
                },
                Link {
                    from: GroupId(6),
                    dir: Dir::Right,
                    to: GroupId(1),
                },
                Link {
                    from: GroupId(1),
                    dir: Dir::Left,
                    to: GroupId(6),
                },
            ]
        } else if self.surface.page == Page::Home {
            vec![
                Link {
                    from: GroupId(1),
                    dir: Dir::Down,
                    to: GroupId(2),
                },
                Link {
                    from: GroupId(1),
                    dir: Dir::Up,
                    to: GroupId(7),
                },
                Link {
                    from: GroupId(2),
                    dir: Dir::Up,
                    to: GroupId(8),
                },
                Link {
                    from: GroupId(7),
                    dir: Dir::Down,
                    to: if self.surface.cards.items.is_empty() {
                        GroupId(8)
                    } else {
                        GroupId(1)
                    },
                },
                Link {
                    from: GroupId(8),
                    dir: Dir::Up,
                    to: if self.surface.cards.items.is_empty() {
                        GroupId(7)
                    } else {
                        GroupId(1)
                    },
                },
                Link {
                    from: GroupId(8),
                    dir: Dir::Down,
                    to: GroupId(2),
                },
                Link {
                    from: GroupId(7),
                    dir: Dir::Left,
                    to: GroupId(0),
                },
                Link {
                    from: GroupId(8),
                    dir: Dir::Left,
                    to: GroupId(0),
                },
            ]
        } else if self.surface.page == Page::Applications && !self.drawer.editing {
            vec![
                Link {
                    from: GroupId(1),
                    dir: Dir::Up,
                    to: GroupId(3),
                },
                Link {
                    from: GroupId(3),
                    dir: Dir::Down,
                    to: GroupId(1),
                },
            ]
        } else {
            Vec::new()
        };
        let outcome = self
            .input
            .engine
            .move_dir(owner, &self.surface, &links, dir, &cx);
        self.moved(outcome);
    }
    pub fn cancel_press(&mut self) {
        self.input.cancel_press();
    }
    pub fn repeat(&mut self) {
        self.input.note_alive(self.now);
    }
    pub fn press(&mut self, down: bool, from: PressFrom) {
        if self.drawer.system_keyboard() {
            self.input.cancel_press();
            return;
        }
        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return;
        }
        if down {
            if let Some(key) = self
                .input
                .engine
                .current(InputOwner::Entry(entry(self.surface.page)))
            {
                let card = self
                    .surface
                    .group_of(&key.elem, &self.context(&LegacyMeasure))
                    == Some(GroupId(1))
                    && !self.surface.table_mode;
                self.input.arm(
                    PressArm {
                        key,
                        from,
                        holdable: card && self.surface.page != Page::Inputs,
                    },
                    MachineId::Input,
                    self.now,
                );
            }
        } else {
            self.input.release(self.now);
        }
    }
    pub fn back(&mut self) {
        if self.artwork_ever_enabled {
            self.pending
                .retain(|a| crate::actions::Request::from_action(a.clone()).is_none());
        }
        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return;
        }
        self.input.cancel_press();
        if self.surface.page == Page::RangeEditor {
            self.close_range_editor();
            return;
        }
        if self.surface.page == Page::PointerSpeed {
            self.close_pointer_picker();
            return;
        }
        if self.surface.control_page.is_some() {
            let current = self
                .input
                .engine
                .current(InputOwner::Entry(entry(self.surface.page)))
                .map(|key| key.elem);
            if current.is_some_and(|key| {
                category_key(key).is_some() || (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(&key)
            }) {
                self.rebuild(Page::Home);
            } else {
                self.select_control_category(self.surface.selected_category());
            }
            return;
        }
        if self.menu.is_some() {
            self.close_app_menu();
            return;
        }
        if self.surface.page == Page::Applications && self.drawer.editing {
            self.drawer.finish_editing();
            self.rebuild(Page::Applications);
            self.focus_drawer_control(2);
            return;
        }
        if self
            .home_return
            .is_some_and(|(page, _)| page == self.surface.page)
        {
            let (_, index) = self.home_return.take().unwrap();
            self.rebuild(Page::Home);
            self.focus_home_link(index);
            return;
        }
        if matches!(
            self.surface.page,
            Page::Picture
                | Page::Sound
                | Page::Network
                | Page::Accessibility
                | Page::General
                | Page::PanelCare
        ) {
            self.rebuild(Page::Controls)
        } else if self.surface.page != Page::Home {
            self.rebuild(Page::Home)
        } else {
            self.pending.clear();
            self.exit = true
        }
    }
    pub fn pointer(&mut self, x: f32, y: f32, click: bool) {
        if self.drawer.system_keyboard() {
            self.input.cancel_press();
            return;
        }
        if self.surface.page == Page::RangeEditor
            && click
            && self.range.as_ref().is_some_and(|r| !r.pending)
            && (RANGE_FRAME.x..=RANGE_FRAME.x + RANGE_FRAME.w).contains(&x)
            && (RANGE_FRAME.y..=RANGE_FRAME.y + RANGE_FRAME.h).contains(&y)
        {
            self.input.cancel_press();
            self.focus_special(RANGE_BASE, GroupId(9));
            if let Some(r) = self.range.as_mut() {
                if let Some(value) = RangeLayout::new(RANGE_FRAME, r.state)
                    .and_then(|layout| layout.value_at(r.state.scale(), x))
                {
                    if r.state.stage(value) {
                        plx_machine::idle::invalidate();
                    }
                }
            }
            return;
        }

        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return;
        }
        if click {
            // A pointer release supersedes any keyboard gesture, even when
            // focus stays on the same card or the pointer misses every stop.
            self.input.cancel_press();
        }
        let owner = InputOwner::Entry(entry(self.surface.page));
        let kind = if click {
            plx_ui::hit::PointerKind::Click
        } else {
            plx_ui::hit::PointerKind::Move
        };
        let resolution = self.input.hit.resolve(
            Some(entry(self.surface.page)),
            kind,
            x,
            y,
            self.input.engine.current(owner),
        );
        if let Some(key) = resolution.focus {
            let Some(group) = self
                .surface
                .group_of(&key.elem, &self.context(&LegacyMeasure))
            else {
                // Hit stops belong to the previous painted frame. A live refresh, category
                // change or menu can remove that target before another event is polled.
                return;
            };
            let outcome = self.input.engine.set(owner, key, Some(group), By::Pointer);
            self.moved(outcome);
        }
        if click {
            if let Some((key, _)) = resolution.activate {
                self.activate(key, false);
            }
        }
    }
    fn activate(&mut self, key: FocusKey<u32>, hold: bool) {
        if self.exit {
            return;
        }
        if self.surface.page == Page::RangeEditor {
            if key.entry != entry(Page::RangeEditor) {
                return;
            }
            match range_key(key.elem) {
                Some(1) => {
                    if self.range.as_ref().is_some_and(|r| {
                        r.description
                            && self.description_value().map(|v| i64::from(v.value()))
                                != Some(r.state.saved())
                    }) {
                        self.close_range_editor();
                        self.status = cstr("Value changed or expired • reopen to adjust");
                        return;
                    }
                    if let Some(r) = self.range.as_mut().filter(|r| !r.pending) {
                        if let Some(proposal) = r.state.proposal() {
                            r.pending = true;
                            self.range_request = Some(proposal);
                            self.status = cstr("Applying value…");
                            plx_machine::idle::invalidate();
                        }
                    }
                }
                Some(2) => self.close_range_editor(),
                _ => {}
            }
            return;
        }
        if key.entry != entry(self.surface.page) {
            return;
        }
        if self.surface.menu.is_some() {
            let action = self.surface.menu.as_ref().and_then(|menu| {
                key.elem
                    .checked_sub(MENU_BASE)
                    .and_then(|k| menu.index_of_key(RowKey(k)))
                    .and_then(|i| menu.activate(i))
            });
            if let Some(plx_ui::form::Activation::Action(action)) = action {
                self.menu_action(action);
            }
            return;
        }
        if self.surface.page == Page::Applications {
            if let Some(index) = keyboard_key(key.elem).filter(|_| self.drawer.editing) {
                if self.drawer.key(index) {
                    self.drawer_scroll.jump(0.0);
                    self.drawer_scroll_target = 0.0;
                    self.rebuild(Page::Applications);
                    if !self.drawer.editing {
                        self.focus_drawer_control(2);
                    }
                }
                return;
            }
            if let Some(index) = toolbar_key(key.elem) {
                match index {
                    0 => self.drawer.favourites_only = false,
                    1 => self.drawer.favourites_only = true,
                    _ => {
                        self.drawer.editing = true;
                        self.drawer
                            .use_system_keyboard(index == 2 && self.system_keyboard_available);
                    }
                }
                self.drawer_scroll.jump(0.0);
                self.drawer_scroll_target = 0.0;
                self.rebuild(Page::Applications);
                if self.drawer.editing {
                    self.focus_search_key(0);
                }
                return;
            }
        }
        if let Some(index) = home_link_key(key.elem).filter(|_| self.surface.page == Page::Home) {
            if index == 0 {
                self.drawer = Drawer::default();
                self.drawer_scroll.jump(0.0);
                self.drawer_scroll_target = 0.0;
                self.home_return = Some((Page::Applications, index));
                self.rebuild(Page::Applications);
                if let Some(id) = self.surface.cards.items.first().map(|c| c.id.clone()) {
                    self.focus_application(&id);
                } else {
                    self.focus_drawer_control(0);
                }
            } else {
                self.home_return = Some((Page::Inputs, index));
                self.rebuild(Page::Inputs);
            }
            return;
        }
        if key.elem == LG_HOME_KEY {
            if self
                .surface
                .group_of(&key.elem, &self.context(&LegacyMeasure))
                != Some(GroupId(0))
            {
                return;
            }
            self.status = cstr(if self.home_launches_enabled {
                "Opening LG Home…"
            } else {
                "LG Home return is unavailable in this preview"
            });
            if self.home_launches_enabled {
                self.queue_external_action(Action::OpenLgHome);
            }
            return;
        }
        if (RAIL_BASE..RAIL_BASE + 4).contains(&key.elem) {
            self.home_return = None;
            self.rebuild(TABS[(key.elem - RAIL_BASE) as usize].0);
            return;
        }
        if let Some(index) = category_key(key.elem).filter(|_| self.surface.control_page.is_some())
        {
            self.rebuild(CATEGORIES[index].0);
            if let Some(key) = self.surface.keys().first().copied() {
                self.focus_special(key, GroupId(1));
            }
            return;
        }
        if self.surface.table_mode {
            if let Some(i) = key
                .elem
                .checked_sub(TABLE_BASE)
                .and_then(|k| self.surface.table.index_of_key(RowKey(k)))
            {
                match self.surface.table.activate(i) {
                    Some(plx_ui::form::Activation::Push(p)) => self.rebuild(p),
                    Some(plx_ui::form::Activation::Action(Action::OpenDescriptionLevel)) => {
                        self.open_description_editor();
                    }
                    Some(plx_ui::form::Activation::Action(Action::OpenPointerSpeed)) => {
                        self.open_pointer_picker();
                    }
                    Some(plx_ui::form::Activation::Action(Action::ClosePointerSpeed)) => {
                        self.close_pointer_picker();
                    }
                    Some(plx_ui::form::Activation::Action(Action::SetPointerSpeed(change))) => {
                        if self.surface.page == Page::PointerSpeed
                            && !self.pointer_pending
                            && self.pointer_expected == Some(change.expected)
                            && self.pointer_value() == Some(change.expected)
                        {
                            if change.value == change.expected {
                                self.close_pointer_picker();
                            } else {
                                self.pointer_pending = true;
                                self.status = cstr("Applying pointer speed…");
                                self.pending.push(Action::SetPointerSpeed(change));
                                self.rebuild(Page::PointerSpeed);
                            }
                        }
                    }
                    Some(plx_ui::form::Activation::Action(a)) => {
                        self.status = cstr("View only — use LG Settings for changes");
                        self.queue_external_action(a);
                    }
                    None => {}
                }
            }
            return;
        }
        let Some(card) = self
            .surface
            .cards
            .items
            .iter()
            .chain(self.surface.input_cards.items.iter())
            .find(|c| c.key == key.elem)
        else {
            return;
        };
        let id = card.id.clone();
        let input = input_key(&id).is_some();
        if hold && !input && self.surface.page == Page::Applications {
            let index = self
                .surface
                .cards
                .items
                .iter()
                .position(|c| c.key == key.elem)
                .unwrap_or(0);
            self.menu = Some(AppMenu {
                id,
                origin: key.elem,
                index,
            });
            self.rebuild(Page::Applications);
            self.focus_menu_action(MenuAction::ToggleFavourite);
        } else if hold && !input && self.surface.page == Page::Home {
            self.prefs.toggle_favourite(&id);
            self.preferences_changed = true;
            self.rebuild(self.surface.page);
        } else {
            if !self.request_is_current(input) {
                self.status = cstr("Reading unavailable • refresh before opening this item");
                return;
            }
            if input && !self.input_launches_enabled {
                self.status = cstr("Source switching is unavailable on this TV");
                return;
            }
            self.status = cstr(if input {
                "Opening source…"
            } else if self.app_launches_enabled {
                "Opening application…"
            } else {
                "Application launching is unavailable in this preview"
            });
            self.queue_external_action(if input {
                Action::RequestInput(id)
            } else {
                Action::RequestApplication(id)
            });
        }
    }
    fn select_control_category(&mut self, index: usize) {
        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return;
        }
        let Some((page, _)) = CATEGORIES.get(index) else {
            return;
        };
        self.control_category = *page;
        // Seed the shared engine's root entry before rebuilding, so a remembered old category
        // cannot overwrite this deliberate selection during reconciliation.
        self.input.engine.set(
            InputOwner::Entry(entry(Page::Controls)),
            FocusKey {
                entry: entry(Page::Controls),
                elem: CATEGORY_BASE + index as u32,
            },
            Some(GroupId(6)),
            By::Restore,
        );
        self.rebuild(Page::Controls);
    }
    pub fn focus_control_category(&mut self, index: usize) -> bool {
        if self.surface.page == Page::RangeEditor {
            return false;
        }
        if self.surface.control_page.is_none() || index >= 6 {
            return false;
        }
        self.select_control_category(index);
        true
    }
    pub fn control_category(&self) -> Option<Page> {
        self.surface.control_page
    }
    fn close_app_menu(&mut self) {
        let Some(menu) = self.menu.take() else {
            return;
        };
        self.rebuild(Page::Applications);
        let key = self
            .surface
            .cards
            .items
            .iter()
            .find(|c| c.key == menu.origin)
            .or_else(|| {
                self.surface.cards.items.get(
                    menu.index
                        .min(self.surface.cards.items.len().saturating_sub(1)),
                )
            })
            .map(|c| c.key);
        if let Some(key) = key {
            self.focus_special(key, GroupId(1));
        } else {
            self.focus_drawer_control(1);
        }
    }
    fn menu_action(&mut self, action: MenuAction) {
        let Some(id) = self.menu.as_ref().map(|menu| menu.id.clone()) else {
            return;
        };
        if !self.catalogue.applications.iter().any(|a| a.id == id) {
            self.close_app_menu();
            return;
        }
        match action {
            MenuAction::Close => {
                self.close_app_menu();
                return;
            }
            MenuAction::Open => {
                if self.app_launches_enabled && self.request_is_current(false) {
                    self.close_app_menu();
                    self.queue_external_action(Action::RequestApplication(id));
                    self.status = cstr("Opening application…");
                }
                return;
            }
            MenuAction::ToggleFavourite => {
                self.prefs.toggle_favourite(&id);
                self.preferences_changed = true;
            }
            MenuAction::Earlier | MenuAction::Later => {
                if !self
                    .prefs
                    .move_favourite(&id, action == MenuAction::Earlier, &self.catalogue)
                {
                    return;
                }
                self.preferences_changed = true;
            }
        }
        self.rebuild(Page::Applications);
        self.focus_menu_action(action);
    }
    pub fn focus_menu_action(&mut self, action: MenuAction) -> bool {
        if self.surface.menu.is_none() {
            return false;
        }
        self.focus_special(MENU_BASE + action.key(), GroupId(5));
        true
    }
    pub fn menu_application(&self) -> Option<&str> {
        self.menu.as_ref().map(|m| m.id.as_str())
    }
    /// Process queued input at this frame's controller time. Deferred gestures are
    /// advanced only after the batch, so queued cancellation can win the frame.
    pub fn input_frame<E>(
        &mut self,
        ms: u32,
        dt: f32,
        input: impl FnOnce(&mut Self) -> Result<(), E>,
    ) -> Result<(), E> {
        self.now = ms;
        input(self)?;
        if self.exit {
            self.input.cancel_press();
        } else {
            self.tick(ms, dt);
        }
        Ok(())
    }
    pub fn tick(&mut self, ms: u32, dt: f32) {
        self.now = ms;
        for event in self.input.tick(ms, dt) {
            match event {
                PressEvent::Commit(_, _, k) => self.activate(k, false),
                PressEvent::Hold(_, _, k) => self.activate(k, true),
            }
        }
        let owner = InputOwner::Entry(entry(self.surface.page));
        let cx = Cx::<HomeHost> {
            views: (),
            tick: Tick {
                ms,
                dt_us: (dt * 1e6) as u32,
            },
            measure: &LegacyMeasure,
            press: PressRead {
                scale: self.input.press.scale(),
                owner: self.input.dip_owner,
                ..Default::default()
            },
            focus: self.input.engine.read(owner),
            owner,
        };
        let mut buf = Vec::new();
        let mut present = Present::new();
        let mut fx = Effects::new(&mut buf, MachineId::Input, &mut present);
        if self.surface.page == Page::Home {
            self.surface.home_favourites.on(
                &ScreenEvent::Tick(cx.tick),
                &cx,
                &self.surface.cards,
                &mut fx,
            );
            self.surface.home_inputs.on(
                &ScreenEvent::Tick(cx.tick),
                &cx,
                &self.surface.input_cards,
                &mut fx,
            );
        } else {
            self.surface.grid.on(
                &ScreenEvent::Tick(cx.tick),
                &cx,
                &self.surface.cards,
                &mut fx,
            );
        }
        if self.surface.page == Page::Applications {
            let anchor = self.menu.as_ref().map(|m| m.origin).or_else(|| {
                cx.focus
                    .current
                    .filter(|k| self.surface.cards.index_of(&k.elem).is_some())
                    .map(|k| k.elem)
            });
            if let Some(key) = anchor {
                if let Some(placed) =
                    self.surface
                        .grid
                        .place(&cx, &self.surface.cards, &key, At::SpringTarget)
                {
                    let area = self.surface.grid_area();
                    let top = placed.rect.y;
                    let bottom = placed.rect.y + placed.rect.h + plx_ui::cards::UNDER_LABEL_H;
                    if top < area.y + 28.0 {
                        self.drawer_scroll_target =
                            (self.drawer_scroll.pos + top - area.y - 28.0).max(0.0);
                    } else if bottom > area.y + area.h - 8.0 {
                        self.drawer_scroll_target =
                            (self.drawer_scroll.pos + bottom - area.y - area.h + 8.0).max(0.0);
                    }
                }
            }
            let end = self.surface.cards.items.last().and_then(|card| {
                self.surface
                    .grid
                    .place(&cx, &self.surface.cards, &card.key, At::SpringTarget)
            });
            let max_scroll = end.map_or(0.0, |p| {
                let area = self.surface.grid_area();
                (self.drawer_scroll.pos + p.rect.y + p.rect.h + plx_ui::cards::UNDER_LABEL_H
                    - area.y
                    - area.h
                    + 8.0)
                    .max(0.0)
            });
            self.drawer_scroll_target = self.drawer_scroll_target.clamp(0.0, max_scroll);
            self.drawer_scroll
                .step(self.drawer_scroll_target, plx_ui::consts::K_SCROLL, dt);
            // Hand the externally positioned grid an exact resting value. Its
            // jump reports discrete changes, including invisible numeric tails.
            if plx_machine::idle::settled(
                self.drawer_scroll.pos,
                self.drawer_scroll_target,
                self.drawer_scroll.vel,
            ) {
                self.drawer_scroll.jump(self.drawer_scroll_target);
            }
            self.surface.grid.set_page(340.0, self.drawer_scroll.pos);
            if (self.drawer_scroll.pos - self.drawer_scroll_target).abs() > 0.25
                || self.drawer_scroll.vel.abs() > 0.5
            {
                fx.note(PresentEvent::Motion);
            }
        }
        let current = cx.focus.current.map(|k| k.elem);
        self.surface.rail.table.list_focused =
            current.is_some_and(|k| (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(&k));
        self.surface.rail.table.sel = current
            .filter(|k| (RAIL_BASE..RAIL_BASE + RAIL_COUNT).contains(k))
            .map(|k| (k - RAIL_BASE) as i32)
            .unwrap_or(
                TABS.iter()
                    .position(|(p, _)| *p == root_tab(self.surface.page))
                    .unwrap_or(0) as i32,
            );
        self.surface.table.table.list_focused =
            current.is_some_and(|k| (TABLE_BASE..TOOLBAR_BASE).contains(&k));
        if let Some(i) = current
            .and_then(|k| k.checked_sub(TABLE_BASE))
            .and_then(|k| self.surface.table.index_of_key(RowKey(k)))
        {
            self.surface.table.table.sel = i as i32;
        }
        if let Some(menu) = &mut self.surface.menu {
            menu.table.list_focused = true;
            if let Some(i) = current
                .and_then(|k| k.checked_sub(MENU_BASE))
                .and_then(|k| menu.index_of_key(RowKey(k)))
            {
                menu.table.sel = i as i32;
            }
            menu.table.update(dt, MENU_ROWS.h);
        }
        self.surface.rail.table.update(dt, RAIL.h);
        self.surface
            .table
            .table
            .update(dt, self.surface.table_area().h);
        // Input-batch cancellation and tick-derived activation finish before
        // intent reasons suppress dispatch; paused frames still poll results.
        self.reconcile_artwork_pause();
        self.pump_artwork();
        self.pump_packaged_icons();
    }
    /// Diagnostic geometry for an existing control binding; never grants activation.
    /// Values and app inventory are intentionally excluded.
    pub fn control_row_layout(&self, id: &str) -> Option<serde_json::Value> {
        self.surface.control_page?;
        let index = self.surface.table.index_of(&id.to_owned())?;
        let binding = self.surface.table.binding_at(index)?;
        let clip = self.surface.table_area();
        let rect = self.surface.table.table.row_frame(clip, index as i32)?;
        let focused = self
            .input
            .engine
            .current(InputOwner::Entry(entry(self.surface.page)))
            .is_some_and(|key| key.elem == TABLE_BASE + binding.key.0);
        Some(serde_json::json!({
            "id":id,"key":binding.key.0,"index":index,"focused":focused,
            "disabled":binding.disabled,"rect":[rect.x,rect.y,rect.w,rect.h],
            "clip":[clip.x,clip.y,clip.w,clip.h],"scroll":self.surface.table.table.scroll_pos(),
            "fully_visible":rect.x >= clip.x && rect.y >= clip.y && rect.x+rect.w <= clip.x+clip.w && rect.y+rect.h <= clip.y+clip.h
        }))
    }
    /// Presentation metadata only; no app inventory or TV values are recorded here.
    pub fn presentation_probe(&self) -> serde_json::Value {
        serde_json::json!({
            "page":format!("{:?}",self.surface.page),
            "range_editor":(self.surface.page==Page::RangeEditor).then(||serde_json::json!({"modal":true,"stops":(0..3).map(|i|{let r=range_rect(i);[r.x,r.y,r.w,r.h]}).collect::<Vec<_>>()})),
            "rail_rows":self.surface.rail.table.n_rows(),
            "rail_scroll":self.surface.rail.table.scroll_pos(),
            "rail_focused":self.surface.rail.table.list_focused,
            "rail_selection":self.surface.rail.table.sel,
            "drawer_scroll":self.drawer_scroll.pos,
            "drawer_anchor":self.drawer_focus_bounds().map(|(r,c)|serde_json::json!({"rect":[r.x,r.y,r.w,r.h],"clip":[c.x,c.y,c.w,c.h]})),
            "logical_clip_clear":plx_ui::screen::ClipScope::current().is_none(),
            "control_category":self.surface.control_page.map(|page| format!("{page:?}")),
            "category_rows":if self.surface.control_page.is_some(){6}else{0},
            "detail_scroll":self.surface.table.table.scroll_pos(),
            "clock":self.clock
            ,"branding_loaded":self.brand_texture!=0,"navigation_icons_loaded":self.navigation_textures.iter().all(|id|*id!=0)
        })
    }
    pub fn draw(&mut self) {
        let p = Painter::root();
        let ground = theme::SURFACE_OLED;
        plx_gfx::gfx::frame_clear(ground[0], ground[1], ground[2]);
        Label::new(
            c"Luma Home".as_ptr(),
            theme::size::DISPLAY,
            theme::TEXT_PRIMARY,
        )
        .bold()
        .draw(p, Rect::new(184.0, 60.0, 1150.0, 76.0));
        if self.brand_texture != 0 {
            p.tex(
                self.brand_texture,
                Rect::new(96.0, 68.0, 64.0, 64.0),
                0.0,
                [1.0; 4],
            );
        }
        Label::new(
            self.clock_time.as_ptr(),
            theme::size::DISPLAY,
            theme::TEXT_PRIMARY,
        )
        .h(HAlign::Right)
        .draw(p, Rect::new(1450.0, 60.0, 374.0, 76.0));
        Label::new(
            self.clock_source.as_ptr(),
            theme::size::CAPTION,
            theme::TEXT_SECONDARY,
        )
        .h(HAlign::Right)
        .draw(p, Rect::new(1334.0, 154.0, 490.0, 44.0));
        let subtitle = cstr(&format!(
            "{} • {}",
            root_tab(self.surface.page).title().unwrap_or("Home"),
            self.reading_label()
        ));
        Label::new(
            subtitle.as_ptr(),
            theme::size::CAPTION,
            theme::TEXT_SECONDARY,
        )
        .draw(p, Rect::new(96.0, 154.0, 1300.0, 44.0));
        p.rect(
            Rect::new(96.0, 214.0, 1728.0, 1.0),
            0.0,
            theme::HAIRLINE,
            theme::HAIRLINE,
            0.0,
        );
        let current = match self.surface.page {
            Page::Home => 0,
            Page::Applications => 1,
            Page::Inputs => 2,
            _ => 3,
        };
        let owner = InputOwner::Entry(entry(self.surface.page));
        let focused = self.input.engine.current(owner).map(|key| key.elem);
        for (i, title) in TABS
            .iter()
            .map(|(_, title)| *title)
            .chain(std::iter::once("LG Home"))
            .enumerate()
        {
            let rect = rail_rect(i);
            let active = focused == Some(RAIL_BASE + i as u32);
            let ink = if active {
                theme::INK_ON_ACCENT
            } else if i == current {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };
            if active || i == current {
                let fill = if active {
                    theme::TEXT_PRIMARY
                } else {
                    theme::SURFACE_PANEL
                };
                p.rect(rect, 18.0, fill, fill, 0.0);
            }
            Label::new(cstr(title).as_ptr(), theme::size::BODY, ink)
                .draw(p, Rect::new(rect.x + 80.0, rect.y, rect.w - 96.0, rect.h));
            if self.navigation_textures[if i == 4 { 0 } else { i }] != 0 {
                p.tex(
                    self.navigation_textures[if i == 4 { 0 } else { i }],
                    Rect::new(rect.x + 28.0, rect.cy() - 16.0, 32.0, 32.0),
                    0.0,
                    ink,
                );
            }
        }
        p.rect(
            Rect::new(120.0, 670.0, 238.0, 1.0),
            0.0,
            theme::HAIRLINE,
            theme::HAIRLINE,
            0.0,
        );
        let network = self.snapshot.settings.get("network");
        let connected = network.is_some_and(|rows| {
            rows.iter().any(|r| {
                matches!(r.id.as_str(), "wiredState" | "wifiState")
                    && r.available
                    && r.value == "Connected"
            })
        });
        let network_current = self
            .live_state
            .as_ref()
            .is_some_and(|fresh| Freshness::current(fresh.network, self.live_now));
        let connection = if !network_current {
            "TV status unavailable"
        } else if connected {
            "TV connected"
        } else {
            "TV disconnected"
        };
        Label::new(
            cstr(connection).as_ptr(),
            theme::size::CAPTION,
            theme::TEXT_SECONDARY,
        )
        .draw(p, Rect::new(120.0, 800.0, 280.0, 40.0));
        let sound = self
            .snapshot
            .settings
            .get("sound")
            .and_then(|rows| rows.iter().find(|r| r.id == "soundOutput" && r.available));
        let sound_current = self
            .live_state
            .as_ref()
            .is_some_and(|fresh| Freshness::current(fresh.sound, self.live_now));
        let output = if sound_current {
            sound
                .map(crate::model::display_value)
                .unwrap_or("Unavailable")
        } else {
            "Unavailable"
        };
        let sound_status = cstr(&plx_gfx::text::elide(
            &format!("Sound • {output}"),
            280.0,
            theme::size::CAPTION,
            0,
            false,
        ));
        Label::new(
            sound_status.as_ptr(),
            theme::size::CAPTION,
            theme::TEXT_SECONDARY,
        )
        .draw(p, Rect::new(120.0, 840.0, 280.0, 40.0));
        let cx = self.context(&LegacyMeasure);
        let mut f = DrawFrame::<HomeHost>::new(&cx, p);
        if self.surface.table_mode {
            if let Some(category) = self.surface.control_page {
                Label::new(
                    c"Control centre".as_ptr(),
                    theme::size::TITLE,
                    theme::TEXT_PRIMARY,
                )
                .bold()
                .draw(p, Rect::new(480.0, 244.0, 1344.0, 60.0));
                for i in 0..CATEGORIES.len() {
                    let rect = category_rect(i);
                    let active = focused == Some(CATEGORY_BASE + i as u32);
                    let selected = i == self.surface.selected_category();
                    if active || selected {
                        p.rrect(
                            rect,
                            18.0,
                            18.0,
                            if active {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::SURFACE_PANEL
                            },
                        );
                    }
                    let label = Label::new(
                        self.category_labels[i].as_ptr(),
                        theme::size::LABEL,
                        if active {
                            theme::INK_ON_ACCENT
                        } else if selected {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        },
                    );
                    let label = if active || selected {
                        label.bold()
                    } else {
                        label
                    };
                    label.draw(p, Rect::new(rect.x + 24.0, rect.y, rect.w - 48.0, rect.h));
                    f.stop(
                        p,
                        Stop {
                            key: FocusKey {
                                entry: entry(self.surface.page),
                                elem: CATEGORY_BASE + i as u32,
                            },
                            rect,
                            rest_rect: rect,
                            clip: CATEGORY_AREA,
                            hover: Hover::Focus,
                            activate: Activate::Press,
                        },
                    );
                }
                p.rrect(DETAIL_PANEL, 28.0, 28.0, theme::SURFACE_PANEL);
                Label::new(
                    cstr(if self.surface.page == Page::RangeEditor {
                        "Adjust value"
                    } else if self.surface.page == Page::PointerSpeed {
                        "Pointer speed"
                    } else {
                        category.title().unwrap_or("Settings")
                    })
                    .as_ptr(),
                    theme::size::HEADLINE,
                    theme::TEXT_PRIMARY,
                )
                .bold()
                .draw(p, Rect::new(838.0, 352.0, 946.0, 58.0));
                let at = Rect::new(838.0, 410.0, 946.0, 44.0);
                Label::new(
                    self.control_breadcrumb.as_ptr(),
                    theme::size::CAPTION,
                    theme::TEXT_SECONDARY,
                )
                .draw(p.clipped(at), at);
            }
            if let Some(r) = self
                .range
                .as_ref()
                .filter(|_| self.surface.page == Page::RangeEditor)
            {
                RangeControl {
                    frame: RANGE_FRAME,
                    state: r.state,
                    label: &r.label,
                    focused: focused == Some(RANGE_BASE),
                    enabled: !r.pending,
                }
                .draw(&Env::inert(), p);
                for (i, text) in [(1, "Apply"), (2, "Cancel")] {
                    let enabled = !r.pending && (i != 1 || r.state.dirty());
                    TabPill::new(cstr(text).as_ptr(), theme::size::BODY, range_rect(i))
                        .focused(focused == Some(RANGE_BASE + i as u32))
                        .draw(&Env::inert(), if enabled { p } else { p.alpha(0.4) });
                }
                for i in 0..3 {
                    let rect = range_rect(i);
                    f.stop(
                        p,
                        Stop {
                            key: FocusKey {
                                entry: entry(self.surface.page),
                                elem: RANGE_BASE + i as u32,
                            },
                            rect,
                            rest_rect: rect,
                            clip: DETAIL_ROWS,
                            hover: Hover::Focus,
                            activate: Activate::Press,
                        },
                    );
                }
            }
            if self.surface.page != Page::RangeEditor {
                let area = self.surface.table_area();
                let _clip = f.clip(p, area);
                self.surface
                    .table
                    .table
                    .draw_page(p.clipped(area), area, &LegacyMeasure);
            }
        } else if self.surface.page == Page::Home {
            Label::new(
                c"Your favourites".as_ptr(),
                theme::size::HEADLINE,
                theme::TEXT_PRIMARY,
            )
            .bold()
            .draw(p, Rect::new(480.0, 244.0, 1344.0, 48.0));
            let source_heading = if self
                .live_state
                .as_ref()
                .is_some_and(|fresh| !Freshness::current(fresh.inputs, self.live_now))
            {
                c"Inputs • status unavailable"
            } else {
                c"Inputs"
            };
            Label::new(
                source_heading.as_ptr(),
                theme::size::HEADLINE,
                theme::TEXT_PRIMARY,
            )
            .bold()
            .draw(p, Rect::new(480.0, 652.0, 1344.0, 48.0));
            if self.surface.cards.items.is_empty() {
                Label::new(
                    c"Hold OK on an app to add a favourite".as_ptr(),
                    theme::size::BODY,
                    theme::TEXT_SECONDARY,
                )
                .draw(p, Rect::new(480.0, 366.0, 1344.0, 100.0));
            }
            {
                let _clip = f.clip(p, HOME_FAVOURITES.clip);
                self.surface.home_favourites.draw(
                    &mut f,
                    p.clipped(HOME_FAVOURITES.clip),
                    &self.surface.cards,
                    HOME_FAVOURITES,
                );
            }
            {
                let _clip = f.clip(p, HOME_INPUTS.clip);
                self.surface.home_inputs.draw(
                    &mut f,
                    p.clipped(HOME_INPUTS.clip),
                    &self.surface.input_cards,
                    HOME_INPUTS,
                );
            }
        } else {
            let content = if self.surface.page == Page::Applications {
                self.surface.grid_area()
            } else if self.surface.editing {
                Rect::new(CONTENT.x, 330.0, CONTENT.w, 320.0)
            } else if self.surface.menu.is_some() || self.surface.page == Page::Inputs {
                Rect::new(CONTENT.x, CONTENT.y, 884.0, CONTENT.h)
            } else {
                CONTENT
            };
            let _clip = f.clip(p, content);
            self.surface
                .grid
                .draw(&mut f, p.clipped(content), &self.surface.cards);
        }
        if self.surface.page == Page::Home {
            for (i, title) in ["View all apps ›", "View all inputs ›"]
                .into_iter()
                .enumerate()
            {
                let r = home_link_rect(i);
                let key = if i == 0 {
                    HOME_APPS_LINK
                } else {
                    HOME_INPUTS_LINK
                };
                TabPill::new(cstr(title).as_ptr(), theme::size::CAPTION, r)
                    .focused(focused == Some(key))
                    .draw(&Env::inert(), p);
                f.stop(
                    p,
                    Stop {
                        key: FocusKey {
                            entry: entry(Page::Home),
                            elem: key,
                        },
                        rect: r,
                        rest_rect: r,
                        clip: r,
                        hover: Hover::Focus,
                        activate: Activate::Press,
                    },
                );
            }
        }
        if self.surface.page == Page::Inputs {
            Label::new(
                c"Choose a source".as_ptr(),
                theme::size::TITLE,
                theme::TEXT_PRIMARY,
            )
            .bold()
            .draw(p, Rect::new(480.0, 244.0, 1344.0, 60.0));
            p.rrect(INPUT_DETAIL, 28.0, 28.0, theme::SURFACE_PANEL);
            let input = focused
                .and_then(|key| {
                    self.snapshot
                        .inputs
                        .iter()
                        .find(|input| input_key(&input.id) == Some(key))
                })
                .or_else(|| self.snapshot.inputs.first());
            if let Some(input) = input {
                let current = self
                    .live_state
                    .as_ref()
                    .is_some_and(|f| Freshness::current(f.inputs, self.live_now));
                let title = cstr(&plx_gfx::text::elide(
                    &input.label,
                    418.0,
                    theme::size::HEADLINE,
                    1,
                    false,
                ));
                Label::new(title.as_ptr(), theme::size::HEADLINE, theme::TEXT_PRIMARY)
                    .bold()
                    .draw(p, Rect::new(1366.0, 354.0, 418.0, 60.0));
                for (i, (label, value)) in [
                    ("Input", input.id.replace('_', " ")),
                    (
                        "Cable",
                        if !current {
                            "Unavailable"
                        } else if input.plugged {
                            "Connected"
                        } else {
                            "Not connected"
                        }
                        .into(),
                    ),
                    (
                        "Signal",
                        if !current {
                            "Unavailable"
                        } else if input.signal {
                            "Detected"
                        } else {
                            "Not detected"
                        }
                        .into(),
                    ),
                    (
                        "Selected input",
                        match input.selected.filter(|_| current) {
                            Some(true) => "Yes",
                            Some(false) => "No",
                            None => "Unavailable",
                        }
                        .into(),
                    ),
                    ("Video format", "Unavailable".into()),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = 422.0 + i as f32 * 86.0;
                    Label::new(
                        cstr(label).as_ptr(),
                        theme::size::CAPTION,
                        theme::TEXT_SECONDARY,
                    )
                    .draw(p, Rect::new(1366.0, y, 418.0, 34.0));
                    Label::new(
                        cstr(&value).as_ptr(),
                        theme::size::LABEL,
                        theme::TEXT_PRIMARY,
                    )
                    .draw(
                        p.clipped(INPUT_DETAIL),
                        Rect::new(1366.0, y + 32.0, 418.0, 40.0),
                    );
                }
            }
        }
        for i in 0..RAIL_COUNT {
            {
                let r = rail_rect(i as usize);
                f.stop(
                    p,
                    Stop {
                        key: FocusKey {
                            entry: entry(self.surface.page),
                            elem: RAIL_BASE + i,
                        },
                        rect: r,
                        rest_rect: r,
                        clip: RAIL,
                        hover: Hover::Focus,
                        activate: Activate::Press,
                    },
                );
            }
        }
        if self.surface.table_mode {
            for i in 0..self.surface.table.table.n_rows() {
                if let Some(key) = self.surface.table.key_at(i as usize) {
                    if let Some(r) = self
                        .surface
                        .table
                        .table
                        .row_frame(self.surface.table_area(), i)
                    {
                        f.stop(
                            p,
                            Stop {
                                key: FocusKey {
                                    entry: entry(self.surface.page),
                                    elem: TABLE_BASE + key.0,
                                },
                                rect: r,
                                rest_rect: r,
                                clip: self.surface.table_area(),
                                hover: Hover::Focus,
                                activate: Activate::Press,
                            },
                        );
                    }
                }
            }
        }
        if self.surface.page == Page::Applications {
            let focused = cx.focus.current.map(|k| k.elem);
            for (i, title) in [
                "All apps".to_owned(),
                "Favourites".to_owned(),
                if self.drawer.query().is_empty() {
                    "Search apps…".into()
                } else {
                    format!(
                        "Search: {}{}",
                        self.drawer.query(),
                        self.drawer.composition()
                    )
                },
                "Keypad".to_owned(),
            ]
            .iter()
            .enumerate()
            {
                let label = cstr(&plx_gfx::text::elide(
                    title,
                    toolbar_rect(i).w - 44.0,
                    theme::size::LABEL,
                    1,
                    false,
                ));
                if i == 2 && self.drawer.editing {
                    let (before, preedit, after) = self.drawer.display_parts();
                    TextEntry {
                        frame: toolbar_rect(i),
                        before: &before,
                        preedit: &preedit,
                        after: &after,
                        placeholder: "Search apps…",
                        focused: true,
                    }
                    .draw(&Env::inert(), p);
                } else {
                    TabPill::new(label.as_ptr(), theme::size::LABEL, toolbar_rect(i))
                        .selected(i < 2 && (i == 1) == self.drawer.favourites_only)
                        .focused(focused == Some(TOOLBAR_BASE + i as u32))
                        .draw(&Env::inert(), p);
                }
                if !self.drawer.editing {
                    f.stop(
                        p,
                        Stop {
                            key: FocusKey {
                                entry: entry(self.surface.page),
                                elem: TOOLBAR_BASE + i as u32,
                            },
                            rect: toolbar_rect(i),
                            rest_rect: toolbar_rect(i),
                            clip: TOOLBAR,
                            hover: Hover::Focus,
                            activate: Activate::Press,
                        },
                    );
                }
            }
            if let Some((title, hint)) = self.drawer_empty_copy() {
                Label::new(title.as_ptr(), theme::size::BODY, theme::TEXT_PRIMARY)
                    .draw(p, Rect::new(480.0, 390.0, 1344.0, 52.0));
                Label::new(hint.as_ptr(), theme::size::CAPTION, theme::TEXT_SECONDARY)
                    .draw(p, Rect::new(480.0, 448.0, 1344.0, 72.0));
            }
            if self.drawer.editing && !self.drawer.system_keyboard() {
                for (i, label) in KEYS.iter().enumerate() {
                    TabPill::new(cstr(label).as_ptr(), theme::size::CAPTION, keyboard_rect(i))
                        .focused(focused == Some(KEYBOARD_BASE + i as u32))
                        .draw(&Env::inert(), p);
                    f.stop(
                        p,
                        Stop {
                            key: FocusKey {
                                entry: entry(self.surface.page),
                                elem: KEYBOARD_BASE + i as u32,
                            },
                            rect: keyboard_rect(i),
                            rest_rect: keyboard_rect(i),
                            clip: KEYBOARD,
                            hover: Hover::Focus,
                            activate: Activate::Press,
                        },
                    );
                }
            }
        }
        if let (Some(menu), Some(state)) = (&self.surface.menu, &self.menu) {
            p.rrect(MENU_PANEL, 28.0, 28.0, theme::SURFACE_PANEL);
            let title = self
                .catalogue
                .applications
                .iter()
                .find(|a| a.id == state.id)
                .map(|a| a.title.as_str())
                .unwrap_or("App unavailable");
            let title = cstr(&plx_gfx::text::elide(
                title,
                408.0,
                theme::size::HEADLINE,
                1,
                false,
            ));
            Label::new(title.as_ptr(), theme::size::HEADLINE, theme::TEXT_PRIMARY)
                .bold()
                .draw(p, Rect::new(1388.0, 352.0, 408.0, 58.0));
            menu.table.draw_page(p, MENU_ROWS, &LegacyMeasure);
            for i in 0..menu.table.n_rows() {
                if let (Some(key), Some(rect)) =
                    (menu.key_at(i as usize), menu.table.row_frame(MENU_ROWS, i))
                {
                    f.stop(
                        p,
                        Stop {
                            key: FocusKey {
                                entry: entry(self.surface.page),
                                elem: MENU_BASE + key.0,
                            },
                            rect,
                            rest_rect: rect,
                            clip: MENU_ROWS,
                            hover: Hover::Focus,
                            activate: Activate::Press,
                        },
                    );
                }
            }
        }
        let mut stops = f.into_stops();
        if self.surface.menu.is_some() {
            stops.retain(|stop| (MENU_BASE..MENU_BASE + 5).contains(&stop.key.elem));
        }
        if self.drawer.system_keyboard() {
            stops.clear();
        } else if self.surface.editing {
            stops.retain(|stop| keyboard_key(stop.key.elem).is_some());
        }
        self.input.hit.fill(stops);
        self.input.hit.swap();
        let status = if self.menu.is_some() {
            c"App options • ordering affects Luma favourites only • Back to close"
        } else if self.drawer.system_keyboard() {
            c"Use the TV keyboard • Back to finish search"
        } else if self.drawer.editing && self.surface.page == Page::Applications {
            c"OK to type • Back to finish search"
        } else {
            self.status.as_c_str()
        };
        Label::new(status.as_ptr(), theme::size::CAPTION, theme::TEXT_TERTIARY)
            .draw(p, Rect::new(96.0, 985.0, 1728.0, 44.0));
    }
    pub fn last_application_id(&self) -> Option<&str> {
        if self.surface.page == Page::Applications {
            self.surface.cards.items.last().map(|c| c.id.as_str())
        } else {
            None
        }
    }
    pub fn focus_home_link(&mut self, index: usize) -> bool {
        if self.surface.page != Page::Home || index > 1 {
            return false;
        }
        self.focus_special(
            if index == 0 {
                HOME_APPS_LINK
            } else {
                HOME_INPUTS_LINK
            },
            GroupId(if index == 0 { 7 } else { 8 }),
        );
        true
    }
    pub fn drawer_focus_bounds(&self) -> Option<(Rect, Rect)> {
        if self.surface.page != Page::Applications {
            return None;
        }
        let key = self.menu.as_ref().map(|m| m.origin).or_else(|| {
            self.input
                .engine
                .current(InputOwner::Entry(entry(self.surface.page)))
                .map(|k| k.elem)
        })?;
        let placed = self.surface.grid.place(
            &self.context(&LegacyMeasure),
            &self.surface.cards,
            &key,
            At::SpringTarget,
        )?;
        Some((
            Rect::new(
                placed.rect.x,
                placed.rect.y,
                placed.rect.w,
                placed.rect.h + plx_ui::cards::UNDER_LABEL_H,
            ),
            self.surface.grid_area(),
        ))
    }
    /// Current shared card placement for isolated native presentation receipts.
    /// No layout, focus, image ownership or activation is changed by this read.
    pub fn icon_presentation_cards(&self) -> Vec<serde_json::Value> {
        if !matches!(self.surface.page, Page::Home | Page::Applications) {
            return Vec::new();
        }
        let cx = self.context(&LegacyMeasure);
        let focus = self
            .input
            .engine
            .current(InputOwner::Entry(entry(self.surface.page)));
        let rect = |r: Rect| [r.x, r.y, r.w, r.h];
        self.surface
            .cards
            .items
            .iter()
            .filter_map(|card| {
                let (placed, clip) = if self.surface.page == Page::Home {
                    (
                        self.surface.home_favourites.place(
                            &cx,
                            &self.surface.cards,
                            &card.key,
                            HOME_FAVOURITES,
                            At::Drawn,
                        )?,
                        HOME_FAVOURITES.clip,
                    )
                } else {
                    (
                        self.surface
                            .grid
                            .place(&cx, &self.surface.cards, &card.key, At::Drawn)?,
                        self.surface.grid_area(),
                    )
                };
                let (handle, width, height) = if card.has_icon() {
                    card.texture
                } else {
                    (0, 0.0, 0.0)
                };
                Some(
                    serde_json::json!({"id":card.id,"title":card.title.to_string_lossy(),
                "rect":rect(placed.rect),"clip":rect(clip),
                "focused":focus.is_some_and(|key| key.elem == card.key),
                "width":width as u32,"height":height as u32,"texture_handle":handle}),
                )
            })
            .collect()
    }
    /// Session-proof observation only. The Main caller gates this by actual
    /// ordinary Preview + session capture, after draw and before swap.
    #[doc(hidden)]
    pub fn session_cache_frame(&self) -> Option<(Page, serde_json::Value)> {
        let controller = self.packaged_icons.as_ref()?;
        self.session_cache_frame_with(controller.stats(), |id| {
            Some((
                controller.texture(id)?,
                controller.current_cached_source(id)?.clone(),
            ))
        })
    }
    fn session_cache_frame_with(
        &self,
        stats: crate::packaged_icons::Stats,
        origin: impl Fn(
            &str,
        ) -> Option<(
            crate::packaged_icons::TextureView<u32>,
            crate::app_artwork::SourceKey,
        )>,
    ) -> Option<(Page, serde_json::Value)> {
        if !self.current_icon_cache
            || !matches!(self.surface.page, Page::Home | Page::Applications)
            || !self.ordinary_icons_allowed()
            || self.drawer.editing
            || self.menu.is_some()
            || !Freshness::current(self.current_icon_sources_observed, self.live_now)
        {
            return None;
        }
        let sources = self.current_icon_sources.as_ref()?;
        let observed = self.current_icon_sources_observed?;
        let visible = self.packaged_visible().into_iter().collect::<BTreeSet<_>>();
        let mut cached = Vec::new();
        for row in self.icon_presentation_cards() {
            let id = row["id"].as_str()?;
            if !visible.contains(id) {
                continue;
            }
            let Some((view, key)) = origin(id) else {
                continue;
            };
            if view.handle == 0
                || sources.get(id).and_then(|s| s.preferred()).map(|s| s.key()) != Some(&key)
                || row["texture_handle"].as_u64() != Some(u64::from(view.handle))
                || row["width"].as_u64() != Some(u64::from(view.width))
                || row["height"].as_u64() != Some(u64::from(view.height))
            {
                continue;
            }
            // Existing shared placement is unchanged; intersect actual drawn
            // clip once more, never spring-target/offscreen eligibility.
            let numbers = |value: &serde_json::Value| -> Option<[f64; 4]> {
                let a = value.as_array()?;
                if a.len() != 4 {
                    return None;
                }
                let out = [
                    a[0].as_f64()?,
                    a[1].as_f64()?,
                    a[2].as_f64()?,
                    a[3].as_f64()?,
                ];
                out.iter().all(|v| v.is_finite()).then_some(out)
            };
            let r = numbers(&row["rect"])?;
            let c = numbers(&row["clip"])?;
            if r[2] <= 0.0
                || r[3] <= 0.0
                || c[2] <= 0.0
                || c[3] <= 0.0
                || r[0] >= c[0] + c[2]
                || r[0] + r[2] <= c[0]
                || r[1] >= c[1] + c[3]
                || r[1] + r[3] <= c[1]
            {
                continue;
            }
            cached.push(serde_json::json!({"id":id,"source_key":key,"rect":row["rect"],"clip":row["clip"],
                "focused":row["focused"],"texture_handle":view.handle,"width":view.width,"height":view.height}));
            if cached.len() > crate::packaged_icons::MAX_TEXTURES {
                return None;
            }
        }
        if cached.is_empty() || cached.len() > stats.texture_entries {
            return None;
        }
        let page = self.surface.page;
        Some((
            page,
            serde_json::json!({"schema":1,"kind":"ordinary-cache-frame-v1",
            "page":if page==Page::Home {"home"} else {"apps"},"source_observed_ms":observed,
            "source_sample_now_ms":self.live_now,"source_age_ms":self.live_now.wrapping_sub(observed),
            "cached_cards":cached,"cache_readiness":format!("{:?}",stats.cache_readiness),
            "cache_readiness_is_index_not_images":true,"cache_uploads":stats.cache_uploads,
            "packaged_uploads":stats.packaged_uploads,"cache_adoptions":stats.cache_adoptions,
            "texture_entries":stats.texture_entries,"Root_authority_qualified":false,
            "positive_Root_producer_qualified":false,"native_foreground_scene_qualified":false,
            "physical_panel_qualified":false,"cleanup_qualified":false,"performance_qualified":false,
            "continuous_source_file_freshness_qualified":false}),
        ))
    }
    pub fn icon_presentation_resources(&self) -> Vec<u32> {
        std::iter::once(self.brand_texture)
            .chain(self.navigation_textures)
            .chain(
                self.surface
                    .cards
                    .items
                    .iter()
                    .filter(|card| card.has_icon())
                    .map(|card| card.texture.0),
            )
            .collect()
    }
    pub fn focus_drawer_control(&mut self, index: usize) -> bool {
        if self.surface.page != Page::Applications
            || self.drawer.editing
            || self.menu.is_some()
            || index >= TOOLBAR_COUNT
        {
            return false;
        }
        self.focus_special(TOOLBAR_BASE + index as u32, GroupId(3));
        true
    }
    pub fn focus_search_key(&mut self, index: usize) -> bool {
        if self.surface.page != Page::Applications
            || !self.drawer.editing
            || self.drawer.system_keyboard()
            || index >= KEYS.len()
        {
            return false;
        }
        self.focus_special(KEYBOARD_BASE + index as u32, GroupId(4));
        true
    }
    fn focus_special(&mut self, key: u32, group: GroupId) {
        let owner = InputOwner::Entry(entry(self.surface.page));
        let outcome = self.input.engine.set(
            owner,
            FocusKey {
                entry: entry(self.surface.page),
                elem: key,
            },
            Some(group),
            By::Restore,
        );
        self.moved(outcome);
    }
    pub fn first_application(&self) -> Option<String> {
        self.surface.cards.items.first().map(|c| c.id.clone())
    }
    pub fn application_count(&self) -> usize {
        self.surface.cards.len()
    }
    /// Effective targets in the actual front hit map, with current input-owner/mode gates.
    /// This is registration/admission evidence, not a claim about panel presentation.
    pub fn effective_fallback_targets(&self) -> usize {
        if self.page() != Page::Applications
            || !self.drawer.editing
            || self.drawer.system_keyboard()
            || self.input.hit.suppressed
        {
            return 0;
        }
        self.input
            .hit
            .front()
            .iter()
            .filter(|stop| {
                let visible = stop.rect.intersect(stop.clip);
                stop.key.entry == entry(Page::Applications)
                    && keyboard_key(stop.key.elem).is_some()
                    && visible.w > 0.0
                    && visible.h > 0.0
            })
            .count()
    }
    pub fn system_keyboard_available(&self) -> bool {
        self.system_keyboard_available
    }
    pub fn use_system_keyboard(&mut self, active: bool) {
        self.input.cancel_press();
        self.drawer
            .use_system_keyboard(active && self.surface.page == Page::Applications);
        plx_machine::idle::invalidate();
    }
    pub fn text_input(&mut self, text: &str, prediction: bool) -> bool {
        if self.surface.page != Page::Applications || !self.drawer.editing || self.menu.is_some() {
            return false;
        }
        self.input.cancel_press();
        let changed = self.drawer.commit(text, prediction);
        if changed {
            self.drawer_scroll.jump(0.0);
            self.drawer_scroll_target = 0.0;
            self.rebuild(Page::Applications);
        }
        changed
    }
    pub fn text_preedit(&mut self, text: &str) -> bool {
        if self.surface.page != Page::Applications || !self.drawer.editing || self.menu.is_some() {
            return false;
        }
        self.input.cancel_press();
        let changed = self.drawer.preedit(text);
        if changed {
            plx_machine::idle::invalidate();
        }
        changed
    }
    pub fn text_delete_forward(&mut self) -> bool {
        if self.surface.page != Page::Applications || !self.drawer.editing || self.menu.is_some() {
            return false;
        }
        self.input.cancel_press();
        let changed = self.drawer.delete_forward();
        if changed {
            self.rebuild(Page::Applications);
        }
        changed
    }
    pub fn text_edit(&mut self, edit: &plx_machine::machine::TextEdit) -> bool {
        if self.surface.page != Page::Applications || !self.drawer.editing || self.menu.is_some() {
            return false;
        }
        self.input.cancel_press();
        let changed = self.drawer.edit(edit);
        if changed {
            self.drawer_scroll.jump(0.0);
            self.drawer_scroll_target = 0.0;
            self.rebuild(Page::Applications);
        }
        changed
    }
    pub fn drawer_state(&self) -> &Drawer {
        &self.drawer
    }
    /// Used by the app picker/search and by identity-based verification, never a private cursor.
    pub fn focus_application(&mut self, id: &str) -> bool {
        if self.surface.editing
            || self.menu.is_some()
            || !matches!(self.surface.page, Page::Home | Page::Applications)
        {
            return false;
        }
        let Some(card) = self.surface.cards.items.iter().find(|c| c.id == id) else {
            return false;
        };
        let owner = InputOwner::Entry(entry(self.surface.page));
        let outcome = self.input.engine.set(
            owner,
            FocusKey {
                entry: entry(self.surface.page),
                elem: card.key,
            },
            Some(GroupId(1)),
            By::Restore,
        );
        self.moved(outcome);
        true
    }
    pub fn focus_input(&mut self, id: &str) -> bool {
        if !matches!(self.surface.page, Page::Home | Page::Inputs) {
            return false;
        }
        let Some(card) = self
            .surface
            .cards
            .items
            .iter()
            .chain(self.surface.input_cards.items.iter())
            .find(|c| c.id == id)
        else {
            return false;
        };
        let owner = InputOwner::Entry(entry(self.surface.page));
        let outcome = self.input.engine.set(
            owner,
            FocusKey {
                entry: entry(self.surface.page),
                elem: card.key,
            },
            Some(GroupId(if self.surface.page == Page::Home {
                2
            } else {
                1
            })),
            By::Restore,
        );
        self.moved(outcome);
        true
    }
    pub fn focus_lg_settings(&mut self) -> bool {
        let Some(section) = self
            .surface
            .control_page
            .and_then(SettingsSection::from_page)
        else {
            return false;
        };
        if self
            .surface
            .table
            .index_of_key(RowKey(section.focus_key()))
            .is_none()
        {
            return false;
        }
        let owner = InputOwner::Entry(entry(self.surface.page));
        let key = FocusKey {
            entry: entry(self.surface.page),
            elem: TABLE_BASE + section.focus_key(),
        };
        let outcome = self
            .input
            .engine
            .set(owner, key, Some(GroupId(1)), By::Restore);
        self.moved(outcome);
        true
    }
    pub fn focused_item(&self) -> Option<String> {
        let key = self
            .input
            .engine
            .current(InputOwner::Entry(entry(self.surface.page)))?;
        if self.surface.page == Page::RangeEditor {
            return range_key(key.elem)
                .map(|i| ["range-value", "range-apply", "range-cancel"][i].into());
        }

        if let Some(index) = category_key(key.elem).filter(|_| self.surface.control_page.is_some())
        {
            return Some(format!("category-{}", CATEGORIES[index].1));
        }
        if self.menu.is_some() {
            return key
                .elem
                .checked_sub(MENU_BASE)
                .map(|i| format!("app-option-{i}"));
        }
        if self.surface.page == Page::Applications {
            if let Some(i) = keyboard_key(key.elem) {
                return Some(format!("search-key-{}", KEYS[i]));
            }
            if let Some(i) = toolbar_key(key.elem) {
                return Some(["All apps", "Favourites", "Search apps", "Search keypad"][i].into());
            }
        }
        if self.surface.page == Page::Home && key.elem == HOME_APPS_LINK {
            Some("View all apps".into())
        } else if self.surface.page == Page::Home && key.elem == HOME_INPUTS_LINK {
            Some("View all inputs".into())
        } else if key.elem == LG_HOME_KEY {
            Some("LG Home".into())
        } else if (RAIL_BASE..RAIL_BASE + 4).contains(&key.elem) {
            Some(TABS[(key.elem - RAIL_BASE) as usize].1.to_owned())
        } else if self.surface.table_mode {
            self.surface
                .table
                .index_of_key(RowKey(key.elem.checked_sub(TABLE_BASE)?))
                .map(|i| format!("control-row-{i}"))
        } else {
            self.surface
                .cards
                .items
                .iter()
                .chain(self.surface.input_cards.items.iter())
                .find(|c| c.key == key.elem)
                .map(|c| c.id.clone())
        }
    }
    pub fn request_exit(&mut self) {
        self.exit = true;
        self.cancel_press();
        self.set_artwork_pause(PauseReason::ForegroundLost, true);
    }
    pub fn should_exit(&self) -> bool {
        self.exit
    }
    pub fn page(&self) -> Page {
        self.surface.page
    }
    pub fn preferences(&self) -> &Preferences {
        &self.prefs
    }
    pub fn storage_notice(&mut self, message: &str) {
        if self.status.to_str().ok() != Some(message) {
            plx_machine::idle::invalidate();
        }
        self.status = cstr(message);
    }
    pub fn take_preferences_changed(&mut self) -> bool {
        std::mem::take(&mut self.preferences_changed)
    }
    fn queue_external_action(&mut self, action: Action) {
        if self.artwork_ever_enabled {
            if let Some(request) = crate::actions::Request::from_action(action.clone()) {
                // Capture origin at activation, not after a later input in the
                // same batch moves focus/page. Only the latest typed intent wins.
                self.artwork_queued_anchor = Some((request, self.artwork_intent_anchor()));
            }
        }
        self.pending.push(action);
    }
    pub fn take_transition_requests(
        &mut self,
    ) -> (
        Vec<Action>,
        Option<(crate::actions::Request, ArtworkAnchor)>,
    ) {
        (
            std::mem::take(&mut self.pending),
            self.artwork_queued_anchor.take(),
        )
    }
    pub fn take_requests(&mut self) -> Vec<Action> {
        self.artwork_queued_anchor = None;
        std::mem::take(&mut self.pending)
    }
    pub fn description_value(&self) -> Option<crate::description_level::Level> {
        self.description_sample
            .filter(|(stamp, _)| self.live_now.wrapping_sub(*stamp) <= 8_000)
            .map(|(_, value)| value)
    }
    pub fn description_sample(
        &mut self,
        value: Option<crate::description_level::Level>,
        started: u32,
        now: u32,
    ) {
        self.live_now = now;
        if self
            .description_confirmation
            .is_some_and(|(stamp, _)| started <= stamp)
        {
            return;
        }
        self.description_confirmation = None;
        let before = self.description_sample.map(|(_, value)| value);
        self.description_sample = value
            .filter(|_| now.wrapping_sub(started) <= 8_000)
            .map(|v| (started, v));
        self.validate_description_editor();
        if before != self.description_value()
            && (self.surface.control_page == Some(Page::Accessibility)
                && self.surface.page != Page::RangeEditor)
        {
            self.rebuild(self.surface.page);
        }
    }
    pub fn age_description(&mut self, now: u32) {
        self.live_now = now;
        if self.description_sample.is_some() && self.description_value().is_none() {
            self.description_sample = None;
            self.validate_description_editor();
            if self.surface.control_page == Some(Page::Accessibility)
                && self.surface.page != Page::RangeEditor
            {
                self.rebuild(self.surface.page);
            }
        }
    }
    fn validate_description_editor(&mut self) {
        if self.range.as_ref().is_some_and(|r| {
            r.description
                && !r.pending
                && self.description_value().map(|v| i64::from(v.value())) != Some(r.state.saved())
        }) {
            self.close_range_editor();
            self.status = cstr("Value changed or expired • reopen to adjust");
        }
    }
    pub fn focus_description_setting(&mut self) -> bool {
        if self.description_value().is_none() || self.range.as_ref().is_some_and(|r| r.pending) {
            return false;
        }
        self.rebuild(Page::Accessibility);
        self.focus_special(TABLE_BASE + 1580, GroupId(1));
        true
    }
    fn open_description_editor(&mut self) {
        if let Some(value) = self.description_value() {
            if self.open_range_editor(
                "Audio-description level",
                RangeScale::new(-15, 15, 1).unwrap(),
                i64::from(value.value()),
            ) {
                self.range.as_mut().unwrap().description = true;
            }
        }
    }
    pub fn take_description_request(&mut self) -> Option<crate::description_level::Change> {
        if !self
            .range
            .as_ref()
            .is_some_and(|r| r.description && r.pending)
        {
            return None;
        }
        let (expected, value) = self.range_request.take()?;
        Some(crate::description_level::Change {
            expected: crate::description_level::Level::new(i8::try_from(expected).ok()?)?,
            value: crate::description_level::Level::new(i8::try_from(value).ok()?)?,
        })
    }
    pub fn description_reply(&mut self, outcome: &crate::description_level::Outcome, now: u32) {
        if !self.range.as_ref().is_some_and(|r| {
            r.description
                && r.pending
                && r.state.proposal()
                    == Some((
                        i64::from(outcome.change.expected.value()),
                        i64::from(outcome.change.value.value()),
                    ))
        }) {
            return;
        }
        let confirmed = self.range_reply(
            i64::from(outcome.change.expected.value()),
            i64::from(outcome.change.value.value()),
            outcome.verified,
        );
        if confirmed {
            self.description_sample = Some((now, outcome.change.value));
            self.description_confirmation = Some((now, outcome.change.value));
        } else {
            self.description_sample = None;
            self.close_range_editor();
            self.status = cstr("Change not confirmed • use LG Settings or reopen after refresh");
        }
    }
    pub fn open_range_editor(&mut self, label: &str, scale: RangeScale, saved: i64) -> bool {
        if self.pointer_pending || self.range.as_ref().is_some_and(|r| r.pending) {
            return false;
        }
        let Some(state) = StagedRange::new(scale, saved) else {
            return false;
        };
        let return_page = if self.surface.page == Page::RangeEditor {
            self.range.as_ref().unwrap().return_page
        } else {
            self.surface.page
        };
        self.range = Some(RangeSession {
            label: label.into(),
            state,
            pending: false,
            return_page,
            description: false,
        });
        self.range_request = None;
        self.rebuild(Page::RangeEditor);
        self.focus_special(RANGE_BASE, GroupId(9));
        true
    }
    fn close_range_editor(&mut self) {
        if let Some(r) = self.range.as_ref().filter(|r| !r.pending) {
            let page = r.return_page;
            self.rebuild(page);
        }
    }
    pub fn range_layout(&self) -> Option<RangeLayout> {
        self.range
            .as_ref()
            .and_then(|r| RangeLayout::new(RANGE_FRAME, r.state))
    }
    pub fn range_state(&self) -> Option<StagedRange> {
        self.range.as_ref().map(|r| r.state)
    }
    pub fn take_range_request(&mut self) -> Option<(i64, i64)> {
        self.range_request.take()
    }
    pub fn range_reply(&mut self, expected: i64, value: i64, verified: bool) -> bool {
        let Some(r) = self
            .range
            .as_mut()
            .filter(|r| r.pending && r.state.proposal() == Some((expected, value)))
        else {
            return false;
        };
        r.pending = false;
        let confirmed = verified && r.state.confirm(expected, value);
        self.status = cstr(if confirmed {
            "Value confirmed"
        } else {
            "Change not confirmed • Back cancel"
        });
        self.input.cancel_press();
        plx_machine::idle::invalidate();
        confirmed
    }
    pub fn open(&mut self, page: Page) {
        if self.range.as_ref().is_some_and(|r| r.pending) && page != Page::RangeEditor {
            return;
        }
        self.rebuild(page)
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.finish_live_artwork();
        let _ = self.finish_packaged_icons();
        if self.brand_texture != 0 {
            plx_gfx::gfx::delete_tex(self.brand_texture);
        }
        for texture in self.navigation_textures {
            if texture != 0 {
                plx_gfx::gfx::delete_tex(texture);
            }
        }
    }
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shell_artwork_tests.rs"]
mod artwork_tests;

#[cfg(test)]
#[path = "shell_packaged_tests.rs"]
mod packaged_tests;

#[cfg(test)]
#[path = "shell_empty_state_tests.rs"]
mod empty_state_tests;

#[cfg(test)]
#[path = "shell_current_cache_tests.rs"]
mod current_cache_tests;

#[cfg(test)]
#[path = "shell_cache_frame_tests.rs"]
mod cache_frame_tests;
