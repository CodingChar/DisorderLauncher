use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
}

/// `deny_unknown_fields` en todas las secciones: una clave desconocida es un error,
/// no algo que se ignora en silencio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Theme {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub mode: ThemeMode,
    pub colors: Colors,
    pub typography: Typography,
    pub shape: Shape,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub effects: Effects,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Colors {
    pub background: String,
    pub surface: String,
    pub surface_alt: String,
    pub surface_raised: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub border: String,
    pub border_strong: String,
    pub action: String,
    pub action_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Typography {
    pub display_family: String,
    pub body_family: String,
    #[serde(default = "one")]
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shape {
    pub input_radius: u32,
    pub button_radius: u32,
    pub card_radius: u32,
    pub hero_radius: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Density {
    #[serde(default = "one")]
    pub ui_scale: f64,
    #[serde(default)]
    pub compact: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Effects {
    #[serde(default = "default_shadow")]
    pub shadow_strength: f64,
    #[serde(default = "yes")]
    pub animations: bool,
}

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

fn default_shadow() -> f64 {
    0.2
}

impl Default for Density {
    fn default() -> Self {
        Self { ui_scale: 1.0, compact: false }
    }
}

impl Default for Effects {
    fn default() -> Self {
        Self { shadow_strength: default_shadow(), animations: true }
    }
}

/// Un tema de la lista, indicando si viene de serie (no se puede borrar).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeEntry {
    pub builtin: bool,
    #[serde(flatten)]
    pub theme: Theme,
}
