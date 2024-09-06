use lang::compile::compile;
use lang::language::*;

use lang::args::{process_cli, OutputFormat};

use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Read, Write};
use std::path::Path;

const LOADED_PROGRAM_OFFSET: u32 = 0x0;

fn main() -> Result<(), String> {
    let args = process_cli(&env::args().collect::<Vec<_>>())
        .map_err(|x| format!("processing cli: {x}"))?;
    if !args.validate() {
        println!("{}", args.usage());
        return Ok(());
    }

    let target_file = args.target_files.first().unwrap();
    let mut reader: Box<dyn Read> = match target_file.as_ref() {
        "-" => Box::new(stdin()),
        _ => Box::new(
            File::open(Path::new(&target_file)).map_err(|x| format!("failed to open: {}", x))?,
        ),
    };

    let mut code = Vec::new();
    reader.read_to_end(&mut code).unwrap();
    let code_str = std::str::from_utf8(&code).map_err(|_| "not utf8")?;

    match parse_ast(code_str) {
        Ok(program) => {
            let res =
                compile(program, LOADED_PROGRAM_OFFSET).map_err(|x| format!("compiling: {x:?}"))?;
            if args.output_format == OutputFormat::AnnotatedAsm {
                let mut stdout = stdout().lock();
                let offset = format!(".offsetPC {}\n", res.get_code_section_start() / 2,);
                stdout.write(offset.as_ref()).map_err(|x| format!("{x}"))?;
                let symbol_defs = res
                    .symbols
                    .iter()
                    .map(|(k, v)| format!(".defvar {k} {v}\n"))
                    .collect::<Vec<_>>()
                    .join("");
                stdout
                    .write_all(symbol_defs.as_bytes())
                    .map_err(|x| format!("{x}"))?;
                let instructions_txt = res
                    .get_lines_unresolved()
                    .map_err(|x| format!("{x:?}"))?
                    .join("\n");
                stdout
                    .write_all(instructions_txt.as_bytes())
                    .map_err(|x| format!("{x}"))?
            } else {
                let bin = res.to_binary()?;
                let mut output: Vec<u8> = Vec::new();
                bin.to_bytes(&mut output);
                let mut stdout = stdout().lock();
                stdout.write_all(&output).map_err(|x| format!("{}", x))?;
            }
        }
        Err(e) => println!("compiler error:\n{}", e),
    };
    Ok(())
}
