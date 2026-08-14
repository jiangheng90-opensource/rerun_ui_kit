use crate::DesignTokens;

struct DesignTokensPerTheme {
    dark: DesignTokens,
    light: DesignTokens,
}

impl DesignTokensPerTheme {
    fn load() -> anyhow::Result<Self> {
        Ok(Self {
            dark: DesignTokens::load(egui::Theme::Dark, include_str!("../data/dark_theme.ron"))?,
            light: DesignTokens::load(egui::Theme::Light, include_str!("../data/light_theme.ron"))?,
        })
    }
}

mod design_token_access {
    use std::sync::OnceLock;

    use super::DesignTokensPerTheme;

    pub fn design_tokens_per_theme() -> &'static DesignTokensPerTheme {
        static DESIGN_TOKENS: OnceLock<DesignTokensPerTheme> = OnceLock::new();
        DESIGN_TOKENS
            .get_or_init(|| DesignTokensPerTheme::load().expect("Failed to load design tokens"))
    }
}

pub fn design_tokens_of(theme: egui::Theme) -> &'static DesignTokens {
    match theme {
        egui::Theme::Dark => &design_token_access::design_tokens_per_theme().dark,
        egui::Theme::Light => &design_token_access::design_tokens_per_theme().light,
    }
}
