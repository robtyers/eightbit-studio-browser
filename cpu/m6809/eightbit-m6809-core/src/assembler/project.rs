//! Deterministic in-memory linking of explicitly supplied source units.
use super::{
    AssemblyError, Line, MAX_LINES, MAX_PASSES, MAX_SOURCE_BYTES, Pass, emit, error, identifier,
    parse, split_operands,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_SOURCES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct AssemblySource<'a> {
    pub source_uri: &'a str,
    pub source: &'a str,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedSymbol {
    pub source_uri: String,
    pub name: String,
    pub address: u16,
    pub exported: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedLine {
    pub source_uri: String,
    pub source_line: usize,
    pub address: u16,
    pub bytes: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedAssembly {
    pub load_address: u16,
    pub entry_address: u16,
    pub bytes: Vec<u8>,
    pub symbols: Vec<LinkedSymbol>,
    pub lines: Vec<LinkedLine>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedAssemblyError {
    pub source_uri: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}
impl std::fmt::Display for LinkedAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DMA1001: {}:{}:{}: {}",
            self.source_uri, self.line, self.column, self.message
        )
    }
}
impl std::error::Error for LinkedAssemblyError {}
fn located(source: &str, error: AssemblyError) -> LinkedAssemblyError {
    LinkedAssemblyError {
        source_uri: source.to_owned(),
        line: error.line,
        column: error.column,
        message: error.message,
    }
}
fn valid_identity(uri: &str) -> bool {
    !uri.is_empty() && uri.len() <= 1024 && !uri.chars().any(char::is_control)
}
struct Unit<'a> {
    uri: &'a str,
    lines: Vec<Line>,
    exports: BTreeMap<String, usize>,
    imports: BTreeMap<String, usize>,
    definitions: BTreeSet<String>,
}
impl<'a> Unit<'a> {
    fn parse(source: &AssemblySource<'a>) -> Result<Self, LinkedAssemblyError> {
        let lines = parse(source.source).map_err(|error| located(source.source_uri, error))?;
        let definitions = lines
            .iter()
            .filter_map(|line| line.label.clone())
            .collect::<BTreeSet<_>>();
        let mut exports = BTreeMap::new();
        let mut imports = BTreeMap::new();
        for line in &lines {
            let destination = match line.operation.as_str() {
                "EXPORT" => &mut exports,
                "EXTERN" => &mut imports,
                _ => continue,
            };
            for name in split_operands(&line.operand, line.number)
                .map_err(|error| located(source.source_uri, error))?
            {
                if !identifier(name) {
                    return Err(located(
                        source.source_uri,
                        error(line.number, "EXPORT/EXTERN requires symbol names"),
                    ));
                }
                destination.insert(name.to_ascii_uppercase(), line.number);
            }
        }
        for (name, line) in &exports {
            if !definitions.contains(name) {
                return Err(located(
                    source.source_uri,
                    error(
                        *line,
                        format!("Exported symbol {name} is not defined in this source"),
                    ),
                ));
            }
        }
        for (name, line) in &imports {
            if definitions.contains(name) {
                return Err(located(
                    source.source_uri,
                    error(
                        *line,
                        format!("External symbol {name} is also locally defined"),
                    ),
                ));
            }
        }
        Ok(Self {
            uri: source.source_uri,
            lines,
            exports,
            imports,
            definitions,
        })
    }
}

/// Link explicit source units in their supplied order. Locals belong to one
/// unit; cross-unit references require EXTERN and exactly one matching EXPORT.
/// There is no filesystem access, include discovery or external object format.
pub fn assemble_project(
    sources: &[AssemblySource<'_>],
    load_address: u16,
    entry_symbol: &str,
) -> Result<LinkedAssembly, LinkedAssemblyError> {
    let origin = sources
        .first()
        .filter(|source| valid_identity(source.source_uri))
        .map_or("", |source| source.source_uri);
    let failure = |message: &str| located(origin, error(1, message));
    if sources.is_empty() || sources.len() > MAX_SOURCES {
        return Err(failure("Supply 1 to 64 explicit source units"));
    }
    let mut identities = BTreeSet::new();
    let mut total_bytes = 0usize;
    let mut total_lines = 0usize;
    for source in sources {
        if !valid_identity(source.source_uri) {
            // An invalid identity must not put control characters or an
            // unbounded supplied string back into the diagnostic envelope.
            return Err(located(
                "",
                error(1, "Source identity is missing or invalid"),
            ));
        }
        if !identities.insert(source.source_uri) {
            return Err(located(
                source.source_uri,
                error(1, "Source identity is duplicated"),
            ));
        }
        total_bytes = total_bytes
            .checked_add(source.source.len())
            .ok_or_else(|| failure("Project source is too large"))?;
        if total_bytes > MAX_SOURCE_BYTES {
            return Err(failure("Project source exceeds 1 MiB"));
        }
        total_lines = total_lines
            .checked_add(source.source.lines().count())
            .ok_or_else(|| failure("Project source has too many lines"))?;
        if total_lines > MAX_LINES {
            return Err(failure("Project source exceeds 1 MiB or 32768 lines"));
        }
    }
    if !identifier(entry_symbol) {
        return Err(failure("Invalid entry symbol"));
    }
    let entry = entry_symbol.to_ascii_uppercase();
    let units = sources
        .iter()
        .map(Unit::parse)
        .collect::<Result<Vec<_>, _>>()?;
    let mut providers = BTreeMap::new();
    for (index, unit) in units.iter().enumerate() {
        for (name, line) in &unit.exports {
            if providers.insert(name.clone(), index).is_some() {
                return Err(located(
                    unit.uri,
                    error(
                        *line,
                        format!("Exported symbol {name} has more than one provider"),
                    ),
                ));
            }
        }
    }
    for unit in &units {
        for (name, line) in &unit.imports {
            if !providers.contains_key(name) {
                return Err(located(
                    unit.uri,
                    error(
                        *line,
                        format!("External symbol {name} has no exported provider"),
                    ),
                ));
            }
        }
    }
    let entry_unit = if let Some(index) = providers.get(&entry) {
        *index
    } else {
        let matches = units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.definitions.contains(&entry))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [index] = matches.as_slice() else {
            return Err(failure(
                "Entry must have one exported or unique local definition",
            ));
        };
        *index
    };
    let mut previous = (0..units.len())
        .map(|_| BTreeMap::new())
        .collect::<Vec<_>>();
    let mut sizes = Vec::new();
    for _ in 0..MAX_PASSES {
        let passes = layout(&units, &providers, &previous, load_address, &entry, false)?;
        let symbols = passes
            .iter()
            .map(|pass| pass.symbols.clone())
            .collect::<Vec<_>>();
        let next_sizes = passes
            .iter()
            .map(|pass| pass.sizes.clone())
            .collect::<Vec<_>>();
        if symbols == previous && next_sizes == sizes {
            let passes = layout(&units, &providers, &previous, load_address, &entry, true)?;
            let entry_value = passes[entry_unit]
                .symbols
                .get(&entry)
                .copied()
                .ok_or_else(|| failure("Entry is not resolved"))?;
            let entry_address = u16::try_from(entry_value)
                .map_err(|_| failure("Entry is outside the 16-bit address space"))?;
            let mut result = LinkedAssembly {
                load_address,
                entry_address,
                bytes: Vec::new(),
                symbols: Vec::new(),
                lines: Vec::new(),
            };
            for (unit, pass) in units.iter().zip(passes) {
                result.bytes.extend(pass.bytes);
                result
                    .symbols
                    .extend(pass.symbols.into_iter().filter_map(|(name, value)| {
                        u16::try_from(value).ok().map(|address| LinkedSymbol {
                            source_uri: unit.uri.to_owned(),
                            exported: unit.exports.contains_key(&name),
                            name,
                            address,
                        })
                    }));
                result
                    .lines
                    .extend(pass.lines.into_iter().map(|line| LinkedLine {
                        source_uri: unit.uri.to_owned(),
                        source_line: line.source_line,
                        address: line.address,
                        bytes: line.bytes,
                    }));
            }
            if entry_address < load_address
                || usize::from(entry_address - load_address) >= result.bytes.len()
            {
                return Err(failure("Entry must lie inside the assembled image"));
            }
            return Ok(result);
        }
        previous = symbols;
        sizes = next_sizes;
    }
    Err(failure(
        "Assembly addresses did not stabilise; use explicit operand sizes or remove circular constants",
    ))
}
fn layout(
    units: &[Unit<'_>],
    providers: &BTreeMap<String, usize>,
    previous: &[BTreeMap<String, i64>],
    load: u16,
    entry: &str,
    strict: bool,
) -> Result<Vec<Pass>, LinkedAssemblyError> {
    let mut address = u32::from(load);
    let mut passes = Vec::with_capacity(units.len());
    for (index, unit) in units.iter().enumerate() {
        let external = unit
            .imports
            .keys()
            .filter_map(|name| {
                providers
                    .get(name)
                    .and_then(|provider| previous[*provider].get(name))
                    .map(|value| (name.clone(), *value))
            })
            .collect::<BTreeMap<_, _>>();
        let pass = emit(
            &unit.lines,
            address,
            &previous[index],
            strict,
            entry,
            Some(&external),
        )
        .map_err(|error| located(unit.uri, error))?;
        address += u32::try_from(pass.bytes.len())
            .map_err(|_| located(unit.uri, error(1, "Image too large")))?;
        passes.push(pass);
    }
    Ok(passes)
}
