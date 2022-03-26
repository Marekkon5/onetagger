use std::cmp::Ordering;

use pulldown_cmark::{Parser, Options};
use serde::{Serialize, Deserialize};

use crate::docs::{VARIABLES, FUNCTIONS, PROPERTIES};

/// Separate parser for autocomplete
pub struct Autocomplete {
    state: AutocompleteState,
    buffer: String,
}

impl Autocomplete {
    /// Parse the format
    pub fn parse(input: &str) -> Autocomplete {
        let mut output = String::new();

        // States
        let mut command = false;
        let mut escape = false;
        let mut string = false;
        let mut dot_count = 0;
        let mut suggest = false;

        for c in input.chars() {
            match c {
                // Command start/stop
                '%' if !string => {
                    command = !command;
                    suggest = command;
                    dot_count = 0;
                    output.clear();
                },
                // String escapes
                '\\' if command => escape = !escape,
                // String
                '"' if command && !escape => {
                    string = !string;
                    dot_count = 0;
                    output.clear();
                },
                // Params & functions
                '.' if command && !string => {
                    output.clear();
                    dot_count += 1;
                    suggest = true;
                }
                // Parameters
                '(' if command && !string => {
                    suggest = false;
                },
                ')' if command && !string => {
                    output.clear();
                }
                _ => {
                    output.push(c);
                    escape = false;
                }
            }
        }

        // Don't suggest
        if !command || string || !suggest {
            return Autocomplete { 
                state: AutocompleteState::None,
                buffer: String::new()
            }
        }
        // Which part of command
        let state = match dot_count {
            0 => AutocompleteState::Variable,
            1 => AutocompleteState::Property,
            _ => AutocompleteState::Function
        };
        Autocomplete { state, buffer: output }
    }

    /// Generate suggestions
    pub fn suggest(&self) -> Vec<SymbolDoc> {
        if self.state == AutocompleteState::None {
            return vec![];
        }

        let text = self.buffer.to_lowercase();
        let mut matches = match self.state {
            AutocompleteState::Variable => VARIABLES.iter().filter(|d| d.name.starts_with(&text)).collect::<Vec<&SymbolDoc>>(),
            AutocompleteState::Property => PROPERTIES.iter()
                .filter(|d| d.name.starts_with(&text))
                .chain(FUNCTIONS.iter().filter(|d| d.name.starts_with(&text)))
                .collect::<Vec<&SymbolDoc>>(),
            AutocompleteState::Function => FUNCTIONS.iter().filter(|d| d.name.starts_with(&text)).collect::<Vec<&SymbolDoc>>(),
            AutocompleteState::None => unreachable!(),
        };
        // Properties have bigger priority over functions, then sort by length or name
        matches.sort_by(|a, b| {
            if a.kind == DocSymbolType::Property && b.kind == DocSymbolType::Function {
                return Ordering::Less;
            }
            if a.kind == DocSymbolType::Function && b.kind == DocSymbolType::Property {
                return Ordering::Greater;
            }
            if text.trim().is_empty() {
                a.name.cmp(&b.name)
            } else {
                a.name.len().cmp(&b.name.len())
            }
        });
        // Only 5 suggestions max
        matches.truncate(5);
        matches.into_iter().map(|m| m.clone()).collect::<Vec<_>>()
    }

    /// Generate suggestions and convert to html
    pub fn suggest_html(&self) -> Vec<SymbolDoc> {
        let mut suggestions = self.suggest();
        for s in &mut suggestions {
            let parser = Parser::new_ext(&s.doc, Options::empty());
            let mut output = String::new();
            pulldown_cmark::html::push_html(&mut output, parser);
            s.doc = output;
        }
        suggestions
    }

    /// How many characters are already present
    pub fn suggestion_offset(&self) -> usize {
        self.buffer.len()
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutocompleteState {
    None,
    Variable, 
    Property, 
    Function
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDoc {
    pub name: String,
    /// In markdown format
    pub doc: String,
    pub kind: DocSymbolType,
    pub parameters: Vec<DocParameter>
}

impl SymbolDoc {
    /// Short for creating new variable doc 
    pub(crate) fn var(name: &str, doc: &str) -> SymbolDoc {
        SymbolDoc {
            name: name.to_string(),
            doc: doc.to_string(),
            kind: DocSymbolType::Variable,
            parameters: vec![],
        }
    }

    /// Short for creating new property doc
    pub(crate) fn prop(name: &str, doc: &str) -> SymbolDoc {
        SymbolDoc {
            name: name.to_string(),
            doc: doc.to_string(),
            kind: DocSymbolType::Property,
            parameters: vec![],
        }
    }

    // Short for creating new fn doc
    pub(crate) fn f(name: &str, doc: &str, parameters: Vec<DocParameter>) -> SymbolDoc {
        SymbolDoc {
            name: name.to_string(),
            doc: doc.to_string(),
            kind: DocSymbolType::Function,
            parameters,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DocSymbolType {
    Variable,
    Property,
    Function
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum DocParameter {
    Number { name: String, required: bool },
    String { name: String, required: bool }
}

impl DocParameter {
    /// Quick number creation
    pub(crate) fn n(name: &str, required: bool) -> DocParameter {
        DocParameter::Number { name: name.to_string(), required }
    }

    /// Quick string param creation
    pub(crate) fn s(name: &str, required: bool) -> DocParameter {
        DocParameter::String { name: name.to_string(), required }
    }
}

