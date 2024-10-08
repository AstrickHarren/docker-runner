use std::{borrow::Cow, fmt::Display};

use docker_derive::Instruction;
use itertools::Itertools;

pub struct From {
    image: String,
    tag: Option<String>,
}

impl From {
    pub fn image(image: impl ToString) -> Self {
        Self {
            image: image.to_string(),
            tag: Default::default(),
        }
    }

    pub fn with_tag(mut self, tag: impl ToString) -> Self {
        self.tag = Some(tag.to_string());
        self
    }
}

#[derive(Instruction)]
pub struct Copy {
    pub from: String,
    pub to: String,
}

impl Copy {
    pub fn new(from: impl ToString, to: impl ToString) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}

#[derive(Debug, Instruction)]
pub struct Volume {
    pub from: String,
    pub to: String,
}

impl Volume {
    pub fn new(from: impl ToString, to: impl ToString) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}

#[derive(Debug, Instruction)]
pub struct Run {
    pub cmd: String,
}

impl Run {
    pub fn new(cmd: impl ToString) -> Self {
        Self {
            cmd: cmd.to_string(),
        }
    }
}

pub struct EntryPoint {
    pub cmds: Vec<String>,
}

pub struct DockerFile {
    from: From,
    entry_point: Option<EntryPoint>,
    instrs: Vec<Box<dyn Instruction>>,
}

impl DockerFile {
    pub fn new(from: From) -> Self {
        Self {
            from,
            entry_point: Default::default(),
            instrs: Default::default(),
        }
    }

    pub fn entry_point(mut self, entry_point: impl IntoIterator<Item = impl ToString>) -> Self {
        let cmds = entry_point.into_iter().map(|x| x.to_string()).collect();
        self.entry_point = Some(EntryPoint { cmds });
        self
    }

    pub fn then(mut self, instr: impl Instruction + 'static) -> Self {
        self.instrs.push(Box::new(instr));
        self
    }
}

pub trait Instruction: Display {}
/******************* DISPLAYS *******************/
impl Display for From {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FROM {}", self.image)?;
        if let Some(x) = &self.tag {
            write!(f, "{}", x)?;
        }
        Ok(())
    }
}

impl Display for Copy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "COPY {} {}", self.from, self.to)
    }
}

impl Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VOLUME {} {}", self.from, self.to)
    }
}

impl Display for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RUN {}", self.cmd)
    }
}

impl Display for EntryPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cmds = self.cmds.iter().map(|x| format!("\"{}\"", x)).join(", ");
        write!(f, "ENTRYPOINT [{}]", cmds)
    }
}

impl Display for DockerFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.from)?;
        writeln!(f, "{}", self.instrs.iter().join("\n"))?;
        if let Some(p) = &self.entry_point {
            writeln!(f, "{}", p)?;
        }
        Ok(())
    }
}

impl std::convert::From<&DockerFile> for Cow<'_, str> {
    fn from(value: &DockerFile) -> Self {
        value.to_string().into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::instruction::Volume;

    #[test]
    fn test_docker_file_creation() {
        let df = DockerFile::new(From::image("alpine"))
            .then(Copy::new(".", "."))
            .then(Volume::new(".", "."))
            .then(Run::new("apt add curl"))
            .entry_point(["echo", "hello"]);

        let df_exp = r#"
FROM alpine
COPY . .
VOLUME . .
RUN apt add curl
ENTRYPOINT ["echo", "hello"]
            "#;

        assert_eq!(df.to_string().trim(), df_exp.trim());
    }
}
