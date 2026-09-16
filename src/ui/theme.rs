use ratatui::style::Color;

/// Paleta cromática oficial de Timeless Support & Outlook Organizer TS
pub struct Theme;

impl Theme {
    // Acentos primarios y secundarios
    pub const ACCENT_PRIMARY: Color = Color::Rgb(0, 164, 239);    // Cyan vibrante Outlook (#00A4EF)
    pub const ACCENT_SECONDARY: Color = Color::Rgb(91, 95, 199);  // Azul Microsoft 365 (#5B5FC7)

    // Estados
    pub const SUCCESS: Color = Color::Rgb(0, 204, 106);           // Verde éxito (#00CC6A)
    pub const WARNING: Color = Color::Rgb(255, 185, 0);           // Ámbar advertencia (#FFB900)
    pub const DANGER: Color = Color::Rgb(216, 59, 1);             // Rojo peligro (#D83B01)

    // Superficies y texto
    pub const BG_DARK: Color = Color::Rgb(15, 23, 42);            // Slate ultra oscuro fondo
    pub const BG_CARD: Color = Color::Rgb(37, 37, 38);            // Gris oscuro superficie
    pub const TEXT_MAIN: Color = Color::Rgb(255, 255, 255);       // Blanco principal
    pub const TEXT_MUTED: Color = Color::Rgb(138, 136, 134);      // Gris atenuado

    // Identidad de marca Timeless Support
    pub const BRAND_PRIMARY: Color = Color::Rgb(0, 229, 255);     // Cyan brillante logo
    pub const BRAND_SECONDARY: Color = Color::Rgb(200, 200, 200); // Texto 'SUPPORT'
}
