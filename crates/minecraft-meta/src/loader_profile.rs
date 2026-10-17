use crate::{Arguments, Library};
use serde::{Deserialize, Serialize};

/// Perfil de version de un loader (Fabric/Quilt): se aplica *sobre* una version vanilla
/// (`inheritsFrom`) cambiando la clase principal y agregando librerias y argumentos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderProfile {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Perfiles antiguos (Forge <= 1.12): cadena completa de argumentos de juego que
    /// sustituye a la de vanilla.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}
