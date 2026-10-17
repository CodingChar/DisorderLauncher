use crate::{Colors, Theme, SCHEMA_VERSION};

/// Tamano maximo de un archivo de tema: un tema legitimo ocupa ~1 KB.
pub const MAX_THEME_BYTES: usize = 32 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ThemeError {
    #[error("el tema es demasiado grande")]
    TooLarge,
    #[error("el archivo no es un tema valido: {0}")]
    Malformed(String),
    #[error("version de tema no soportada: {0}")]
    UnsupportedSchema(u32),
    #[error("identificador de tema invalido")]
    InvalidId,
    #[error("nombre de tema invalido")]
    InvalidName,
    #[error("color invalido en «{0}»: use el formato #RRGGBB")]
    InvalidColor(&'static str),
    #[error("tipografia invalida en «{0}»")]
    InvalidFont(&'static str),
    #[error("valor fuera de rango en «{0}»")]
    OutOfRange(&'static str),
    #[error("los colores de «{0}» no tienen suficiente contraste para leerse bien")]
    LowContrast(&'static str),
    #[error("ya existe un tema con ese identificador")]
    Duplicate,
    #[error("no se puede modificar un tema incluido")]
    Builtin,
    #[error("tema no encontrado")]
    NotFound,
    #[error("no se pudo leer o guardar el tema")]
    Io,
}

/// Parsea y valida. Cualquier fallo deja el tema rechazado (el llamador conserva el anterior).
pub fn parse_theme(json: &[u8]) -> Result<Theme, ThemeError> {
    if json.len() > MAX_THEME_BYTES {
        return Err(ThemeError::TooLarge);
    }
    let theme: Theme = serde_json::from_slice(json).map_err(|e| ThemeError::Malformed(e.classify_label()))?;
    validate_theme(&theme)?;
    Ok(theme)
}

trait ClassifyLabel {
    fn classify_label(&self) -> String;
}

impl ClassifyLabel for serde_json::Error {
    /// Mensaje corto y sin eco del contenido del archivo.
    fn classify_label(&self) -> String {
        match self.classify() {
            serde_json::error::Category::Syntax | serde_json::error::Category::Eof => "JSON mal formado".into(),
            serde_json::error::Category::Data => "faltan campos o hay campos desconocidos".into(),
            serde_json::error::Category::Io => "error de lectura".into(),
        }
    }
}

pub fn validate_theme(theme: &Theme) -> Result<(), ThemeError> {
    if theme.schema_version != SCHEMA_VERSION {
        return Err(ThemeError::UnsupportedSchema(theme.schema_version));
    }
    validate_id(&theme.id)?;
    let name = theme.name.trim();
    if name.is_empty() || theme.name.chars().count() > 128 || theme.name.chars().any(char::is_control) {
        return Err(ThemeError::InvalidName);
    }

    validate_colors(&theme.colors)?;

    for (label, family) in [("displayFamily", &theme.typography.display_family), ("bodyFamily", &theme.typography.body_family)] {
        validate_font_family(family).map_err(|_| ThemeError::InvalidFont(label))?;
    }
    in_range("typography.scale", theme.typography.scale, 0.8, 1.4)?;
    in_range("density.uiScale", theme.density.ui_scale, 0.8, 1.4)?;
    in_range("effects.shadowStrength", theme.effects.shadow_strength, 0.0, 1.0)?;

    let shape = &theme.shape;
    for (label, radius) in [
        ("shape.inputRadius", shape.input_radius),
        ("shape.buttonRadius", shape.button_radius),
        ("shape.cardRadius", shape.card_radius),
        ("shape.heroRadius", shape.hero_radius),
    ] {
        if radius > 32 {
            return Err(ThemeError::OutOfRange(label));
        }
    }
    Ok(())
}

fn in_range(label: &'static str, value: f64, min: f64, max: f64) -> Result<(), ThemeError> {
    if value.is_finite() && value >= min && value <= max { Ok(()) } else { Err(ThemeError::OutOfRange(label)) }
}

/// `a-z`, `0-9`, `.`, `-`, `_`; empieza con letra o numero; hasta 128 caracteres.
fn validate_id(id: &str) -> Result<(), ThemeError> {
    let ok = !id.is_empty()
        && id.len() <= 128
        && id.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'));
    if ok { Ok(()) } else { Err(ThemeError::InvalidId) }
}

/// Nombre de familia sin comillas, punto y coma, llaves, parentesis ni URLs: solo letras,
/// numeros, espacios y guiones. El frontend lo entrecomilla y agrega una pila de respaldo.
fn validate_font_family(family: &str) -> Result<(), ()> {
    let trimmed = family.trim();
    let ok = !trimmed.is_empty()
        && trimmed.chars().count() <= 64
        && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
    if ok { Ok(()) } else { Err(()) }
}

fn validate_colors(c: &Colors) -> Result<(), ThemeError> {
    let named: [(&'static str, &String); 10] = [
        ("background", &c.background),
        ("surface", &c.surface),
        ("surfaceAlt", &c.surface_alt),
        ("surfaceRaised", &c.surface_raised),
        ("textPrimary", &c.text_primary),
        ("textSecondary", &c.text_secondary),
        ("border", &c.border),
        ("borderStrong", &c.border_strong),
        ("action", &c.action),
        ("actionText", &c.action_text),
    ];
    for (label, value) in named {
        parse_hex(value).ok_or(ThemeError::InvalidColor(label))?;
    }

    // Accesibilidad AA (4.5:1) para el texto sobre sus fondos habituales.
    let pairs: [(&'static str, &str, &str); 5] = [
        ("textPrimary", &c.text_primary, &c.background),
        ("textPrimary", &c.text_primary, &c.surface),
        ("textSecondary", &c.text_secondary, &c.surface),
        ("textSecondary", &c.text_secondary, &c.background),
        ("actionText", &c.action_text, &c.action),
    ];
    for (label, fg, bg) in pairs {
        if contrast_ratio(parse_hex(fg).unwrap(), parse_hex(bg).unwrap()) < 4.5 {
            return Err(ThemeError::LowContrast(label));
        }
    }
    Ok(())
}

/// Solo `#RRGGBB` (mayusculas o minusculas).
pub fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn luminance([r, g, b]: [u8; 3]) -> f64 {
    let channel = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// Relacion de contraste WCAG entre dos colores.
pub fn contrast_ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: &str = include_str!("../../../theme_examples/default-dark.theme.json");
    const LIGHT: &str = include_str!("../../../theme_examples/default-light.theme.json");

    fn with(replacements: &[(&str, &str)]) -> String {
        let mut s = DARK.to_string();
        for (from, to) in replacements {
            assert!(s.contains(from), "no esta {from} en el ejemplo");
            s = s.replace(from, to);
        }
        s
    }

    #[test]
    fn bundled_examples_are_valid() {
        assert!(parse_theme(DARK.as_bytes()).is_ok());
        assert!(parse_theme(LIGHT.as_bytes()).is_ok());
    }

    #[test]
    fn contrast_math_matches_wcag() {
        assert!((contrast_ratio([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 0.01);
        assert!((contrast_ratio([255, 255, 255], [255, 255, 255]) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_hex_and_css_injection_in_colors() {
        for bad in ["red", "#12345", "#1234567", "rgb(0,0,0)", "url(http://x)", "#GGGGGG", "#0C0C0C; } body{display:none", "var(--x)"] {
            let json = with(&[("\"#0C0C0C\"", &format!("{bad:?}"))]);
            assert!(matches!(parse_theme(json.as_bytes()), Err(ThemeError::InvalidColor("background"))), "{bad}");
        }
    }

    #[test]
    fn rejects_unsafe_font_families() {
        for bad in ["Arial; } body { display: none", "url(https://evil/x.woff)", "\\\"x", "A{B}", "Arial, sans-serif", ""] {
            let json = with(&[("\"Space Grotesk\"", &format!("{bad:?}"))]);
            assert!(matches!(parse_theme(json.as_bytes()), Err(ThemeError::InvalidFont("displayFamily"))), "{bad}");
        }
    }

    #[test]
    fn rejects_unknown_fields_everywhere() {
        let top = with(&[("\"mode\": \"dark\",", "\"mode\": \"dark\", \"script\": \"alert(1)\",")]);
        assert!(matches!(parse_theme(top.as_bytes()), Err(ThemeError::Malformed(_))));
        let nested = with(&[("\"scale\": 1.0\n  },", "\"scale\": 1.0, \"importUrl\": \"https://x\"\n  },")]);
        assert!(matches!(parse_theme(nested.as_bytes()), Err(ThemeError::Malformed(_))));
    }

    #[test]
    fn rejects_out_of_range_values() {
        let radius = with(&[("\"cardRadius\": 13", "\"cardRadius\": 500")]);
        assert_eq!(parse_theme(radius.as_bytes()), Err(ThemeError::OutOfRange("shape.cardRadius")));
        let scale = with(&[("\"scale\": 1.0", "\"scale\": 9.0")]);
        assert_eq!(parse_theme(scale.as_bytes()), Err(ThemeError::OutOfRange("typography.scale")));
        let shadow = with(&[("\"shadowStrength\": 0.2", "\"shadowStrength\": -1")]);
        assert_eq!(parse_theme(shadow.as_bytes()), Err(ThemeError::OutOfRange("effects.shadowStrength")));
    }

    #[test]
    fn rejects_unreadable_color_combinations() {
        // Texto casi igual al fondo.
        let json = with(&[("\"#F0F0EA\"", "\"#151515\"")]);
        assert_eq!(parse_theme(json.as_bytes()), Err(ThemeError::LowContrast("textPrimary")));
    }

    #[test]
    fn rejects_bad_ids_schema_and_size() {
        for bad in ["", "Has Space", "../x", "UPPER", "a/b", &"x".repeat(129)] {
            let json = with(&[("\"disorderlaucher.default.dark\"", &format!("{bad:?}"))]);
            assert_eq!(parse_theme(json.as_bytes()), Err(ThemeError::InvalidId), "{bad}");
        }
        let v2 = with(&[("\"schemaVersion\": 1", "\"schemaVersion\": 2")]);
        assert_eq!(parse_theme(v2.as_bytes()), Err(ThemeError::UnsupportedSchema(2)));
        assert_eq!(parse_theme(&vec![b' '; MAX_THEME_BYTES + 1]), Err(ThemeError::TooLarge));
        assert!(matches!(parse_theme(b"not json"), Err(ThemeError::Malformed(_))));
    }

    #[test]
    fn optional_sections_default() {
        let json = with(&[]);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut value = value;
        value.as_object_mut().unwrap().remove("density");
        value.as_object_mut().unwrap().remove("effects");
        let theme = parse_theme(value.to_string().as_bytes()).unwrap();
        assert!(theme.effects.animations);
        assert_eq!(theme.density.ui_scale, 1.0);
    }
}
