//! One family-owned project scaffold shared by desktop and browser creation.
/// Default project agent guidance.
pub const AGENTS: &str = include_str!("../../../templates/project-AGENTS.md");
/// Default project walkthrough.
pub const README: &str = include_str!("../../../templates/project-README.md");
/// Default verification folder guidance.
pub const TESTS_README: &str = include_str!("../../../templates/project-tests-README.md");
/// Generic Claude guidance shared across projects.
pub const CLAUDE: &str = include_str!("../../../../../templates/project-CLAUDE.md");
/// Default generated-output exclusions.
pub const GITIGNORE: &str =
    "# Eightbit Studio 6809 generated output\n/.dragon/build/\n\n# macOS metadata\n.DS_Store\n";
/// Default BASIC entry path.
pub const BASIC_ENTRY: &str = "src/basic/main.bas";
/// Generate the existing desktop manifest for a validated portable project name.
pub fn manifest(name: &str, mixed: bool) -> Result<String, super::ProjectError> {
    if !super::is_portable_name(name, 64) {
        return Err(super::ProjectError::new(
            "DSP1007",
            "invalid portable project name",
        ));
    }
    let assembler = if mixed {
        "enabled = true\nsources = [\"src/asm/main.asm\"]\noutput_format = \"raw\"\noutput_name = \"machine.bin\"\nload_address = 28672\nentry_symbol = \"START\""
    } else {
        "enabled = false\nsources = []"
    };
    Ok(format!(
        r#"schema_version = 3

[project]
name = "{name}"

[machine]
model = "dragon32"
ram_kib = 32
tv = "pal"
rom_profile = "dragon32-standard"

[basic]
dialect = "dragon32-basic"
entry = "src/basic/main.bas"
sources = ["src/basic/main.bas"]
entry_line = 10
source_encoding = "ascii"
line_numbers = "required"

[build]
profile = "debug"
output_dir = ".dragon/build"
basic_media = "cas"

[run]
adapter = "xroar"
autorun = true
speed = "normal"

[debug]
mode = "basic"
break_on_runtime_error = false

[assembler]
{assembler}
"#
    ))
}
