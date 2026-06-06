#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingCommand<'a> {
    pub(crate) command: &'a str,
    pub(crate) arguments: Vec<&'a str>,
}

impl<'a> BindingCommand<'a> {
    #[must_use]
    pub const fn new(command: &'a str) -> Self {
        Self {
            command,
            arguments: Vec::new(),
        }
    }

    #[must_use]
    pub fn arg(mut self, argument: &'a str) -> Self {
        self.arguments.push(argument);
        self
    }

    #[must_use]
    pub fn confirm_before(prompt: &'a str, run_command: &'a str) -> Self {
        Self::new("confirm-before")
            .arg("-p")
            .arg(prompt)
            .arg(run_command)
    }

    #[must_use]
    pub fn if_shell_format(format: &'a str, then_command: &'a str) -> Self {
        Self::new("if-shell")
            .arg("-F")
            .arg(format)
            .arg(then_command)
    }
}
