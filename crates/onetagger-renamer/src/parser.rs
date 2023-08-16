use std::path::Path;
use onetagger_tag::Tag;
use onetagger_tagger::{AudioFileInfo, Field};
use pad::{PadStr, Alignment};
use regex::Regex;

use crate::RenamerConfig;

/// Illegal filename characters
static ILLEGAL_FILENAME: &'static str = "<>:\"/\\|?*\0";

#[derive(Debug, Clone)]
pub struct TemplateParser {
    tokens: Vec<TokenType>,
    pub syntax: Vec<SyntaxData>,
}

impl TemplateParser {
    /// Apply template
    pub fn evaluate(&mut self, info: &AudioFileInfo, config: &RenamerConfig) -> String {
        let mut output = String::new();
        for token in &self.tokens {
            if let Some(data) = token.token().get_value(None, info, config) {
                match token {
                    // Do not sanitize constants
                    TokenType::Constant(_) => output.push_str(&data.to_string(&config.separator)),
                    _ => output.push_str(&data.sanitize().to_string(&config.separator))
                }
            }
        }
        output
    }


    /// Parse the template
    pub fn parse(input: &str) -> TemplateParser {
        let mut buffer = String::new();
        let mut command = false;
        let mut tokens = vec![];
        let mut syntax = SyntaxBuilder::new();
        let mut escape = false;
        let mut string = false;

        for c in input.chars() {
            match c {
                '%' => {
                    // End of command
                    if command {
                        if !string {
                            tokens.push(TokenType::Command(TokenCommand::parse(&buffer, &mut syntax)));
                            syntax.add(1, SyntaxType::Operator);
                            buffer.clear();
                            command = false;
                            continue;
                        }
                    // Start command
                    } else {
                        if !buffer.is_empty() {
                            tokens.push(TokenType::Constant(TokenConstant::new(&buffer)));
                            syntax.add(buffer.len(), SyntaxType::Text);
                            buffer.clear();
                        }
                        syntax.add(1, SyntaxType::Operator);
                        command = true;
                        continue;
                    }
                },
                '\\' if command => {
                    escape = true;
                },
                '"' if command => {
                    if escape {
                        escape = false;
                    } else {
                        string = !string;
                    }
                },
                _ => {
                    escape = false;
                }
            }
            buffer.push(c);
        }

        // Leftover
        if !buffer.is_empty() {
            tokens.push(TokenType::Constant(TokenConstant::new(&buffer)));
            syntax.add(buffer.len(), SyntaxType::Text);
        }
        TemplateParser { tokens, syntax: syntax.build() }
    }
}

/// For syntax highlighting
#[derive(Debug, Clone)]
pub struct SyntaxData {
    pub start: usize,
    pub length: usize,
    pub syntax: SyntaxType
}

impl SyntaxData {
    /// Create new instance
    pub fn new(start: usize, length: usize, syntax: SyntaxType) -> SyntaxData {
        SyntaxData { start, length, syntax }
    }
}

#[derive(Debug, Clone)]
pub enum SyntaxType {
    /// Outside command text
    Text,
    /// Text inside command
    String,
    /// Constant number
    Number,
    /// Function name
    Function,
    /// . ( ) %
    Operator,
    /// Property name
    Property,
    /// Variable name
    Variable
}

struct SyntaxBuilder {
    data: Vec<SyntaxData>,
    index: usize
}

impl SyntaxBuilder {
    /// Create new empty instance
    pub fn new() -> SyntaxBuilder {
        SyntaxBuilder { data: vec![], index: 0 }
    }

    /// Add new item
    pub fn add(&mut self, length: usize, syntax: SyntaxType) {
        self.data.push(SyntaxData::new(self.index, length, syntax));
        self.index += length;
    }

    /// Create syntax data
    pub fn build(self) -> Vec<SyntaxData> {
        self.data
    }
}

#[derive(Debug, Clone)]
enum Data {
    String(String),
    Array(Vec<String>)
}

impl Data {
    /// Convert any to string
    pub fn to_string(&self, separator: &str) -> String {
        match self {
            Data::String(s) => s.to_string(),
            Data::Array(a) => a.join(separator),
        }
    }

    /// Sanitize illegal filename characters
    pub fn sanitize(self) -> Self {
        // Sanitize string
        let sanitize = |mut s: String| {
            for c in ILLEGAL_FILENAME.chars() {
                s = s.replace(c, "");
            }
            s
        };

        match self {
            Data::String(s) => Data::String(sanitize(s)),
            Data::Array(a) => Data::Array(a.into_iter().map(|s| sanitize(s)).collect()),
        }
    }
}

/// Every token type should implement this
trait Token {
    /// Evaluate the token
    fn get_value(&self, input: Option<&Data>, info: &AudioFileInfo, config: &RenamerConfig) -> Option<Data>;
}


/// Wrapper for all token types
#[derive(Debug, Clone)]
enum TokenType {
    Command(TokenCommand),
    Constant(TokenConstant),
    Variable(TokenVariable),
    Property(TokenProperty),
    Function(TokenFunction)
}

impl TokenType {
    /// Into `Token` trait
    pub fn token(&self) -> &dyn Token {
        match self {
            TokenType::Command(t) => t,
            TokenType::Constant(t) => t,
            TokenType::Variable(t) => t,
            TokenType::Property(t) => t,
            TokenType::Function(t) => t,
        }
    }
}



/// Token with everything inside % %
#[derive(Debug, Clone)]
struct TokenCommand {
    tokens: Vec<TokenType>
}

impl TokenCommand {
    fn parse(input: &str, syntax: &mut SyntaxBuilder) -> TokenCommand {
        let mut buffer = String::new();
        let mut was_variable = false;
        let mut tokens = vec![];
        let mut escape = false;
        let mut function = false;
        let mut string = false;

        for c in input.chars() {
            match c {
                '.' if !string && !function => {
                    if buffer.is_empty() {
                        syntax.add(1, SyntaxType::Operator);
                        continue;
                    }

                    // Initial variable
                    if !was_variable {
                        was_variable = true;
                        tokens.push(TokenType::Variable(TokenVariable::new(&buffer)));
                        syntax.add(buffer.len(), SyntaxType::Variable);
                        syntax.add(1, SyntaxType::Operator);
                        buffer.clear();
                        continue;
                    // Property
                    } else {
                        tokens.push(TokenType::Property(TokenProperty::new(&buffer)));
                        syntax.add(buffer.len(), SyntaxType::Property);
                        syntax.add(1, SyntaxType::Operator);
                        buffer.clear();
                        continue;
                    }

                },
                '(' if !string => function = true,
                // Function end
                ')' if !string => {
                    buffer.push(c);
                    match TokenFunction::parse(&buffer, syntax) {
                        Some(f) => tokens.push(TokenType::Function(f)),
                        None => {
                            error!("Failed to parse function, will be empty: {buffer}");
                        },
                    }
                    buffer.clear();
                    function = false;
                    continue;
                },
                '\\' if !function => escape = true,
                // Constant
                '"' if !function => {
                    // Escaped ""
                    if escape {
                        escape = false;
                        buffer.push('"');
                        continue;
                    }

                    // End of string
                    if !buffer.is_empty() {
                        if !string {
                            // End of variable
                            if !was_variable {
                                tokens.push(TokenType::Variable(TokenVariable::new(&buffer)));
                                syntax.add(buffer.len(), SyntaxType::Variable);
                                buffer.clear();
                                string = true;
                                was_variable = true;
                                syntax.add(1, SyntaxType::Operator);
                                continue;
                            }
                            // End of property
                            tokens.push(TokenType::Property(TokenProperty::new(&buffer)));
                            syntax.add(buffer.len(), SyntaxType::Property);
                            buffer.clear();
                            string = true;
                            syntax.add(1, SyntaxType::Operator);
                            continue;
                        }

                        // End of string
                        tokens.push(TokenType::Constant(TokenConstant::new(&buffer)));
                        syntax.add(buffer.len(), SyntaxType::String);
                        buffer.clear();
                        string = false;
                        syntax.add(1, SyntaxType::Operator);
                        continue;
                    }
                    // Start of string
                    syntax.add(1, SyntaxType::Operator);
                    string = true;
                    continue;
                },
                _ => {}
            }
            buffer.push(c);
        }

        // Trailing
        if !buffer.is_empty() {
            if was_variable {
                tokens.push(TokenType::Property(TokenProperty::new(&buffer)));
                syntax.add(buffer.len(), SyntaxType::Property);
            } else {
                tokens.push(TokenType::Variable(TokenVariable::new(&buffer)));
                syntax.add(buffer.len(), SyntaxType::Variable);

            }
        }
        TokenCommand { tokens }
    }
}


impl Token for TokenCommand {
    fn get_value(&self, _input: Option<&Data>, info: &AudioFileInfo, config: &RenamerConfig) -> Option<Data> {
        let mut output = vec![];
        let mut data = None;
        for t in &self.tokens {
            // Constants
            if let TokenType::Constant(c) = t {
                // Leftover data
                if data.is_some() {
                    output.push(data);
                    data = None;
                }
                output.push(c.get_value(None, info, config));
                continue;
            }
            // Normal
            data = t.token().get_value(data.as_ref(), info, config);
            if data.is_none() {
                return None;
            }
        }
        // leftover
        if data.is_some() {
            output.push(data);
        }
        // Merge
        let o = output
            .iter()
            .filter_map(|d| d.as_ref().map(|d| d.to_string(&config.separator)))
            .collect::<String>();
        Some(Data::String(o))
    }
}


/// Constant string value
#[derive(Debug, Clone)]
struct TokenConstant {
    string: String
}

impl TokenConstant {
    /// Create new one
    pub fn new(value: &str) -> TokenConstant {
        TokenConstant { string: value.to_string() }
    }
}

impl Token for TokenConstant {
    fn get_value(&self, _input: Option<&Data>, _info: &AudioFileInfo, _config: &RenamerConfig) -> Option<Data> {
        Some(Data::String(self.string.to_string()))
    }
}

#[derive(Debug, Clone)]
struct TokenVariable {
    var: String
}

impl TokenVariable {
    pub fn new(name: &str) -> TokenVariable {
        TokenVariable { var: name.to_string() }
    }

    /// Get raw value
    pub fn get_raw_value(&self, info: &AudioFileInfo) -> Option<Data> {
        // Parse field name
        let lower = self.var.to_lowercase();
        let field = match &lower[..] {
            "title" => Some(Field::Title),
            "artist" | "artists" => Some(Field::Artist),
            "album" => Some(Field::Album),
            "albumartist" | "albumartists" => Some(Field::AlbumArtist),
            "key" => Some(Field::Key),
            "bpm" => Some(Field::BPM),
            "genre" => Some(Field::Genre),
            "style" => Some(Field::Style),
            "label" => Some(Field::Label),
            "isrc" => Some(Field::ISRC),
            "catalognumber" => Some(Field::CatalogNumber),
            "version" => Some(Field::Version),
            "track" | "tracknumber" => Some(Field::TrackNumber),
            "duration" => Some(Field::Duration),
            "remixer" => Some(Field::Remixer),
            "total" | "tracktotal" => Some(Field::TrackTotal),
            "disc" | "disk" | "discnumber" | "disknumber" => Some(Field::DiscNumber),
            _ => None
        };
        if let Some(field) = field {
            let tag = field.by_format(&info.format);
            if let Some(v) = info.tags.get(tag) {
                // Artist/Album artist override
                if self.var.to_lowercase() == "artists" || self.var.to_lowercase() == "albumartists" {
                    if v.is_empty() {
                        return None;
                    }
                    return Some(Data::String(v.first().unwrap().to_string()));
                }
                return Some(Data::Array(v.clone()));
            }
        }

        // Date
        if lower == "year" || lower == "month" || lower == "day" {
            let tag = Tag::load_file(&info.path, false).ok()?;
            if let Some(date) = tag.tag().get_date() {
                let val = match &lower[..] {
                    "year" => Some(date.year),
                    "month" => date.month.map(|m| m as i32),
                    "day" => date.day.map(|d| d as i32),
                    _ => None
                }?;
                return Some(Data::String(val.to_string()));
            }
        }

        // Try to get tag directly
        if let Some(v) = info.tags.get(&self.var) {
            return Some(Data::Array(v.clone()));
        }
        // Built-ins
        match &self.var.to_lowercase()[..] {
            "filename" => Some(Data::String(Path::new(&info.path).file_stem().unwrap().to_string_lossy().to_string())),
            "path" => Some(Data::String(info.path.to_string_lossy().to_string())),
            "abspath" => Some(Data::String(dunce::canonicalize(&info.path).ok()?.to_string_lossy().to_string())),
            _ => None
        }
    }
}

impl Token for TokenVariable {
    fn get_value(&self, _input: Option<&Data>, info: &AudioFileInfo, _config: &RenamerConfig) -> Option<Data> {
        Some(self.get_raw_value(info)?)
    }
}


#[derive(Debug, Clone)]
struct TokenProperty {
    property: String
}

impl TokenProperty {
    fn new(property: &str) -> TokenProperty {
        TokenProperty { property: property.to_string() }
    }
}

impl Token for TokenProperty {
    fn get_value(&self, input: Option<&Data>, _info: &AudioFileInfo, _config: &RenamerConfig) -> Option<Data> {
        match input? {
            Data::String(_) => None,
            Data::Array(a) => {
                // Int index
                if let Ok(index) = self.property.parse::<usize>() {
                    return a.get(index).map(|i: &String| Data::String(i.to_string()));
                }
                // Friendly helpers
                match &self.property[..] {
                    "first" => a.first().map(|i| Data::String(i.to_string())),
                    "last" => a.last().map(|i| Data::String(i.to_string())),
                    _ => None
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
enum FunctionParameter {
    Number(i32),
    String(String)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FunctionParseState {
    Name,
    String,
    Number,
    Params
}

#[derive(Debug, Clone)]
struct TokenFunction {
    name: String,
    params: Vec<FunctionParameter>,
}

impl TokenFunction {
    /// Parse the function from string
    pub fn parse(input: &str, syntax: &mut SyntaxBuilder) -> Option<TokenFunction> {
        let mut name = String::new();
        let mut param = String::new();
        let mut state = FunctionParseState::Name;
        let mut params = vec![];
        let mut escape = false;

        for c in input.chars() {
            // Parse name
            if state == FunctionParseState::Name {
                if c != '(' {
                    name.push(c);
                    continue;
                }
                state = FunctionParseState::Params;
                syntax.add(name.len(), SyntaxType::Function);
                syntax.add(1, SyntaxType::Operator);
                continue;
            }

            // Parse args
            match c {
                '"' => {
                    // Start string param
                    if state == FunctionParseState::Params {
                        state = FunctionParseState::String;
                    // End string
                    } else {
                        // \" escape
                        if escape {
                            escape = false;
                            param.push('"');
                            continue;
                        }
                        state = FunctionParseState::Params;
                        params.push(FunctionParameter::String(param.clone()));
                        syntax.add(1, SyntaxType::Operator);
                        // add escape char length coz it's double in shown, but only single in buffer
                        syntax.add(param.len() + param.chars().filter(|c| c == &'\\').count(), SyntaxType::String);
                        syntax.add(1, SyntaxType::Operator);
                        param.clear();
                    }
                },
                // End of number parameter
                ' ' | ',' | ')' if !escape => {
                    if state == FunctionParseState::Number {
                        match param.parse() {
                            Ok(n) => params.push(FunctionParameter::Number(n)),
                            Err(e) => {
                                error!("Failed parsing number function parameter ({param}): {e}");
                                return None;
                            },
                        }
                        syntax.add(param.len(), SyntaxType::Number);
                        state = FunctionParseState::Params;
                        param.clear();
                    }
                    syntax.add(1, SyntaxType::Operator);
                },
                '\\' => {
                    if escape {
                        param.push('\\');
                        escape = false;
                    } else {
                        escape = true;
                    }
                },
                // Number
                c if c.is_digit(10) => {
                    if state == FunctionParseState::Params {
                        state = FunctionParseState::Number;
                    }
                    param.push(c);
                }
                _ => {
                    match state {
                        FunctionParseState::String => param.push(c),
                        FunctionParseState::Number => {
                            error!("Failed parsing number param: invalid end sequence: {c}");
                            return None;
                        }
                        _ => {},
                    }
                    escape = false;
                }
            }
        };

        Some(TokenFunction { name, params })
    }

    /// Get int param or error
    fn param_int(&self, index: usize, required: bool) -> Option<i32> {
        match self.params.get(index) {
            Some(FunctionParameter::Number(i)) => Some(*i),
            Some(FunctionParameter::String(s)) => {
                error!("Invalid parameter String ({s}), int needed!");
                None
            }
            None => {
                if required {
                    error!("Function is missing int parameter in position: {index}");
                }
                None
            },
        }
    }

    /// Get string parameter
    fn param_str(&self, index: usize, required: bool) -> Option<&str> {
        match self.params.get(index) {
            Some(FunctionParameter::String(s)) => Some(s.as_str()),
            Some(FunctionParameter::Number(n)) => {
                error!("Invalid parameter String ({n}), str needed!");
                None
            }
            None => {
                if required {
                    error!("Function is missing str parameter in position: {index}");
                }
                None
            },
        }
    }
}

impl Token for TokenFunction {
    fn get_value(&self, input: Option<&Data>, _info: &AudioFileInfo, config: &RenamerConfig) -> Option<Data> {
        let data = input?;

        match &self.name.to_lowercase()[..] {
            // Lowercase
            "lower" | "lowercase" => {
                return Some(Data::String(data.to_string(&config.separator).to_lowercase()));
            },
            // Uppercase
            "upper" | "uppercase" => {
                return Some(Data::String(data.to_string(&config.separator).to_uppercase()));
            },
            // Substring or array range
            "slice" | "range" => {
                let start = self.param_int(0, true)?;
                let end = self.param_int(1, false).unwrap_or(0);

                match data {
                    // Substring
                    Data::String(s) => {
                        let i = s.chars().skip(start as usize);
                        let s = if end != 0 && end >= start {
                            i.take((end - start) as usize).collect::<String>()
                        } else {
                            i.collect::<String>()
                        };
                        Some(Data::String(s))
                    },
                    // Subarray
                    Data::Array(a) => {
                        if start as usize > a.len() || (end <= start && end > 0) {
                            None
                        } else if end == 0 || end as usize > a.len() {
                            Some(Data::Array(a[start as usize..].to_vec()))
                        } else {
                            Some(Data::Array(a[start as usize..end as usize].to_vec()))
                        }
                    },
                }
            },
            // Capitalize first letter
            "capitalize" => {
                let s = data.to_string(&config.separator);
                let mut c = s.chars();
                let o = match c.next() {
                    None => String::new(),
                    Some(i) => i.to_uppercase().collect::<String>() + c.as_str(),
                };
                Some(Data::String(o))
            },
            // Replace string with string
            "replace" => {
                let from = self.param_str(0, true)?;
                let to = self.param_str(1, true)?;
                let re = match Regex::new(from) {
                    Ok(re) => re,
                    Err(e) => {
                        error!("Invalid regex: {from}: {e}");
                        return None;
                    }
                };
                let text = data.to_string(&config.separator);
                let text = re.replace_all(&text, to);
                Some(Data::String(text.to_string()))
            },
            // Padding on beggining
            "pad" => {
                let character = self.param_str(0, true)?;
                let len = self.param_int(1, true)?;
                if len == 0 {
                    return Some(data.clone());
                }
                Some(Data::String(data.to_string(&config.separator).pad(len as usize, character.chars().next()?, Alignment::Right, false)))
            },
            // Sort array
            "sort" => {
                if let Data::Array(arr) = data {
                    let mut arr = arr.clone();
                    arr.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                    Some(Data::Array(arr))
                } else {
                    Some(data.clone())
                }
            },
            // Reverse array or string
            "reverse" => {
                match data {
                    Data::String(s) => Some(Data::String(s.chars().rev().collect::<String>())),
                    Data::Array(a) => Some(Data::Array(a.iter().rev().map(String::from).collect::<Vec<_>>())),
                }
            },
            // Join array
            "join" => {
                let c = self.param_str(0, true)?;
                match data {
                    Data::String(s) => Some(Data::String(s.to_string())),
                    Data::Array(a) => Some(Data::String(a.join(c)))
                }
            },
            // Path parent
            "parent" => {
                match data {
                    Data::String(s) => {
                        let s = s.replace("\\", "/");
                        let parts = s.split("/").collect::<Vec<_>>();
                        if parts.len() < 2 {
                            return None;
                        }
                        let count = parts.len() - 1;
                        Some(Data::String(parts.into_iter().take(count).collect::<Vec<_>>().join("/")))
                    },
                    Data::Array(a) => {
                        if a.len() < 2 {
                            return None;
                        }
                        Some(Data::Array(a.iter().take(a.len() - 1).map(String::from).collect()))
                    }
                }
            },
            // Path file/folder name
            "filename" => {
                match data {
                    Data::String(s) => {
                        let s = s.replace("\\", "/");
                        let parts = s.split("/").collect::<Vec<_>>();
                        Some(Data::String(parts.last()?.to_string()))
                    },
                    // Same as array.last()
                    Data::Array(a) => {
                        Some(Data::String(a.last()?.to_string()))
                    },
                }
            }
            f => {
                error!("Invalid function: {f}!");
                None
            }
        }
    }
}