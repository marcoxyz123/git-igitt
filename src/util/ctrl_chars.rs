use std::fmt;

use muncher::Muncher;
use ratatui::style::Style;
use ratatui::text::Text;

use crate::theme;

#[derive(Clone, Debug, Default)]
pub struct CtrlChunk {
    ctrl: Vec<String>,
    text: String,
}

impl CtrlChunk {
    pub fn text(text: String) -> Self {
        Self {
            ctrl: Vec::new(),
            text,
        }
    }

    pub fn parse(munch: &mut Muncher) -> Self {
        munch.reset_peek();
        if munch.seek(1) == Some("\x1B") {
            munch.eat();
        }

        let text_or_ctrl = munch.eat_until(|c| *c == '\x1B').collect::<String>();

        if text_or_ctrl.is_empty() {
            return Self {
                ctrl: Vec::new(),
                text: String::new(),
            };
        }

        munch.reset_peek();

        if munch.seek(4) == Some("\x1B[0m") {
            // eat the reset escape code
            let _ = munch.eat_until(|c| *c == 'm');
            munch.eat();

            let mut ctrl_chars = Vec::new();
            loop {
                let ctrl_text = text_or_ctrl.splitn(2, 'm').collect::<Vec<_>>();

                let mut ctrl = vec![ctrl_text[0].replace('[', "")];
                if ctrl[0].contains(';') {
                    ctrl = ctrl[0].split(';').map(|s| s.to_string()).collect();
                }
                ctrl_chars.extend(ctrl);
                if ctrl_text[1].contains('\x1B') {
                    continue;
                } else {
                    let mut text = ctrl_text[1].to_string();

                    let ws = munch.eat_until(|c| !c.is_whitespace()).collect::<String>();
                    text.push_str(&ws);

                    return Self {
                        ctrl: ctrl_chars,
                        text,
                    };
                }
            }
        } else {
            // un control coded text
            Self {
                ctrl: Vec::new(),
                text: text_or_ctrl,
            }
        }
    }
    pub fn into_text<'a>(self) -> Text<'a> {
        let mut style = Style::default();
        if self.ctrl.len() > 2 {
            // Map ANSI 256-color codes (from git-graph) to Nord palette
            // Format: ESC[38;5;Nm where N is 0-15 for basic colors
            let color = match self.ctrl[2].as_str() {
                // Standard colors (0-7)
                "0" => theme::polar_night::NORD0, // black → dark background
                "1" => theme::aurora::NORD11,     // red → Nord red
                "2" => theme::aurora::NORD14,     // green → Nord green
                "3" => theme::aurora::NORD13,     // yellow → Nord yellow
                "4" => theme::frost::NORD10,      // blue → Nord dark blue
                "5" => theme::aurora::NORD15,     // magenta → Nord purple
                "6" => theme::frost::NORD8,       // cyan → Nord cyan
                "7" => theme::snow_storm::NORD4,  // white → Nord light gray

                // Bright colors (8-15)
                "8" => theme::polar_night::NORD3, // bright black → Nord comment gray
                "9" => theme::aurora::NORD11,     // bright red → Nord red
                "10" => theme::aurora::NORD14,    // bright green → Nord green
                "11" => theme::aurora::NORD13,    // bright yellow → Nord yellow
                "12" => theme::frost::NORD9,      // bright blue → Nord blue
                "13" => theme::aurora::NORD15,    // bright magenta → Nord purple
                "14" => theme::frost::NORD7,      // bright cyan → Nord teal
                "15" => theme::snow_storm::NORD6, // bright white → Nord white

                _ => return Text::raw(self.text),
            };
            style = style.fg(color);
        } else {
            return Text::raw(self.text);
        }
        Text::styled(self.text, style)
    }
}

impl fmt::Display for CtrlChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctrl_code = self
            .ctrl
            .iter()
            .map(|c| {
                if c == "38;5;" {
                    format!("\x1B]{}", c)
                } else {
                    format!("\x1B[{}", c)
                }
            })
            .collect::<String>();
        if ctrl_code.is_empty() && self.text.is_empty() {
            Ok(())
        } else {
            write!(f, "{}{}", ctrl_code, self.text)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CtrlChars {
    parsed: Vec<CtrlChunk>,
}

impl fmt::Display for CtrlChars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .parsed
            .iter()
            .map(CtrlChunk::to_string)
            .collect::<String>();
        write!(f, "{}", text)
    }
}

impl CtrlChars {
    pub fn parse(input: &str) -> Self {
        let mut parsed = Vec::new();

        let mut munch = Muncher::new(input);
        let pre_ctrl = munch.eat_until(|c| *c == '\x1B').collect::<String>();
        parsed.push(CtrlChunk::text(pre_ctrl));

        loop {
            if munch.is_done() {
                break;
            } else {
                parsed.push(CtrlChunk::parse(&mut munch))
            }
        }
        Self { parsed }
    }

    pub fn into_text<'a>(self) -> Vec<Text<'a>> {
        self.parsed.into_iter().map(CtrlChunk::into_text).collect()
    }
}
