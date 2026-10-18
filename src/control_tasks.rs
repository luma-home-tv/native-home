//! Named LG-managed tasks. These are manual directions plus existing overview
//! actions, never deep links, native settings editors or new service operations.
use crate::model::{Action, Page, SettingsSection};
use plx_ui::{
    form::{Form, FormSection, RowKey, RowKind},
    table::Row,
};

struct Task {
    id: &'static str,
    key: u32,
    title: &'static str,
    detail: &'static str,
    open_overview: bool,
}
const GENERAL: [Task; 6] = [
    Task {
        id: "task-time-timers",
        key: 1700,
        title: "Time & timers",
        detail: "Opens General overview • Then System > Time & Timer",
        open_overview: true,
    },
    Task {
        id: "task-hdmi-cec",
        key: 1701,
        title: "HDMI-CEC (SIMPLINK)",
        detail: "Opens General overview • Then External Devices > HDMI Settings",
        open_overview: true,
    },
    Task {
        id: "task-bluetooth-input",
        key: 1702,
        title: "Bluetooth input devices",
        detail: "Opens General overview • Then External Devices > Connect Bluetooth Controller",
        open_overview: true,
    },
    Task {
        id: "task-programmes",
        key: 1703,
        title: "Programmes & tuning",
        detail: "Opens General overview • Then Programmes > Programme Tuning",
        open_overview: true,
    },
    Task {
        id: "task-app-management",
        key: 1704,
        title: "App management",
        detail: "Use the LG Home sidebar entry, then Edit App List in LG Home",
        open_overview: false,
    },
    Task {
        id: "task-memory",
        key: 1705,
        title: "Memory Optimiser",
        detail: "Use LG Settings > Support > Memory Optimiser",
        open_overview: false,
    },
];
const SOUND: [Task; 1] = [Task {
    id: "task-bluetooth-audio",
    key: 1706,
    title: "Bluetooth audio",
    detail: "Opens Sound overview • Then Sound Out > Use Wireless Speaker",
    open_overview: true,
}];

/// Preserve the existing overview/readout/editor sections and their keys. The
/// extra rows never select parameters beyond the independently gated overview.
/// Unsupported destinations stay legible, focusable, read-only guidance.
pub fn with_task_guides(
    form: Form<String, Action, Page>,
    route: Option<SettingsSection>,
) -> Form<String, Action, Page> {
    let (tasks, route) = match route {
        Some(SettingsSection::General) => (&GENERAL[..], SettingsSection::General),
        Some(SettingsSection::Sound) => (&SOUND[..], SettingsSection::Sound),
        _ => return form,
    };
    let mut section = FormSection::new("LG TASK GUIDES");
    for task in tasks {
        section = section
            .item_keyed(
                task.id.into(),
                RowKey(task.key),
                RowKind::Button,
                if task.open_overview {
                    Action::OpenLgSettingsSection(route)
                } else {
                    Action::Inspect(task.id.into())
                },
                Row::new(task.title)
                    .detail(task.detail)
                    .value(if task.open_overview {
                        "LG overview"
                    } else {
                        "Guide only"
                    })
                    .chevron(task.open_overview),
            )
            .read_only(!task.open_overview);
    }
    form.section(section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actions::{self, Capabilities, Op, Platform, Request},
        model::Context,
    };
    use plx_ui::form::{Activation, FormTable};
    fn form(route: Option<SettingsSection>) -> Form<String, Action, Page> {
        crate::model::controls_form_for_section(
            &[],
            &Context {
                input: String::new(),
                dynamic_range: String::new(),
                picture_mode: String::new(),
            },
            route,
        )
    }
    #[test]
    fn tasks_append_stable_unique_keys_without_replacing_overview_or_readout_sections() {
        let _serial = plx_base::testlock::serial();
        for (route, tasks) in [
            (SettingsSection::General, &GENERAL[..]),
            (SettingsSection::Sound, &SOUND[..]),
        ] {
            let mut table = FormTable::new(2000);
            table.set(with_task_guides(form(Some(route)), Some(route)), None);
            assert_eq!(table.table.sections[0].rows.len(), 1);
            assert_eq!(table.binding_at(0).unwrap().key, RowKey(route.focus_key()));
            assert_eq!(
                table.table.sections[1].header,
                crate::strings::Message::CurrentSettings.text()
            );
            let mut keys = std::collections::BTreeSet::new();
            for task in tasks {
                assert!(keys.insert(task.key));
                assert!(task.key > 1580 && task.key < 2000);
                let i = table.index_of_key(RowKey(task.key)).unwrap();
                let binding = table.binding_at(i).unwrap();
                assert_eq!(binding.id, task.id);
                assert_eq!(binding.disabled, !task.open_overview);
                assert_eq!(
                    table.activate(i),
                    task.open_overview
                        .then_some(Activation::Action(Action::OpenLgSettingsSection(route)))
                );
                if !task.open_overview {
                    assert!(Request::from_action(binding.action.clone()).is_none());
                }
            }
        }
    }
    #[test]
    fn unsupported_categories_and_missing_overview_route_add_no_task_actions() {
        let _serial = plx_base::testlock::serial();
        for route in [
            None,
            Some(SettingsSection::Picture),
            Some(SettingsSection::Network),
            Some(SettingsSection::Accessibility),
            Some(SettingsSection::PanelCare),
        ] {
            let mut table = FormTable::new(2000);
            table.set(with_task_guides(form(route), route), None);
            assert_eq!(table.table.sections.len(), 2);
            assert!(!(0..table.table.n_rows() as usize).any(|i| table
                .binding_at(i)
                .is_some_and(|b| b.id.starts_with("task-"))));
            if route.is_none() {
                assert_eq!(table.activate(0), None);
            }
        }
    }
    struct NoCalls;
    impl Platform for NoCalls {
        fn call(&mut self, _: Op, _: serde_json::Value) -> Result<serde_json::Value, String> {
            panic!("missing settings capability must refuse before platform I/O")
        }
        fn wait(&mut self) {
            panic!("missing settings capability must never wait")
        }
    }
    #[test]
    fn task_links_remain_fixed_overviews_and_cannot_bypass_worker_capability() {
        let _serial = plx_base::testlock::serial();
        for route in [SettingsSection::General, SettingsSection::Sound] {
            let action = Action::OpenLgSettingsSection(route);
            let request = Request::from_action(action).unwrap();
            assert_eq!(
                serde_json::to_value(&request).unwrap(),
                serde_json::json!({"kind":"LgSettingsSection","section":if route==SettingsSection::General {"general"} else {"sound"}})
            );
            let receipt = actions::execute(
                &mut NoCalls,
                crate::identity::PREVIEW,
                request,
                Capabilities::default(),
            );
            assert!(!receipt.admitted && !receipt.verified);
        }
    }
    #[test]
    fn descriptions_state_actual_landing_and_unsupported_tasks_remain_guide_only() {
        for task in GENERAL.iter().chain(SOUND.iter()) {
            if task.open_overview {
                assert!(
                    task.detail.starts_with("Opens ") && task.detail.contains(" overview • Then ")
                );
            } else {
                assert!(!task.detail.starts_with("Opens "));
            }
        }
        assert!(!GENERAL[4].open_overview && !GENERAL[5].open_overview);
    }
}
