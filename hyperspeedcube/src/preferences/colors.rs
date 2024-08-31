use std::collections::{btree_map, BTreeMap};

use hyperpuzzle::{ColorSystem, DefaultColor, Rgb};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::L;

use super::{schema, PresetsList, DEFAULT_PREFS_RAW};

pub type ColorScheme = IndexMap<String, DefaultColor>;

#[derive(Debug, Default)]
pub struct ColorSchemePreferences(BTreeMap<String, ColorSystemPreferences>);
impl schema::PrefsConvert for ColorSchemePreferences {
    type DeserContext = ();
    type SerdeFormat = BTreeMap<String, schema::current::ColorSystemPreferences>;

    fn to_serde(&self) -> Self::SerdeFormat {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), v.to_serde()))
            .collect()
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        schema::reload_btreemap(&mut self.0, ctx, value);
    }
}
impl ColorSchemePreferences {
    pub fn get(&self, color_system: &ColorSystem) -> Option<&ColorSystemPreferences> {
        self.0.get(&color_system.id)
    }
    pub fn get_mut(&mut self, color_system: &ColorSystem) -> &mut ColorSystemPreferences {
        match self.0.entry(color_system.id.clone()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(ColorSystemPreferences::from_color_system(color_system))
            }
            btree_map::Entry::Occupied(mut e) => {
                e.get_mut().update_builtin_schemes(color_system);
                e.into_mut()
            }
        }
    }
}

#[derive(Debug, Default, Display, EnumString, EnumIter, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DefaultColorGradient {
    #[default]
    Rainbow,
    // Sinebow,
    // Turbo,
    // Spectral,
    // Cool,
    // Warm,
    // Plasma,
    // Viridis,
    // Cividis,
}
impl DefaultColorGradient {
    /// Returns the gradient as a [`colorous::Gradient`].
    pub fn to_colorous(self) -> colorous::Gradient {
        match self {
            Self::Rainbow => colorous::RAINBOW,
            // Self::Sinebow => colorous::SINEBOW,
            // Self::Turbo => colorous::TURBO,
            // Self::Spectral => colorous::SPECTRAL,
            // Self::Cool => colorous::COOL,
            // Self::Warm => colorous::WARM,
            // Self::Plasma => colorous::PLASMA,
            // Self::Viridis => colorous::VIRIDIS,
            // Self::Cividis => colorous::CIVIDIS,
        }
    }
    /// Samples the gradient at a point.
    pub fn sample(self, index: usize, total: usize) -> Rgb {
        let rgb = self.to_colorous().eval_rational(index, total).as_array();
        Rgb { rgb }
    }
    /// Returns a [`DefaultColor`] for the gradient
    pub fn default_color_at(self, index: usize, total: usize) -> DefaultColor {
        DefaultColor::Gradient {
            gradient_name: self.to_string(),
            index,
            total,
        }
    }
    pub fn default_color_at_end(self) -> DefaultColor {
        DefaultColor::Gradient {
            gradient_name: self.to_string(),
            index: usize::MAX,
            total: usize::MAX,
        }
    }
}

#[derive(Debug, Default)]
pub struct GlobalColorPalette {
    pub custom_colors: PresetsList<Rgb>,
    pub builtin_colors: IndexMap<String, Rgb>,
    pub builtin_color_sets: IndexMap<String, Vec<Rgb>>,
}
impl schema::PrefsConvert for GlobalColorPalette {
    type DeserContext = ();
    type SerdeFormat = schema::current::GlobalColorPalette;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self {
            custom_colors,
            builtin_colors,
            builtin_color_sets,
        } = self;

        schema::current::GlobalColorPalette {
            custom_colors: custom_colors.to_serde_map(),
            builtin_colors: builtin_colors.clone(),
            builtin_color_sets: builtin_color_sets.clone(),
        }
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        let schema::current::GlobalColorPalette {
            custom_colors,
            builtin_colors,
            builtin_color_sets,
        } = value;

        self.custom_colors.reload_from_serde_map(ctx, custom_colors);

        self.builtin_colors = DEFAULT_PREFS_RAW
            .color_palette
            .builtin_colors
            .iter()
            .map(|(k, v)| (k.clone(), *builtin_colors.get(k).unwrap_or(v)))
            .collect();

        self.builtin_color_sets = DEFAULT_PREFS_RAW
            .color_palette
            .builtin_color_sets
            .iter()
            .map(|(k, v)| {
                let user_value = builtin_color_sets.get(k).unwrap_or(const { &Vec::new() });
                (
                    k.clone(),
                    v.iter()
                        .enumerate()
                        .map(|(i, v)| *user_value.get(i).unwrap_or(v))
                        .collect(),
                )
            })
            .collect();
    }
}
impl GlobalColorPalette {
    pub fn has(&self, color: &DefaultColor) -> bool {
        match color {
            // Skip sampling the gradient
            DefaultColor::Gradient { gradient_name, .. } => {
                gradient_name.parse::<DefaultColorGradient>().is_ok()
            }

            // No way to make the other cases faster
            _ => self.get(color).is_some(),
        }
    }

    pub fn get_set(&self, set_name: &str) -> Option<&Vec<Rgb>> {
        self.builtin_color_sets.get(set_name)
    }

    pub fn get(&self, color: &DefaultColor) -> Option<Rgb> {
        match color {
            DefaultColor::Unknown => None,
            DefaultColor::HexCode { rgb } => Some(*rgb),
            DefaultColor::Single { name } => None
                .or_else(|| self.builtin_colors.get(name))
                .or_else(|| Some(&self.custom_colors.get(name)?.value))
                .copied(),
            DefaultColor::Set { set_name, index } => self
                .get_set(set_name)
                .and_then(|set| set.get(*index))
                .copied(),
            DefaultColor::Gradient {
                gradient_name,
                index,
                total,
            } => {
                let gradient = gradient_name.parse::<DefaultColorGradient>().ok()?;
                Some(gradient.sample(*index, *total))
            }
        }
    }

    /// Modfies a color scheme if necessary to ensure that it is valid for its
    /// color system and the current global palette. Returns `true` if it was
    /// modified.
    #[must_use]
    pub fn ensure_color_scheme_is_valid_for_color_system(
        &self,
        scheme: &mut ColorScheme,
        color_system: &ColorSystem,
    ) -> bool {
        let mut changed = false;

        let names_match = itertools::equal(
            scheme.iter().map(|(k, _v)| k),
            color_system.list.iter().map(|(_id, color)| &color.name),
        );
        if !names_match {
            changed = true;
            *scheme = color_system
                .list
                .iter_values()
                .map(|color| {
                    scheme
                        .swap_remove_entry(&color.name)
                        .unwrap_or_else(|| (color.name.clone(), DefaultColor::Unknown))
                })
                .collect();
        }

        changed |= hyperpuzzle::ensure_color_scheme_is_valid(scheme.values_mut(), |c| self.has(c));

        changed
    }

    pub fn groups_of_sets(&self) -> Vec<(String, Vec<(&String, &[Rgb])>)> {
        self.builtin_color_sets
            .iter()
            .sorted_by_key(|(_, colors)| colors.len())
            .chunk_by(|(_, colors)| colors.len())
            .into_iter()
            .map(|(value, sets)| {
                let group_name = match value {
                    1 => L.colors.set_sizes._1.to_string(),
                    2 => L.colors.set_sizes._2.to_string(),
                    3 => L.colors.set_sizes._3.to_string(),
                    4 => L.colors.set_sizes._4.to_string(),
                    5 => L.colors.set_sizes._5.to_string(),
                    6 => L.colors.set_sizes._6.to_string(),
                    7 => L.colors.set_sizes._7.to_string(),
                    8 => L.colors.set_sizes._8.to_string(),
                    9 => L.colors.set_sizes._9.to_string(),
                    10 => L.colors.set_sizes._10.to_string(),
                    n => L.colors.set_sizes.n.with(&n.to_string()),
                };
                (
                    group_name,
                    sets.map(|(name, rgbs)| (name, rgbs.as_slice())).collect(),
                )
            })
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct ColorSystemPreferences {
    pub schemes: PresetsList<ColorScheme>,
}
impl schema::PrefsConvert for ColorSystemPreferences {
    type DeserContext = ();
    type SerdeFormat = schema::current::ColorSystemPreferences;

    fn to_serde(&self) -> Self::SerdeFormat {
        let Self { schemes } = self;

        schemes
            .user_presets()
            .map(|preset| (preset.name().clone(), preset.value.clone()))
            .collect()
    }
    fn reload_from_serde(&mut self, ctx: &Self::DeserContext, value: Self::SerdeFormat) {
        self.schemes.reload_from_serde_map(ctx, value);
    }
}
impl ColorSystemPreferences {
    /// Creates factory-default preferences for a color system.
    pub fn from_color_system(color_system: &ColorSystem) -> Self {
        let mut ret = Self::default();
        ret.update_builtin_schemes(color_system);
        ret.schemes
            .set_last_loaded(color_system.default_scheme.clone());
        ret
    }
    /// Updates the built-in schemes for the color system.
    ///
    /// Deletes any user color schemes with the same name.
    pub fn update_builtin_schemes(&mut self, color_system: &ColorSystem) {
        self.schemes.set_builtin_presets(
            color_system
                .schemes
                .keys()
                .map(|name| preset_from_color_scheme(color_system, name))
                .collect(),
        );
    }
}

fn preset_from_color_scheme(color_system: &ColorSystem, name: &str) -> (String, ColorScheme) {
    let value = color_system
        .get_scheme_or_default(name)
        .iter()
        .map(|(id, default_color)| (color_system.list[id].name.clone(), default_color.clone()))
        .collect();
    (name.to_string(), value)
}
