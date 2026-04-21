use std::fmt;
use std::str::FromStr;

// ── Model preference wrapper ──

/// A validated model preference string sent to the Perplexity API payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPreference(&'static str);

impl ModelPreference {
    /// Returns the raw API model preference value.
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

macro_rules! define_model_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => { name: $model_name:literal, preference: $preference:literal }
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const VALID_NAMES: &'static [&'static str] = &[$($model_name),+];

            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $model_name,)+
                }
            }

            pub const fn api_preference(&self) -> ModelPreference {
                match self {
                    $(Self::$variant => ModelPreference($preference),)+
                }
            }

            pub fn valid_names_csv() -> String {
                Self::VALID_NAMES.join(", ")
            }
        }

        impl From<$name> for ModelPreference {
            fn from(value: $name) -> Self {
                value.api_preference()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($model_name => Ok(Self::$variant),)+
                    _ => Err(format!(
                        "unknown model '{s}', expected one of: {}",
                        Self::valid_names_csv()
                    )),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = String;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}

define_model_enum! {
    /// Model selection for `perplexity_search` and `perplexity_ask`.
    ///
    /// These are the non-thinking model backends available in Perplexity's
    /// auto and pro (copilot) search modes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SearchModel {
        // ── Perplexity built-in ──
        /// Default (auto) free model.
        Turbo => { name: "turbo", preference: "turbo" },
        /// Pro auto (best) model.
        ProAuto => { name: "pro-auto", preference: "pplx_pro" },
        /// Pro upgraded model.
        ProUpgraded => { name: "pro-upgraded", preference: "pplx_pro_upgraded" },
        /// Sonar model.
        Sonar => { name: "sonar", preference: "experimental" },

        // ── NVIDIA ──
        /// Nemotron 3 Super.
        Nemotron3Super => { name: "nemotron-3-super", preference: "nv_nemotron_3_super" },

        // ── Claude models ──
        /// Claude 4.6 Sonnet.
        Claude46Sonnet => { name: "claude-4.6-sonnet", preference: "claude46sonnet" },
        /// Claude 4.6 Opus.
        Claude46Opus => { name: "claude-4.6-opus", preference: "claude46opus" },

        // ── Gemini models ──
        /// Gemini 3.0 Flash.
        Gemini30Flash => { name: "gemini-3.0-flash", preference: "gemini30flash" },
        /// Gemini 3.0 Pro.
        Gemini30Pro => { name: "gemini-3.0-pro", preference: "gemini30pro" },

        // ── GPT models ──
        /// GPT-5 Pro.
        Gpt5Pro => { name: "gpt-5-pro", preference: "gpt5_pro" },
        /// GPT-5.3 Codex.
        Gpt53Codex => { name: "gpt-5.3-codex", preference: "gpt53codex" },
        /// GPT-5.4.
        Gpt54 => { name: "gpt-5.4", preference: "gpt54" },
        /// GPT-5.4 Mini.
        Gpt54Mini => { name: "gpt-5.4-mini", preference: "gpt54mini" },
    }
}

define_model_enum! {
    /// Model selection for `perplexity_reason`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ReasonModel {
        // ── Claude thinking models ──
        /// Claude 4.6 Sonnet with thinking.
        Claude46SonnetThinking => { name: "claude-4.6-sonnet-thinking", preference: "claude46sonnetthinking" },
        /// Claude 4.6 Opus with thinking.
        Claude46OpusThinking => { name: "claude-4.6-opus-thinking", preference: "claude46opusthinking" },

        // ── Gemini reasoning models ──
        /// Gemini 3.0 Flash High.
        Gemini30FlashHigh => { name: "gemini-3.0-flash-high", preference: "gemini30flash_high" },
        /// Gemini 3.1 Pro High.
        Gemini31ProHigh => { name: "gemini-3.1-pro", preference: "gemini31pro_high" },

        // ── GPT thinking models ──
        /// GPT-5 with thinking.
        Gpt5Thinking => { name: "gpt-5-thinking", preference: "gpt5_thinking" },
        /// GPT-5.1 with thinking.
        Gpt51Thinking => { name: "gpt-5.1-thinking", preference: "gpt51_thinking" },
        /// GPT-5.2 with thinking.
        Gpt52Thinking => { name: "gpt-5.2-thinking", preference: "gpt52_thinking" },
        /// GPT-5.4 with thinking.
        Gpt54Thinking => { name: "gpt-5.4-thinking", preference: "gpt54_thinking" },
    }
}

define_model_enum! {
    /// Model selection for `perplexity_computer` (ASI agentic mode).
    ///
    /// These are the model backends available for Perplexity Computer,
    /// all routed through the `pplx_asi_*` preference namespace.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ComputerModel {
        // ── Perplexity ASI base ──
        /// ASI default model.
        Asi => { name: "asi", preference: "pplx_asi" },
        /// ASI Beta.
        AsiBeta => { name: "asi-beta", preference: "pplx_asi_beta" },

        // ── Claude ASI backends ──
        /// Claude 4.6 Sonnet with thinking.
        Claude46SonnetThinking => { name: "claude-4.6-sonnet-thinking", preference: "pplx_asi_sonnet_thinking" },
        /// Claude 4.6 Sonnet without thinking.
        Claude46Sonnet => { name: "claude-4.6-sonnet", preference: "pplx_asi_sonnet" },
        /// Claude 4.6 Opus with thinking (default).
        Claude46OpusThinking => { name: "claude-4.6-opus-thinking", preference: "pplx_asi_opus_thinking" },
        /// Claude 4.6 Opus without thinking.
        Claude46Opus => { name: "claude-4.6-opus", preference: "pplx_asi_opus" },

        // ── GPT ASI backend ──
        /// GPT-5.4.
        Gpt54 => { name: "gpt-5.4", preference: "pplx_asi_gpt54" },

        // ── Other ASI backends ──
        /// Kimi.
        Kimi => { name: "kimi", preference: "pplx_asi_kimi" },
        /// Qwen.
        Qwen => { name: "qwen", preference: "pplx_asi_qwen" },
    }
}
