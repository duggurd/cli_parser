use std::{
    borrow::{Borrow, BorrowMut},
    collections::{HashMap, HashSet},
    env::{args, Args},
    fmt::Debug,
    iter::Peekable,
    ops::Deref,
};

/// Represents a main command.
/// Currently does not support subcommands.
///
/// Uses builder pattern for construction
///
/// ## Example
/// ```
/// let cmd = Command::new("help")
///     .positional()
///     .flag(Flag::new("--always"));
/// ```
#[derive(Debug, Clone)]
pub struct Command {
    // id of command, ie. identifier or name
    pub id: String,
    // If command has a positioanl value
    positional: bool,
    // Actual positional value after parsing
    pub positional_val: Option<String>,
    // Does command take any flags?
    flags: HashMap<String, Flag>,
    // actually parsed flags
    pub parsed_flags: HashMap<String, Flag>,
}

impl Command {
    /// Crate a new command instance with id of the command
    ///
    /// ## Example
    /// ```
    /// Command::new("version");
    /// ````
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            positional: false,
            positional_val: None,
            flags: HashMap::new(),
            parsed_flags: HashMap::new(),
        }
    }

    /// Does command have a positional value associated?
    pub fn positional(mut self) -> Self {
        self.positional = true;
        self
    }

    /// Does the command have any flags associated?
    /// See [Flag]
    pub fn flag(mut self, flag: Flag) -> Self {
        self.flags.insert(flag.id.clone(), flag);
        self
    }

    // /// Add a parsed flag to `parsed_flags``
    // fn parsed_flag(&mut self, flag: Flag) {
    //     self.parsed_flags.insert(flag.id.clone(), flag);
    // }
}

/// Represents a CLI Flag
///
/// Uses builder pattern to create
///
/// ## Example
/// ```
/// let flag = Flag::new("--ip").positional();
/// // Results in same Flag
/// let flag = Flag::new("ip").positional();
/// ````
#[derive(Debug, Clone)]
pub struct Flag {
    /// Id / name of flag
    pub id: String,
    // If flag has an associated positional value or not
    positional: bool,
    // Actual parsed positional value
    pub positional_val: Option<String>,
    required: bool,
}

impl Flag {
    /// Createa a new `Flag` builder
    pub fn new(id: &str) -> Self {
        let new_id = if !id.starts_with("--") {
            format!("--{}", id)
        } else {
            id.to_string()
        };

        Self {
            id: new_id.into(),
            positional: false,
            positional_val: None,
            required: false,
        }
    }

    /// If flag has associated positional value
    pub fn positional(mut self) -> Self {
        self.positional = true;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Represents an error that occured during parsing of the Cli input, a [Command] or a [Flag].
#[derive(Debug)]
pub enum ParseError {
    None,
    MissingPositional,
    NoCommands,
    InvalidCommand(String),
    InvalidFlag(String),
    ExpectedCommand,
    ExpectedPositional,
    ExpectedFlag,
    RequiredPositional,
    MissingRequiredFlag(String),
}

/// Parses the CLI inputs based on provided `Commands`
///
/// Exposes a builder interface with `::new();`
///
/// ## Usage
/// ```
/// let app = CliParser::new()
///     .command(Command::new("help")
///     .command(Command::new("version")
///     .parse()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct CliParser<It>
where
    It: Iterator<Item = String>,
    It::Item: Debug,
{
    // Provided commands used for parsing
    pub(crate) commands: HashMap<String, Command>,
    // Input program arguments to parse into final [Command] struct
    args: Peekable<It>,
    // Global flags
    pub(crate) global_flags: HashMap<String, Flag>,
    // actually parsed flags
    pub parsed_flags: HashMap<String, Flag>,
}

impl<It> CliParser<It>
where
    It: Iterator<Item = String>,
    It::Item: Debug,
{
    /// Create a new [CliParser] builder
    ///
    /// ## Example
    /// ```
    /// let app = CliParser::new()
    ///     .command(Command::new("help")
    ///     .command(Command::new("version")
    ///     .parse()
    ///     .unwrap();
    /// ```
    pub fn new() -> CliParser<Peekable<Args>> {
        let mut args = args().peekable();
        let _ = args.next().unwrap();

        CliParser::from_args(args)

        // Self {
        //     commands: HashMap::new(),
        //     args: args,
        //     flags: HashMap::new(),
        //     parsed_flags: HashSet::new(),
        // }
    }

    pub fn from_args(it: It) -> Self
    where
        It: Iterator<Item = String>,
        It::Item: Debug,
    {
        Self {
            commands: HashMap::new(),
            args: it.peekable(),
            global_flags: HashMap::new(),
            parsed_flags: HashMap::new(),
        }
    }

    /// Add a [Command] to be parsed
    pub fn command(mut self, command: Command) -> Self {
        self.commands.insert(command.id.clone(), command);
        self
    }

    /// Add a global [Flag] to be parsed
    pub fn global_flag(mut self, flag: Flag) -> Self {
        self.global_flags.insert(flag.id.clone(), flag);
        self
    }

    /// Parse the provided program args into the constructed Command tree
    ///
    /// ## Errors
    /// If any parsing fails return a [ParseError] error
    pub fn parse(&mut self) -> Result<Command, ParseError> {
        self.parse_next(&mut None)
    }

    fn parse_next(&mut self, command: &mut Option<Command>) -> Result<Command, ParseError> {
        self.parse_flags(command)?;
        // Validate so far
        if let Some(command) = command {
            // Validate required flags
            for (id, flag) in command.flags.iter() {
                if flag.required {
                    if !command.parsed_flags.contains_key(id) {
                        Err(ParseError::MissingRequiredFlag(id.into()))?;
                    }
                }
            }
        }
        if self.args.peek().is_some() {
            self.parse_next_cmd(command)
        } else {
            Ok(command.to_owned().unwrap())
        }
    }

    fn parse_flags(&mut self, command: &mut Option<Command>) -> Result<(), ParseError> {
        while self.args.peek().is_some_and(|arg| arg.starts_with("-")) {
            self.parse_next_flag(command)?;
        }
        Ok(())
    }

    fn parse_next_flag(&mut self, command: &mut Option<Command>) -> Result<(), ParseError> {
        let flag_str = match self.args.next() {
            Some(flag) => flag,
            None => Err(ParseError::ExpectedFlag)?,
        };

        // Global flags take precedence over local, should maybe be other way around?
        if self.global_flags.contains_key(&flag_str) {
            let glob_flag = (*self.global_flags.get(&flag_str).unwrap()).clone();
            let parsed_flag = self.parse_flag(&flag_str, &glob_flag)?;
            self.parsed_flags.insert(flag_str.into(), parsed_flag);
        } else if command
            .as_ref()
            .is_some_and(|c| c.flags.contains_key(&flag_str))
        {
            let local_flag = (*command.as_ref().unwrap().flags.get(&flag_str).unwrap()).clone();
            let parsed_flag = self.parse_flag(&flag_str, &local_flag)?;

            command
                .as_mut()
                .unwrap()
                .parsed_flags
                .insert(flag_str.into(), parsed_flag);
        } else {
            Err(ParseError::InvalidFlag(flag_str))?;
        }

        Ok(())
    }

    /// Parse a flag based on a flag_id and a flag_recipe
    /// Parses positional values
    fn parse_flag(&mut self, flag_str: &str, flag_recipe: &Flag) -> Result<Flag, ParseError> {
        let mut parsed_flag = Flag::new(flag_str);
        if flag_recipe.positional {
            parsed_flag.positional_val = match self.args.next() {
                Some(v) => Some(v),
                None => Err(ParseError::MissingPositional)?,
            };
        }
        Ok(parsed_flag)
    }

    /// Recursively parse a command based on constructed cli recipe
    fn parse_next_cmd(&mut self, command: &mut Option<Command>) -> Result<Command, ParseError> {
        let cmd_str: String = match self.args.next() {
            Some(cmd_str) => cmd_str,
            None => Err(ParseError::ExpectedCommand)?,
        };

        // TODO: Prune branches, branches on cmd_str and cmd_recipe.id
        let mut cmd_recipe = match command {
            Some(recipe) => recipe.to_owned(),

            None => match self.commands.get(&cmd_str) {
                Some(cmd) => (*cmd).clone(),
                None => Err(ParseError::InvalidCommand(cmd_str))?,
            },
        };

        if cmd_recipe.positional {
            match self.args.next() {
                Some(pos) => cmd_recipe.positional_val = Some(pos),
                None => Err(ParseError::ExpectedPositional)?,
            }
        }

        self.parse_next(&mut Some(cmd_recipe))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_from_str() {
        let args = "help".split(" ").map(|s| s.to_string());

        let parse_res = CliParser::from_args(args)
            .command(Command::new("help"))
            .command(Command::new("version"))
            .parse()
            .unwrap();
        println!("{:?}", parse_res);
    }

    #[test]
    fn test_multi_flags() {
        let args = "command --flag1 --flag2 test"
            .split(" ")
            .map(|s| s.to_string());

        let cmd = CliParser::from_args(args)
            .command(
                Command::new("command")
                    .flag(Flag::new("--flag1"))
                    .flag(Flag::new("--flag2").positional()),
            )
            .command(Command::new("version"))
            .parse()
            .unwrap();

        assert_eq!("command", cmd.id);
        assert!(cmd.parsed_flags.get("--flag1").is_some());
        assert!(cmd
            .parsed_flags
            .get("--flag2")
            .is_some_and(|f| f.positional_val == Some("test".into())));

        // println!("{:?}", cmd);
    }
    #[test]
    fn test_positional_command_and_flags() {
        let args = "with_positional something --test something --test2 somethingelse"
            .split(" ")
            .map(|s| s.to_string());

        let _parse_res = CliParser::from_args(args)
            .command(
                Command::new("with_positional")
                    .positional()
                    .flag(Flag::new("--test").positional())
                    .flag(Flag::new("--test2").positional()),
            )
            .command(Command::new("version"))
            .parse()
            .unwrap();

        println!("{:?}", _parse_res)
    }

    #[test]
    fn test_positional_command() {
        let args = "with_positional test".split(" ").map(|s| s.to_string());

        let _parse_res = CliParser::from_args(args)
            .command(Command::new("with_positional").positional())
            .command(Command::new("version"))
            .parse()
            .unwrap();
    }
    #[test]
    fn test_required_flag() {
        let args = "help --test banaa".split(" ").map(|s| s.to_string());

        let parse_res = CliParser::from_args(args)
            .command(
                Command::new("help")
                    // .positional()
                    .flag(Flag::new("test").positional().required()),
            )
            .command(Command::new("version"))
            .parse()
            .unwrap();
    }

    #[test]
    fn test_retrieval() {
        let args = "help --test banana".split(" ").map(|s| s.to_string());

        let parse_res = CliParser::from_args(args)
            .command(Command::new("help").flag(Flag::new("test").positional().required()))
            .command(Command::new("version"))
            .parse()
            .unwrap();

        assert_eq!(parse_res.id.as_str(), "help");

        assert_eq!(parse_res.positional_val, None);
        assert!(parse_res.parsed_flags.get("--test").is_some());

        assert!(parse_res
            .parsed_flags
            .get("--test")
            .is_some_and(|f| f.positional_val.as_ref().is_some_and(|v| v == "banana")));
        // println!("{:?}", parse_res);
    }

    #[test]
    fn test_glob_and_local_flags() {
        let args = "command --glob1 --local1 --glob2"
            .split(" ")
            .map(|s| s.to_string());

        let parse_res = CliParser::from_args(args)
            .command(Command::new("command").flag(Flag::new("--local1").positional().required()))
            .global_flag(Flag::new("--glob1"))
            // .global_flag(Flag::new("--glob2"))
            .parse()
            .unwrap();

        assert!(parse_res.parsed_flags.contains_key("--glob1"));
        assert!(parse_res.parsed_flags.contains_key("--glob2"));
        assert!(parse_res.parsed_flags.contains_key("--local1"));
    }

    #[test]
    #[ignore]
    /// THis does not work as intended atm
    fn test_parse_from_env() {
        let args = args().collect::<Vec<_>>();

        let parse_res = CliParser::<Args>::new()
            .command(
                Command::new("help")
                    .positional()
                    .flag(Flag::new("test").positional()),
            )
            .command(Command::new("version"))
            .parse();

        // println!("{:?}", parse_res);
    }
}
