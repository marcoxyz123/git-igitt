//! Nord color theme for git-igitt
//!
//! Based on the Nord color palette: https://www.nordtheme.com/

use ratatui::style::Color;

/// Nord Polar Night - Background colors
pub mod polar_night {
    use super::Color;

    /// nord0 - Dark background
    pub const NORD0: Color = Color::Rgb(46, 52, 64);
    /// nord1 - Lighter background (elevated surfaces, status bars)
    pub const NORD1: Color = Color::Rgb(59, 66, 82);
    /// nord2 - Even lighter background (line highlighting, selections)
    pub const NORD2: Color = Color::Rgb(67, 76, 94);
    /// nord3 - Brightest background (comments, subtle elements)
    pub const NORD3: Color = Color::Rgb(76, 86, 106);
}

/// Nord Snow Storm - Text colors
pub mod snow_storm {
    use super::Color;

    /// nord4 - Primary text, variables
    pub const NORD4: Color = Color::Rgb(216, 222, 233);
    /// nord5 - Subtle text, secondary content
    pub const NORD5: Color = Color::Rgb(229, 233, 240);
    /// nord6 - Bright white, emphasized text
    pub const NORD6: Color = Color::Rgb(236, 239, 244);
}

/// Nord Frost - Accent colors (blues and cyan)
pub mod frost {
    use super::Color;

    /// nord7 - Teal/cyan (classes, types)
    pub const NORD7: Color = Color::Rgb(143, 188, 187);
    /// nord8 - Bright cyan (declarations, titles)
    pub const NORD8: Color = Color::Rgb(136, 192, 208);
    /// nord9 - Blue (functions, methods)
    pub const NORD9: Color = Color::Rgb(129, 161, 193);
    /// nord10 - Dark blue (keywords, tags)
    pub const NORD10: Color = Color::Rgb(94, 129, 172);
}

/// Nord Aurora - Accent colors (semantic colors)
pub mod aurora {
    use super::Color;

    /// nord11 - Red (errors, deletions, failed)
    pub const NORD11: Color = Color::Rgb(191, 97, 106);
    /// nord12 - Orange (warnings, modifications)
    pub const NORD12: Color = Color::Rgb(208, 135, 112);
    /// nord13 - Yellow (caution, pending, strings)
    pub const NORD13: Color = Color::Rgb(235, 203, 139);
    /// nord14 - Green (success, additions, strings)
    pub const NORD14: Color = Color::Rgb(163, 190, 140);
    /// nord15 - Purple (special, numbers)
    pub const NORD15: Color = Color::Rgb(180, 142, 173);
}

// Convenient re-exports for common usage patterns

/// Background color (nord0)
pub const BG: Color = polar_night::NORD0;
/// Elevated surface background (nord1)
pub const BG_ELEVATED: Color = polar_night::NORD1;
/// Highlight/selection background (nord2)
pub const BG_HIGHLIGHT: Color = polar_night::NORD2;
/// Border color (nord3)
pub const BORDER: Color = polar_night::NORD3;

/// Primary text color (nord4)
pub const TEXT: Color = snow_storm::NORD4;
/// Secondary/dimmed text (nord3)
pub const TEXT_DIM: Color = polar_night::NORD3;
/// Emphasized text (nord6)
pub const TEXT_BRIGHT: Color = snow_storm::NORD6;

/// Accent color - cyan (nord8)
pub const ACCENT: Color = frost::NORD8;
/// Accent color - blue (nord9)
pub const ACCENT_BLUE: Color = frost::NORD9;
/// Accent color - teal (nord7)
pub const ACCENT_TEAL: Color = frost::NORD7;

/// Success color - green (nord14)
pub const SUCCESS: Color = aurora::NORD14;
/// Warning color - yellow (nord13)
pub const WARNING: Color = aurora::NORD13;
/// Error color - red (nord11)
pub const ERROR: Color = aurora::NORD11;
/// Info color - orange (nord12)
pub const INFO: Color = aurora::NORD12;
/// Special/highlight color - purple (nord15)
pub const SPECIAL: Color = aurora::NORD15;

/// Pipeline status colors
pub mod pipeline {
    use super::*;

    /// Job success (green)
    pub const SUCCESS: Color = aurora::NORD14;
    /// Job running (cyan)
    pub const RUNNING: Color = frost::NORD8;
    /// Job pending (yellow)
    pub const PENDING: Color = aurora::NORD13;
    /// Job failed (red)
    pub const FAILED: Color = aurora::NORD11;
    /// Job canceled (gray)
    pub const CANCELED: Color = polar_night::NORD3;
    /// Job skipped (dim gray)
    pub const SKIPPED: Color = polar_night::NORD3;
    /// Job manual (purple)
    pub const MANUAL: Color = aurora::NORD15;
    /// Job created/waiting (blue)
    pub const CREATED: Color = frost::NORD9;
}

/// Diff view colors
pub mod diff {
    use super::*;

    /// Added lines (green)
    pub const ADDED: Color = aurora::NORD14;
    /// Removed lines (red)
    pub const REMOVED: Color = aurora::NORD11;
    /// Modified/changed (orange)
    pub const MODIFIED: Color = aurora::NORD12;
    /// Context lines (dim)
    pub const CONTEXT: Color = snow_storm::NORD4;
    /// Hunk header (cyan)
    pub const HUNK_HEADER: Color = frost::NORD8;
}

/// Graph view colors for branch visualization
pub mod graph {
    use super::*;

    /// Branch colors - cycle through these for different branches
    pub const BRANCH_COLORS: [Color; 6] = [
        frost::NORD8,   // cyan
        aurora::NORD14, // green
        aurora::NORD13, // yellow
        aurora::NORD12, // orange
        aurora::NORD15, // purple
        frost::NORD9,   // blue
    ];

    /// HEAD indicator
    pub const HEAD: Color = frost::NORD8;
    /// Selected commit
    pub const SELECTED: Color = snow_storm::NORD6;
    /// Tag indicator
    pub const TAG: Color = aurora::NORD13;
    /// Remote branch indicator
    pub const REMOTE: Color = aurora::NORD12;
}
