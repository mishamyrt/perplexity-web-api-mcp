use std::fmt;
use std::str::FromStr;

pub const DEEP_RESEARCH_MODEL_PREFERENCE: &str = "pplx_alpha";

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
    /// Model selection for `perplexity_search`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SearchModel {
        /// Default (auto) free model
        Turbo => { name: "turbo", preference: "turbo" },
        /// Pro auto (best) model.
        ProAuto => { name: "pro-auto", preference: "pplx_pro" },
        /// Sonar model.
        Sonar => { name: "sonar", preference: "experimental" },
        /// GPT-5.4 model.
        Gpt54 => { name: "gpt-5.4", preference: "gpt54" },
        /// Claude 4.6 Sonnet model.
        Claude46Sonnet => { name: "claude-4.6-sonnet", preference: "claude46sonnet" },
        /// Nemotron 3 Super
        Nemotron3Super => { name: "nemotron-3-super", preference: "nv_nemotron_3_super" },
    }
}

define_model_enum! {
    /// Model selection for `perplexity_reason`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ReasonModel {
        /// Gemini 3.1 Pro model.
        Gemini31Pro => { name: "gemini-3.1-pro", preference: "gemini31pro_high" },
        /// GPT-5.4 with thinking capabilities.
        Gpt54Thinking => { name: "gpt-5.4-thinking", preference: "gpt54_thinking" },
        /// Claude 4.6 Sonnet with thinking capabilities.
        Claude46SonnetThinking => { name: "claude-4.6-sonnet-thinking", preference: "claude46sonnetthinking" },
    }
}
