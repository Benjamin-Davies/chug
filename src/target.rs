use std::{fmt, process::Command};

#[derive(Debug)]
pub struct Target {
    arch: String,
    os: String,
}

impl Target {
    pub fn current() -> anyhow::Result<Self> {
        let arch = command_output("uname", &["-m"])?;
        let mut os = command_output("uname", &["-s"])?.to_lowercase();

        if os == "darwin" {
            let version = command_output("sw_vers", &["--productVersion"])?;
            let major_version = version.split('.').next().unwrap();
            match major_version {
                "13" => os = "ventura".to_owned(),
                "14" => os = "sonoma".to_owned(),
                "15" => os = "sequoia".to_owned(),
                "16" => os = "cheer".to_owned(),
                _ => anyhow::bail!("Unsupported macOS version: {version}"),
            }
        }

        Ok(Target { arch, os })
    }

    pub fn current_str() -> anyhow::Result<&'static str> {
        let target = cache!(String).get_or_init(|| {
            let target = Target::current()?;
            Ok(target.to_string())
        })?;
        Ok(target)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}_{}", self.arch, self.os)
    }
}

fn command_output(command: &str, args: &[&str]) -> anyhow::Result<String> {
    let bytes = Command::new(command).args(args).output()?.stdout;
    let mut output = String::from_utf8(bytes)?;

    while output.chars().last().unwrap().is_whitespace() {
        output.pop();
    }

    Ok(output)
}
