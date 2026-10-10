use serde::Deserialize;
use thiserror::Error;

const DIALECT_SOURCE: &str = include_str!("../../../dialects/dragon32-basic/dialect.toml");
const TOKEN_SOURCE: &str = include_str!("../../../dialects/dragon32-basic/tokens.toml");

/// One token in a versioned BASIC dialect.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokenDefinition {
    /// Token byte following the optional function prefix.
    pub code: u8,
    /// Canonical text.
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DialectFile {
    schema_version: u32,
    id: String,
    display_name: String,
    program_start: u16,
    maximum_program_address: u16,
    maximum_line_number: u16,
    significant_variable_characters: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenFile {
    schema_version: u32,
    dialect: String,
    statement: Vec<TokenDefinition>,
    function: Vec<TokenDefinition>,
}

/// Fully validated Dragon BASIC dialect data.
#[derive(Debug, Clone)]
pub struct Dialect {
    /// Stable dialect identifier.
    pub id: String,
    /// User-facing name.
    pub display_name: String,
    /// Runtime address at which the first linked line starts.
    pub program_start: u16,
    /// Highest permitted address for the linked program.
    pub maximum_program_address: u16,
    /// Highest valid line number.
    pub maximum_line_number: u16,
    /// Number of significant variable-name characters.
    pub significant_variable_characters: u8,
    /// Statement/operator tokens.
    pub statement_tokens: Vec<TokenDefinition>,
    /// `0xFF`-prefixed function tokens.
    pub function_tokens: Vec<TokenDefinition>,
}

/// Invalid built-in dialect data.
#[derive(Debug, Error)]
pub enum DialectError {
    /// The dialect metadata could not be decoded.
    #[error("invalid built-in dialect metadata: {0}")]
    Metadata(#[source] toml::de::Error),
    /// The token table could not be decoded.
    #[error("invalid built-in token table: {0}")]
    Tokens(#[source] toml::de::Error),
    /// The decoded files disagree or contain duplicate/invalid values.
    #[error("invalid built-in dialect contract: {0}")]
    Contract(String),
}

impl Dialect {
    /// Select a supported built-in language; never silently fall back to Dragon BASIC.
    pub fn for_id(id: &str) -> Result<Self, DialectError> {
        if id == "dragon64-basic" {
            let mut dialect = Self::dragon32()?;
            id.clone_into(&mut dialect.id);
            "Dragon 64 BASIC (four graphics pages)".clone_into(&mut dialect.display_name);
            dialect.maximum_program_address = 0xbfff;
            return Ok(dialect);
        }
        if id != eightbit_dragon_targets::BASIC_DIALECT {
            return Err(DialectError::Contract("unsupported BASIC dialect".into()));
        }
        Self::dragon32()
    }

    /// Load the embedded, data-driven Dragon 32 BASIC dialect.
    pub fn dragon32() -> Result<Self, DialectError> {
        let metadata: DialectFile =
            toml::from_str(DIALECT_SOURCE).map_err(DialectError::Metadata)?;
        let tokens: TokenFile = toml::from_str(TOKEN_SOURCE).map_err(DialectError::Tokens)?;
        if metadata.schema_version != 1 || tokens.schema_version != 1 {
            return Err(DialectError::Contract(
                "only dialect schema version 1 is supported".to_owned(),
            ));
        }
        if metadata.id != tokens.dialect {
            return Err(DialectError::Contract(
                "token table dialect does not match metadata".to_owned(),
            ));
        }
        validate_tokens(&tokens.statement, false)?;
        validate_tokens(&tokens.function, true)?;
        Ok(Self {
            id: metadata.id,
            display_name: metadata.display_name,
            program_start: metadata.program_start,
            maximum_program_address: metadata.maximum_program_address,
            maximum_line_number: metadata.maximum_line_number,
            significant_variable_characters: metadata.significant_variable_characters,
            statement_tokens: tokens.statement,
            function_tokens: tokens.function,
        })
    }
}

fn validate_tokens(tokens: &[TokenDefinition], function: bool) -> Result<(), DialectError> {
    let mut codes = [false; 256];
    let mut names = std::collections::HashSet::new();
    for token in tokens {
        if token.code < 0x80 {
            return Err(DialectError::Contract(format!(
                "token '{}' has an invalid code below 0x80",
                token.text
            )));
        }
        if codes[usize::from(token.code)] || !names.insert(token.text.to_ascii_uppercase()) {
            return Err(DialectError::Contract(format!(
                "duplicate {} token '{}'",
                if function { "function" } else { "statement" },
                token.text
            )));
        }
        if !token.text.is_ascii() || token.text.is_empty() {
            return Err(DialectError::Contract(
                "token names must be non-empty ASCII".to_owned(),
            ));
        }
        codes[usize::from(token.code)] = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_dialect_is_valid() {
        let dialect = Dialect::dragon32().unwrap_or_else(|error| panic!("dialect: {error}"));
        assert_eq!(dialect.program_start, 0x1e01);
        assert_eq!(
            dialect
                .statement_tokens
                .first()
                .map(|token| token.text.as_str()),
            Some("FOR")
        );
        assert_eq!(
            dialect
                .function_tokens
                .last()
                .map(|token| token.text.as_str()),
            Some("USR")
        );
    }
}
