use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::binfmt::SectionMode;
use crate::resolve::UnresolvedInstruction;
use crate::{Instruction, Register};

pub mod macros;

pub enum Error {
    UnknownToken(String),
    MacroEval(String, String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownToken(s) => write!(f, "unknown token: {}", s),
            Error::MacroEval(name, err) => write!(f, "eval macro {}: {}", name, err),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

type MacroFunc = fn(&mut PreProcessor, input: Vec<&str>) -> Result<Vec<String>, String>;

#[derive(Debug)]
pub enum Macro {
    Func(MacroFunc),
    Subst(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Variable {
    Label(String),
    User(String),
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Label(s) => write!(f, "{s}"),
            Self::User(s) => write!(f, "{s}"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Data<T> {
    pub offset: u32,
    pub mode: SectionMode,
    pub chunks: Vec<Chunk<T>>,
}

impl<T> Data<T> {
    fn new(offset: u32, mode: SectionMode) -> Self {
        Self {
            offset,
            mode,
            chunks: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum Chunk<T> {
    Raw(Vec<u8>),
    Lines(Vec<T>),
}

#[derive(Debug, Default)]
pub struct PreProcessor {
    entrypoint: u16,
    sections: HashMap<String, Data<ProcessedLine>>,
    heaps: Vec<(u32, u32)>,
    pub variables: HashMap<String, Variable>,
    pub labels: HashMap<String, u32>,
    macros: HashMap<String, Macro>,
    active_section: Option<String>,
}

// TODO(phy1um): use unresolvedinstrction abstraction from lang
#[derive(Debug)]
pub struct ProcessedLine {
    source_line_number: usize,
    line: Vec<ProcessedLinePart>,
}

#[allow(dead_code)]
#[derive(Debug)]
enum ProcessedLinePart {
    Body(String),
    Variable(String),
    Label(String),
}

impl ProcessedLine {
    pub fn from_str(s: &str, source_line_number: usize) -> Self {
        Self {
            source_line_number,
            line: vec![ProcessedLinePart::Body(s.to_string())],
        }
    }

    pub fn parse(parts: &[&str], source_line_number: usize) -> Self {
        let mut line = Vec::new();
        for part in parts {
            if part.is_empty() {
                continue;
            }
            if part.chars().nth(0) == Some('!') {
                line.push(ProcessedLinePart::Variable(part[1..].to_string()));
            } else {
                line.push(ProcessedLinePart::Body(part.to_string()));
            }
        }
        Self {
            source_line_number,
            line,
        }
    }

    fn label(s: &str, source_line_number: usize) -> Self {
        Self {
            source_line_number,
            line: vec![ProcessedLinePart::Label(s.to_string())],
        }
    }

    pub fn get_line_number(&self) -> usize {
        self.source_line_number
    }
}

impl PreProcessor {
    pub fn set_entrypoint(&mut self, entrypoint: u16) {
        self.entrypoint = entrypoint;
    }

    pub fn set_active_section(&mut self, name: &str) {
        self.active_section = Some(name.to_string());
    }

    pub fn create_section(&mut self, name: &str, offset: u32, kind: SectionMode) {
        self.sections
            .insert(name.to_string(), Data::new(offset, kind));
    }

    pub fn create_heap(&mut self, offset: u32, size: u32) {
        self.heaps.push((offset, size));
    }

    pub fn write_section_raw(&mut self, data: &[u8]) {
        if let Some(section_name) = self.active_section.clone() {
            self.sections
                .get_mut(&section_name)
                .unwrap()
                .chunks
                .push(Chunk::Raw(data.to_vec()));
        } else {
            todo!("handle not in section");
        }
    }

    pub fn resolve_pass2(&self, p: &ProcessedLine) -> Result<String, Error> {
        self.reprocess_line(&p.line)
    }

    pub fn define_labels(
        &mut self,
        sections: &HashMap<String, Data<UnresolvedInstruction>>,
    ) -> Result<(), Error> {
        for s in sections.values() {
            let mut head = s.offset;
            for c in &s.chunks {
                match c {
                    Chunk::Raw(v) => head += v.len() as u32,
                    Chunk::Lines(urs) => {
                        for ur in urs {
                            if let UnresolvedInstruction::Label(name) = ur {
                                self.define_label(name, head);
                            };
                            head += ur.size();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_unresolved_instructions(
        &self,
    ) -> Result<HashMap<String, Data<UnresolvedInstruction>>, Error> {
        let mut out = HashMap::new();
        for (section_name, data) in &self.sections {
            let mut new_chunks = Vec::new();
            for chunk in &data.chunks {
                match chunk {
                    Chunk::Lines(lines) => {
                        let mut res = Vec::new();
                        for line in lines.iter() {
                            if let Some(ur) = self.part_resolve_line(&line.line)? {
                                res.push(ur);
                            }
                        }
                        new_chunks.push(Chunk::Lines(res))
                    }
                    Chunk::Raw(x) => new_chunks.push(Chunk::Raw(x.to_vec())),
                }
            }
            out.insert(
                section_name.to_string(),
                Data {
                    offset: data.offset,
                    mode: data.mode,
                    chunks: new_chunks,
                },
            );
        }
        Ok(out)
    }

    fn reprocess_line(&self, p: &[ProcessedLinePart]) -> Result<String, Error> {
        let mapped: Result<Vec<String>, Error> = p
            .iter()
            .map(|part| match part {
                ProcessedLinePart::Variable(s) => self
                    .get_variable(s)
                    .map(|x| x.to_string())
                    .ok_or(Error::UnknownToken(s.to_string())),
                ProcessedLinePart::Body(s) => Ok(s.to_string()),
                ProcessedLinePart::Label(_) => todo!("handle labels"),
            })
            .collect();
        mapped.map(|x| x.join(" "))
    }

    fn try_parse_unresolved_instruction(
        first: &ProcessedLinePart,
        parts: &[ProcessedLinePart],
    ) -> Option<UnresolvedInstruction> {
        if let ProcessedLinePart::Body(head) = first {
            match head.as_str() {
                "Imm" => {
                    if parts.len() != 2 {
                        None
                    } else if let ProcessedLinePart::Body(reg_str) = parts.first().unwrap() {
                        let reg = Register::from_str(reg_str).ok()?;
                        if let ProcessedLinePart::Variable(label) = parts.get(1).unwrap() {
                            Some(UnresolvedInstruction::Imm(reg, label.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                "AddImm" => {
                    if parts.len() != 2 {
                        None
                    } else if let ProcessedLinePart::Body(reg_str) = parts.first().unwrap() {
                        let reg = Register::from_str(reg_str).ok()?;
                        if let ProcessedLinePart::Variable(label) = parts.get(1).unwrap() {
                            Some(UnresolvedInstruction::AddImm(reg, label.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                "AddImmSigned" => {
                    if parts.len() != 2 {
                        None
                    } else if let ProcessedLinePart::Body(reg_str) = parts.first().unwrap() {
                        let reg = Register::from_str(reg_str).ok()?;
                        if let ProcessedLinePart::Variable(label) = parts.get(1).unwrap() {
                            Some(UnresolvedInstruction::AddImmSigned(reg, label.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn part_resolve_line(
        &self,
        p: &[ProcessedLinePart],
    ) -> Result<Option<UnresolvedInstruction>, Error> {
        if let Some(head) = p.first() {
            match head {
                ProcessedLinePart::Body(s) => {
                    if let Some(c) = s.chars().nth(0) {
                        if c == ';' {
                            // skip comments
                            Ok(None)
                        } else if p.len() > 1 {
                            if let Some(ur) =
                                Self::try_parse_unresolved_instruction(head, p.get(1..).unwrap())
                            {
                                Ok(Some(ur))
                            } else {
                                let instruction_str = p
                                    .iter()
                                    .map(|s| match s {
                                        ProcessedLinePart::Body(s) => Ok(s.to_string()),
                                        ProcessedLinePart::Label(_) => Ok("".to_string()),
                                        ProcessedLinePart::Variable(v) => self
                                            .get_variable(v)
                                            .map(|s| s.to_string())
                                            .ok_or(Error::UnknownToken(v.to_string())),
                                    })
                                    .collect::<Result<Vec<_>, _>>()?
                                    .join(" ");
                                let ins = Instruction::from_str(&instruction_str).map_err(|e| {
                                    Error::Other(format!("invalid instruction: {e:?}"))
                                })?;
                                Ok(Some(UnresolvedInstruction::Instruction(ins)))
                            }
                        } else {
                            Ok(None)
                        }
                    } else {
                        // empty line
                        Ok(None)
                    }
                }
                ProcessedLinePart::Variable(_) => Err(Error::Other(
                    "invalid variable in first position".to_string(),
                )),
                ProcessedLinePart::Label(l) => {
                    Ok(Some(UnresolvedInstruction::Label(l.to_string())))
                }
            }
        } else {
            Ok(None)
        }
    }

    /* TODO: maybe remove
    fn push_instruction(&mut self, line: ProcessedLine) -> Result<(), Error> {
        let key = self.active_section.clone().ok_or(Error::Other("not in section".to_string()))?;
        let section = self.sections.get_mut(&key).ok_or(Error::Other(format!("no such section {key}")))?;
        let len = section.chunks.len()-1;
        if section.chunks.is_empty() {
            section.chunks.push(Chunk::Lines(vec![line]));
        } else {
            if let Chunk::Lines(ref mut v) = section.chunks.get_mut(len).unwrap() {
                v.push(line);
            } else {
                section.chunks.push(Chunk::Lines(vec![line]));
            }
        };
        Ok(())
    }
    */

    fn resolve_line(
        &mut self,
        line: &str,
        line_number: usize,
    ) -> Result<Vec<ProcessedLine>, Error> {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(Vec::new());
        }
        let head = parts.first().unwrap();
        match head.chars().nth(0) {
            Some(';') => Ok(vec![ProcessedLine::from_str(line, line_number)]),
            Some('.') => {
                let name = &head[1..];
                let func = self
                    .get_macro(name)
                    .ok_or(Error::UnknownToken(head[1..].to_string()))?;
                let macro_res = match func {
                    Macro::Func(f) => f(self, parts[1..].to_vec())
                        .map_err(|x| Error::MacroEval(name.to_string(), x))?,
                    Macro::Subst(lines) => {
                        lines
                            .iter()
                            .map(|line| {
                                let mp: Result<Vec<String>, String> = line
                                    .split(' ')
                                    .map(|p| {
                                        match p.chars().nth(0) {
                                            Some('!') => {
                                                match p[1..].parse::<u32>() {
                                                    Ok(n) => parts
                                                        .get((n + 1) as usize)
                                                        .ok_or(format!(
                                                            "subst {}: out of bounds",
                                                            p
                                                        ))
                                                        .map(|x| x.to_string()),
                                                    Err(_) => {
                                                        Ok(p.to_string())
                                                        // Err(format!("parse {}: {}", p, e))
                                                    }
                                                }
                                            }
                                            _ => Ok(p.to_string()),
                                        }
                                    })
                                    .collect();
                                // TODO: handle error here
                                match mp {
                                    Ok(s) => s.join(" "),
                                    Err(e) => format!("err: {}", e),
                                }
                            })
                            .collect()
                    }
                };
                let b: Result<Vec<Vec<ProcessedLine>>, Error> = macro_res
                    .iter()
                    .map(|x| self.resolve_line(x, line_number))
                    .collect();
                let mut out = Vec::new();
                for proc in b? {
                    out.extend(proc)
                }
                Ok(out)
            }
            Some(':') => Ok(vec![ProcessedLine::label(&head[1..], line_number)]),
            _ => Ok(vec![ProcessedLine::parse(&parts, line_number)]),
        }
    }

    pub fn handle(&mut self, input: &str) -> Result<(), Error> {
        for (i, line) in input.lines().enumerate() {
            let resolved = self.resolve_line(line, i)?;
            if !resolved.is_empty() {
                if let Some(section_name) = self.active_section.clone() {
                    let chunks = &mut self.sections.get_mut(&section_name).unwrap().chunks;
                    chunks.push(Chunk::Lines(resolved));
                }
            }
        }
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<Variable> {
        self.variables.get(name).cloned()
    }

    pub fn define_label(&mut self, name: &str, value: u32) {
        self.labels.insert(name.to_string(), value);
    }

    pub fn define_user_variable(&mut self, name: &str, value: &str) {
        self.variables
            .insert(name.to_string(), Variable::User(value.to_string()));
    }

    fn get_macro(&mut self, name: &str) -> Option<&Macro> {
        self.macros.get(name)
    }

    pub fn define_macro(&mut self, name: &str, value: MacroFunc) {
        self.macros.insert(name.to_string(), Macro::Func(value));
    }

    pub fn define_subst_macro(&mut self, name: &str, value: Vec<String>) {
        self.macros.insert(name.to_string(), Macro::Subst(value));
    }
}
