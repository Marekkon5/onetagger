use onetagger_tagger::{AudioFileInfo, Field};
use pad::{PadStr, Alignment};


#[derive(Debug, Clone)]
pub struct TemplateParser {
    tokens: Vec<TokenType>,
    pub syntax: Vec<SyntaxData>,
}

impl TemplateParser {
    /// Apply template
    pub fn evaluate(&mut self, info: &AudioFileInfo) -> String {
        let mut output = String::new();
        for token in &self.tokens {
            if let Some(data) = token.token().get_value(None, info) {
                output.push_str(&data.to_string());
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

        for c in input.chars() {
            match c {
                '%' => {
                    // End of command
                    if command {
                        tokens.push(TokenType::Command(TokenCommand::parse(&buffer, &mut syntax)));
                        syntax.add(1, SyntaxType::Operator);
                        buffer.clear();
                        command = false;
                        continue;
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
                }
                _ => {}
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
    pub fn to_string(&self) -> String {
        match self {
            Data::String(s) => s.to_string(),
            Data::Array(a) => a.join(", "),
        }
    }
}

/// Every token type should implement this
trait Token {
    /// Evaluate the token
    fn get_value(&self, input: Option<&Data>, info: &AudioFileInfo) -> Option<Data>;
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
                '.' if !string => {
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
    fn get_value(&self, _input: Option<&Data>, info: &AudioFileInfo) -> Option<Data> {
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
                output.push(c.get_value(None, info));
                continue;
            }
            // Normal
            data = t.token().get_value(data.as_ref(), info);
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
            .filter_map(|d| d.as_ref().map(|d| d.to_string()))
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
    fn get_value(&self, _input: Option<&Data>, _info: &AudioFileInfo) -> Option<Data> {
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
}

impl Token for TokenVariable {
    fn get_value(&self, _input: Option<&Data>, info: &AudioFileInfo) -> Option<Data> {
        // Parse field name
        let field = match &self.var.to_lowercase()[..] {
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
            "tracknumber" => Some(Field::TrackNumber),
            "duration" => Some(Field::Duration),
            "remixer" => Some(Field::Remixer),
            _ => None
        };
        if let Some(field) = field {
            let tag = field.by_format(&info.format);
            if let Some(v) = info.tags.get(tag) {
                return Some(Data::Array(v.clone()));
            }
        }
        // Try to get tag directly
        if let Some(v) = info.tags.get(&self.var) {
            return Some(Data::Array(v.clone()));
        }
        // Built-ins
        match &self.var.to_lowercase()[..] {
            _ => None
        }

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
    fn get_value(&self, input: Option<&Data>, _info: &AudioFileInfo) -> Option<Data> {
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
                        syntax.add(param.len(), SyntaxType::String);
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
                '\\' => escape = true,
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
    fn get_value(&self, input: Option<&Data>, _info: &AudioFileInfo) -> Option<Data> {
        let data = input?;

        match &self.name.to_lowercase()[..] {
            // Lowercase
            "lower" | "lowercase" => {
                return Some(Data::String(data.to_string().to_lowercase()));
            },
            // Uppercase
            "upper" | "uppercase" => {
                return Some(Data::String(data.to_string().to_uppercase()));
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
                let s = data.to_string();
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
                Some(Data::String(data.to_string().replace(from, to)))
            },
            // Padding on beggining
            "pad" => {
                let character = self.param_str(0, true)?;
                let len = self.param_int(1, true)?;
                if len == 0 {
                    return Some(data.clone());
                }
                debug!("pad: {} {}", character, len);
                Some(Data::String(data.to_string().pad(len as usize, character.chars().next()?, Alignment::Right, false)))
            }
            f => {
                error!("Invalid function: {f}!");
                None
            }
        }
    }
}