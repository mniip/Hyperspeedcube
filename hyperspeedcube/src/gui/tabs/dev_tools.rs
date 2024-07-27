use itertools::Itertools;
use std::sync::Arc;

use hyperpuzzle::{Color, ColorSystem, DevOrbit, Puzzle, PuzzleElement};

use crate::{
    app::App,
    gui::{
        components::{color_assignment_popup, DragAndDrop},
        util::EguiTempValue,
    },
    preferences::Preferences,
    puzzle::PuzzleView,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let egui_stored_state = EguiTempValue::<DevToolsState>::new(ui);

    let mut state = egui_stored_state.get().unwrap_or_default();

    ui.group(|ui| {
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                ui.set_enabled(app.has_active_puzzle());

                let r = &ui.button("Copy color system definition");
                app.with_active_puzzle_view(|p| {
                    let text_to_copy = r
                        .clicked()
                        .then(|| color_system_to_lua_code(&p.puzzle().colors, &app.prefs));
                    crate::gui::components::copy_on_click(ui, &r, text_to_copy);
                });

                ui.separator();

                if state.loaded_orbit.is_empty() {
                    ui.menu_button("Load orbit from current puzzle", |ui| {
                        app.with_active_puzzle_view(|p| {
                            let puz = p.puzzle();
                            for (i, orbit) in puz.dev_data.orbits.iter().enumerate() {
                                if ui
                                    .button(format!("#{} - {}", i + 1, orbit.description()))
                                    .clicked()
                                {
                                    ui.close_menu();
                                    state.puzzle = Some(Arc::clone(&puz));
                                    state.loaded_orbit = orbit.clone();
                                    state.names_and_order = orbit
                                        .elements
                                        .iter()
                                        .enumerate()
                                        .filter_map(|(i, elem)| {
                                            Some((i, elem.as_ref()?.name(&puz)?.clone()))
                                        })
                                        .collect();
                                }
                            }
                        });
                    });
                } else {
                    ui.columns(2, |uis| {
                        let r = uis[0].button("Copy Lua code");

                        let text_to_copy = r
                            .clicked()
                            .then(|| state.loaded_orbit.lua_code(&state.names_and_order));
                        crate::gui::components::copy_on_click(&mut uis[0], &r, text_to_copy);

                        if uis[1].button("Clear orbit").clicked() {
                            state = Default::default();
                        }
                    });
                }
            },
        );

        let Some(puz) = state.puzzle.as_ref().map(Arc::clone) else {
            ui.set_height(ui.available_height());
            return;
        };

        ui.add_space(ui.spacing().item_spacing.x - ui.spacing().item_spacing.y);

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut dnd = DragAndDrop::new(ui);
                for (i, (index, name)) in state.names_and_order.iter_mut().enumerate() {
                    dnd.vertical_reorder_by_handle(ui, i, i, |ui, _is_dragging| {
                        let text_edit = egui::TextEdit::singleline(name);
                        match &state.loaded_orbit.elements[*index] {
                            Some(PuzzleElement::Axis(axis)) => {
                                let r = ui.add(text_edit);
                                if r.hovered() || r.has_focus() {
                                    app.with_active_puzzle_view(|p| {
                                        if Arc::ptr_eq(&p.puzzle(), &puz) {
                                            p.view.temp_gizmo_highlight = Some(*axis);
                                        }
                                    });
                                }
                            }

                            Some(PuzzleElement::Color(color)) => {
                                ui.horizontal(|ui| {
                                    app.with_active_puzzle_view(|p| {
                                        if Arc::ptr_eq(&p.puzzle(), &puz) {
                                            puzzle_color_edit_button(
                                                ui,
                                                &mut p.view,
                                                &app.prefs,
                                                *color,
                                            );
                                        }
                                    });
                                    ui.add(text_edit);
                                });
                            }

                            None => todo!(),
                        }
                    });
                }
                dnd.paint_reorder_drop_lines(ui);
                if let Some(drag) = dnd.end_drag() {
                    if let Some(before_or_after) = drag.before_or_after {
                        crate::util::reorder_list(
                            &mut state.names_and_order,
                            drag.payload,
                            drag.end,
                            before_or_after,
                        )
                    }
                }
            });
    });

    egui_stored_state.set(Some(state));
}

#[derive(Debug, Default, Clone)]
struct DevToolsState {
    puzzle: Option<Arc<Puzzle>>,

    loaded_orbit: DevOrbit<PuzzleElement>,
    names_and_order: Vec<(usize, String)>,
}

fn puzzle_color_edit_button(
    ui: &mut egui::Ui,
    puzzle_view: &mut PuzzleView,
    prefs: &Preferences,
    color: Color,
) {
    let Some(rgb) = puzzle_view.get_rgb_color(color, prefs) else {
        return;
    };

    let popup_id = ui.next_auto_id().with("color_edit_popup");

    let r = crate::gui::components::show_color_button(
        ui,
        rgb,
        ui.memory(|mem| mem.is_popup_open(popup_id)),
        ui.spacing().interact_size,
        egui::Sense::click(),
    );

    if r.clicked() {
        ui.memory_mut(|mem| mem.open_popup(popup_id));
    }

    egui::popup_below_widget(ui, popup_id, &r, |ui| {
        color_assignment_popup(ui, puzzle_view, &prefs.color_palette, Some(color))
    });
}

fn color_system_to_lua_code(color_system: &ColorSystem, prefs: &Preferences) -> String {
    use hyperpuzzle::util::{escape_lua_table_key, lua_string_literal};

    use crate::preferences::MODIFIED_SUFFIX;

    let id_string_literal = lua_string_literal(&color_system.id);
    let name_string_literal = format!("{:?}", color_system.name); // escape using double quotes
    let mut default_scheme = hyperpuzzle::DEFAULT_COLOR_SCHEME_NAME.to_string();

    let mut schemes = color_system.schemes.clone();
    if let Some(custom_schemes) = prefs.color_schemes.color_systems.get(&color_system.id) {
        for scheme in custom_schemes.schemes.user_list() {
            let name = scheme
                .name
                .strip_suffix(MODIFIED_SUFFIX)
                .unwrap_or(&scheme.name)
                .to_string();
            schemes.insert(name, scheme.value.values().cloned().collect());
        }

        default_scheme = custom_schemes.schemes.last_loaded_name().clone();
        if let Some(original_name) = default_scheme.strip_suffix(MODIFIED_SUFFIX) {
            default_scheme = original_name.to_string();
        }
    }

    let mut s = format!("color_systems:add({id_string_literal}, {{\n");

    s += &format!("  name = {name_string_literal},\n");

    let has_default_colors = schemes.len() == 1;

    let color_name_kv_pairs = pad_to_common_length(color_system.list.iter_values().map(|info| {
        let string_literal = hyperpuzzle::util::lua_string_literal(&info.name);
        format!(" name = {string_literal},")
    }));
    let color_display_kv_pairs =
        pad_to_common_length(color_system.list.iter_values().map(|info| {
            let display = &info.display;
            format!(" display = {display:?},")
        }));
    let default_color_kv_pairs = match schemes.get_index(0).filter(|_| has_default_colors) {
        Some((_name, default_colors)) => Some(
            default_colors
                .iter_values()
                .map(|default_color| {
                    let default_color_string = default_color.to_string();
                    format!(" default = {default_color_string:?}")
                })
                .collect_vec(),
        ),
        None => None,
    };

    s += "\n  colors = {\n";
    for i in 0..color_system.list.len() {
        s += "    {";
        s += &color_name_kv_pairs[i];
        if has_default_colors {
            s += &color_display_kv_pairs[i];
        } else {
            s += color_display_kv_pairs[i]
                .trim_end()
                .strip_suffix(",")
                .expect("no trailing comma");
        }
        if let Some(kv_pairs) = &default_color_kv_pairs {
            s += &kv_pairs[i];
        }
        s += " },\n";
    }
    s += "  },\n";

    if !has_default_colors {
        s += "\n  schemes = {\n";
        for (name, colors) in &schemes {
            s += &format!("    {{{name:?}, {{\n");
            for (k, v) in colors {
                let k = escape_lua_table_key(&color_system.list[k].name);
                let v = v.to_string();
                s += &format!("      {k} = {v:?},\n");
            }
            s += "    }},\n";
        }
        s += "  },\n";
        s += &format!("  default = {default_scheme:?},\n");
    }

    s += "})\n";
    s
}

fn pad_to_common_length(strings: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut ret = strings.into_iter().collect_vec();
    let max_len = ret.iter().map(|s| s.len()).max().unwrap_or(0);
    for s in &mut ret {
        while s.len() < max_len {
            s.push(' ');
        }
    }
    ret
}
