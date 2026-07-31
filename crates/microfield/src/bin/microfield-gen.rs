//! Deterministic field specification and certification command.

#![forbid(unsafe_code)]

use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
};

use microfield::spec::{
    FileSystemArtifactSink, Generator, JsonFileOracle, SageOracle, error::PipelineError,
};
use serde::Serialize;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let json = arguments.iter().any(|argument| argument == "--json");
    match run(arguments) {
        Ok(outcome) => outcome,
        Err(error) => {
            if json {
                let escaped = serde_json::to_string(&error.to_string())
                    .unwrap_or_else(|_| "\"unserializable error\"".to_owned());
                eprintln!("{{\"ok\":false,\"error\":{escaped}}}");
            } else {
                eprintln!("microfield-gen: {error}");
            }
            ExitCode::from(2)
        }
    }
}

fn run(mut arguments: Vec<String>) -> Result<ExitCode, CliError> {
    if arguments.is_empty()
        || take_flag(&mut arguments, "-h")
        || take_flag(&mut arguments, "--help")
    {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
    if take_flag(&mut arguments, "-V") || take_flag(&mut arguments, "--version") {
        println!("microfield-gen {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    let json = take_flag(&mut arguments, "--json");
    let command = arguments.remove(0);
    let generator = Generator::default();
    match command.as_str() {
        "normalize" => command_normalize(generator, &arguments, json),
        "validate" | "certify" => {
            command_validate(generator, &arguments, json || command == "certify")
        }
        "plan" => command_plan(generator, &arguments),
        "emit" | "all" => command_emit(generator, arguments, json),
        "check" => command_check(generator, arguments, json),
        "vectors" => command_vectors(generator, arguments),
        _ => Err(CliError::Usage(format!("unknown command `{command}`"))),
    }
}

fn command_normalize(
    generator: Generator,
    arguments: &[String],
    json: bool,
) -> Result<ExitCode, CliError> {
    let normalized = generator.normalize(one_manifest(arguments)?)?;
    if json {
        println!("{}", normalized.identity_json());
    } else {
        print!("{}", normalized.canonical_toml());
    }
    Ok(ExitCode::SUCCESS)
}

fn command_validate(
    generator: Generator,
    arguments: &[String],
    json: bool,
) -> Result<ExitCode, CliError> {
    let validated = generator.validate(one_manifest(arguments)?)?;
    if json {
        print_json(validated.certificate())?;
    } else {
        println!(
            "valid {} {}",
            validated.normalized().name(),
            validated.field_id()
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn command_plan(generator: Generator, arguments: &[String]) -> Result<ExitCode, CliError> {
    let validated = generator.validate(one_manifest(arguments)?)?;
    print_json(&generator.plan(&validated)?)?;
    Ok(ExitCode::SUCCESS)
}

fn command_emit(
    generator: Generator,
    mut arguments: Vec<String>,
    json: bool,
) -> Result<ExitCode, CliError> {
    let output = take_value(&mut arguments, "--out")?;
    let sink = FileSystemArtifactSink::new(output);
    let publication = generator.emit(one_manifest(&arguments)?, &sink)?;
    if json {
        print_json(&PublicationOutput {
            ok: true,
            output_directory: publication.output_directory(),
            replaced_existing: publication.replaced_existing(),
        })?;
    } else {
        println!("{}", publication.output_directory().display());
    }
    Ok(ExitCode::SUCCESS)
}

fn command_check(
    generator: Generator,
    mut arguments: Vec<String>,
    json: bool,
) -> Result<ExitCode, CliError> {
    let output = take_value(&mut arguments, "--out")?;
    let sink = FileSystemArtifactSink::new(output);
    let matches = generator.check(one_manifest(&arguments)?, &sink)?;
    if json {
        println!("{{\"ok\":true,\"matches\":{matches}}}");
    } else if matches {
        println!("artifacts are reproducible and current");
    } else {
        println!("artifacts differ from a clean regeneration");
    }
    Ok(if matches {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn command_vectors(generator: Generator, mut arguments: Vec<String>) -> Result<ExitCode, CliError> {
    let imported = take_optional_value(&mut arguments, "--oracle-json")?;
    let sage_executable = take_optional_value(&mut arguments, "--sage")?;
    let sage_script = take_optional_value(&mut arguments, "--sage-script")?;
    let output = take_optional_value(&mut arguments, "--out")?;
    let manifest = one_manifest(&arguments)?;
    let vectors = match (imported, sage_executable) {
        (Some(path), None) => generator.vectors(manifest, &JsonFileOracle::new(path))?,
        (None, executable) => {
            let executable = executable.unwrap_or_else(|| PathBuf::from("sage"));
            let script = sage_script.unwrap_or_else(default_sage_script);
            generator.vectors(manifest, &SageOracle::new(executable, script))?
        }
        (Some(_), Some(_)) => {
            return Err(CliError::Usage(
                "choose either --oracle-json or --sage, not both".to_owned(),
            ));
        }
    };
    if let Some(path) = output {
        write_json(&path, &vectors)?;
        println!("{}", path.display());
    } else {
        print_json(&vectors)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn one_manifest(arguments: &[String]) -> Result<PathBuf, CliError> {
    match arguments {
        [manifest] => Ok(PathBuf::from(manifest)),
        _ => Err(CliError::Usage(
            "exactly one manifest path is required".to_owned(),
        )),
    }
}

fn take_flag(arguments: &mut Vec<String>, flag: &str) -> bool {
    if let Some(index) = arguments.iter().position(|argument| argument == flag) {
        arguments.remove(index);
        true
    } else {
        false
    }
}

fn take_value(arguments: &mut Vec<String>, option: &str) -> Result<PathBuf, CliError> {
    take_optional_value(arguments, option)?
        .ok_or_else(|| CliError::Usage(format!("missing required option `{option} PATH`")))
}

fn take_optional_value(
    arguments: &mut Vec<String>,
    option: &str,
) -> Result<Option<PathBuf>, CliError> {
    let Some(index) = arguments.iter().position(|argument| argument == option) else {
        return Ok(None);
    };
    if index + 1 >= arguments.len() {
        return Err(CliError::Usage(format!(
            "option `{option}` requires a path"
        )));
    }
    arguments.remove(index);
    Ok(Some(PathBuf::from(arguments.remove(index))))
}

fn default_sage_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join("sage")
        .join("generate_vectors.sage")
}

fn print_json(value: &impl Serialize) -> Result<(), CliError> {
    let output =
        serde_json::to_string_pretty(value).map_err(|error| CliError::Output(error.to_string()))?;
    println!("{output}");
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), CliError> {
    let mut output =
        serde_json::to_vec_pretty(value).map_err(|error| CliError::Output(error.to_string()))?;
    output.push(b'\n');
    std::fs::write(path, output).map_err(|error| {
        CliError::Output(format!(
            "cannot write JSON output {}: {error}",
            path.display()
        ))
    })
}

fn print_help() {
    println!(
        "microfield-gen {version}\n\n\
         Usage:\n  \
         microfield-gen normalize MANIFEST [--json]\n  \
         microfield-gen validate MANIFEST [--json]\n  \
         microfield-gen certify MANIFEST\n  \
         microfield-gen plan MANIFEST\n  \
         microfield-gen emit MANIFEST --out DIRECTORY [--json]\n  \
         microfield-gen check MANIFEST --out DIRECTORY [--json]\n  \
         microfield-gen vectors MANIFEST [--oracle-json FILE | --sage PATH] [--out FILE]\n  \
         microfield-gen all MANIFEST --out DIRECTORY [--json]\n",
        version = env!("CARGO_PKG_VERSION")
    );
}

#[derive(Serialize)]
struct PublicationOutput<'a> {
    ok: bool,
    output_directory: &'a Path,
    replaced_existing: bool,
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Pipeline(PipelineError),
    Output(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Output(message) => formatter.write_str(message),
            Self::Pipeline(error) => error.fmt(formatter),
        }
    }
}

impl From<PipelineError> for CliError {
    fn from(error: PipelineError) -> Self {
        Self::Pipeline(error)
    }
}
