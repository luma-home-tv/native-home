//! Host-only framebuffer fixture for shared read-only form rendering.
//! No TV transport, input injection, snapshot inventory or settings writer.
//!
//! Run with --features hostsim and one absolute, previously absent output directory.
//! PLXNATIVE_APP_DIR must point to the framework's shipped font assets.
#[cfg(target_arch = "arm")]
compile_error!("read_only_contrast is a desktop fixture and must never be built for webOS ARM");

use native_home::model::{
    controls_form_for_section, Action, Context, ControlConstraint, Page, Setting, SettingsSection,
};
use native_home::runtime::Window;
use plx_ui::form::{Activation, FormTable};
use plx_ui::widgets::LegacyMeasure;
use plx_ui::{theme, Painter, Rect};
use serde_json::{json, Value};
use std::error::Error;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const PANEL: Rect = Rect::new(180.0, 100.0, 1560.0, 880.0);
const TABLE: Rect = Rect::new(260.0, 150.0, 1400.0, 780.0);

fn fixture() -> (Vec<Setting>, Context) {
    let context = Context {
        input: "synthetic-input".into(),
        dynamic_range: "synthetic-range".into(),
        picture_mode: "synthetic-profile".into(),
    };
    let mut rows = [
        (
            "localeInfo.locales.UI",
            "Current protected reading",
            "en-NZ",
        ),
        ("localeInfo.locales.AUD", "Unavailable reading", "fr-CA"),
        ("localeInfo.locales.AUD2", "Wrong-context reading", "de-DE"),
        ("country", "Inactive descriptor reading", "NZL"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (id, label, value))| Setting {
        id: id.into(),
        focus_key: 145 + index as u32,
        label: label.into(),
        value: value.into(),
        context: context.clone(),
        protected: true,
        available: true,
        constraint: None,
    })
    .collect::<Vec<_>>();
    rows[1].available = false;
    rows[2].context.input = "different-synthetic-input".into();
    rows[3].constraint = Some(ControlConstraint::Choices {
        values: vec!["NZL".into()],
        active: false,
    });
    (rows, context)
}

fn assert_safety(table: &FormTable<String, Action, Page>) {
    assert_eq!(
        table.table.n_rows(),
        5,
        "four readings and one enabled route"
    );
    for index in 0..4 {
        assert!(table.binding_at(index).expect("reading binding").disabled);
        assert_eq!(
            table.activate(index),
            None,
            "read-only activation must stay blocked"
        );
        assert_eq!(table.table.sections[0].rows[index].dim, index != 0);
    }
    assert_eq!(
        table.table.sections[0].rows[0].value.as_deref(),
        Some("en-NZ")
    );
    for index in [1, 2] {
        assert_eq!(
            table.table.sections[0].rows[index].value.as_deref(),
            Some("Unavailable")
        );
    }
    assert!(!table.binding_at(4).expect("LG route binding").disabled);
    assert_eq!(
        table.activate(4),
        Some(Activation::Action(Action::OpenLgSettingsSection(
            SettingsSection::General
        ))),
        "this only inspects the route token; no executor exists in this fixture"
    );
    assert_eq!(
        table.table.next_selectable(0, 1),
        Some(1),
        "unavailable rows remain focusable"
    );
}

fn paint(table: &mut FormTable<String, Action, Page>) {
    let ground = theme::SURFACE_OLED;
    plx_gfx::gfx::frame_clear(ground[0], ground[1], ground[2]);
    let painter = Painter::root();
    painter.rrect(PANEL, 24.0, 24.0, theme::SURFACE_PANEL);
    table.table.update(1.0 / 60.0, TABLE.h);
    table.table.draw(painter, TABLE, &LegacyMeasure);
}

fn capture(
    window: &Window,
    table: &mut FormTable<String, Action, Page>,
    output: &Path,
    name: &str,
    focused: bool,
    selected: i32,
) -> Result<Value, Box<dyn Error>> {
    table.table.list_focused = focused;
    table.table.sel = selected;
    for _ in 0..60 {
        paint(table);
        window.swap();
    }
    paint(table);
    assert_safety(table);
    let graphics = window.graphics_state();
    assert_eq!(graphics.error, 0);
    assert!(!graphics.scissor_enabled);
    assert_eq!(graphics.framebuffer, 0);
    assert_eq!(graphics.viewport, [0, 0, 1920, 1080]);
    let path = output.join(format!("{name}.ppm"));
    window.capture(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let rows = (0..5)
        .map(|index| {
            let binding = table.binding_at(index).expect("bound row");
            let rect = table.table.row_frame(TABLE, index as i32).expect("row geometry");
            let row = if index < 4 { &table.table.sections[0].rows[index] }
                else { &table.table.sections[1].rows[0] };
            json!({
                "index":index,"id":binding.id,"key":binding.key.0,
                "dim":row.dim,"label":row.label,"detail":row.detail,"value":row.value,
                "disabled":binding.disabled,"activation_is_none":table.activate(index).is_none(),
                "rect":[rect.x,rect.y,rect.w,rect.h],
                "fixture_state":match index {0=>"current protected",1=>"unavailable",2=>"wrong context",3=>"inactive descriptor",_=>"enabled LG route token only"}
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "frame":name,"file":format!("{name}.ppm"),
        "list_focused":focused,"selected":selected,"rows":rows,"graphics":graphics,
        "table_clip":[TABLE.x,TABLE.y,TABLE.w,TABLE.h]
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    if cfg!(target_arch = "arm") || !cfg!(feature = "hostsim") {
        return Err("Desktop hostsim feature is required; no TV execution is allowed".into());
    }
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("Usage: read_only_contrast NEW_ABSOLUTE_OUTPUT_DIRECTORY".into());
    }
    let output = PathBuf::from(&args[1]);
    if !output.is_absolute() {
        return Err("Output directory must be absolute and previously absent".into());
    }
    // Creation is exclusive. Partial evidence remains on error; nothing is deleted or reused.
    std::fs::DirBuilder::new().mode(0o700).create(&output)?;
    for font in ["appfont.ttf", "appfont-bold.ttf"] {
        if !plx_base::paths::in_app_dir(font).is_file() {
            return Err(format!("Required shipped font missing: {font}").into());
        }
    }
    let window = Window::new(false)?; // Main-thread SDL/GL and existing shipped font initialization.
    let (settings, context) = fixture();
    let mut table = FormTable::new(2000);
    table.set(
        controls_form_for_section(&settings, &context, Some(SettingsSection::General)),
        None,
    );
    assert_safety(&table);
    let stages = [
        capture(&window, &mut table, &output, "current-focused", true, 0)?,
        capture(&window, &mut table, &output, "current-unfocused", false, 0)?,
        capture(&window, &mut table, &output, "unavailable-focused", true, 1)?,
    ];
    let receipt = json!({
        "schema":1,"scope":"Synthetic desktop shared form render/activation fixture only",
        "renderer":window.renderer,"drawable":[1920,1080],"TV_calls":0,"TV_inputs":0,
        "activation_blocked_readings":4,"enabled_route_executed":false,
        "settings":settings,"context":context,"stages":stages,
        "limits":["No TV rendering, physical input, performance or panel contrast claim.",
            "Pixel contrast must be measured from the captured PPM glyph interiors and paired local background; not antialias edge minima."]
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(output.join("render-receipt.json"))?;
    serde_json::to_writer_pretty(&mut file, &receipt)?;
    file.write_all(b"\n")?;
    println!(
        "Synthetic read-only frames captured at {}",
        output.display()
    );
    Ok(())
}
