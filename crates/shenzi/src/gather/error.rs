use std::fmt;

#[derive(Debug)]
pub struct MultipleGatherErrors {
    pub errors: Vec<anyhow::Error>,
}

impl std::error::Error for MultipleGatherErrors {}

impl fmt::Display for MultipleGatherErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Multiple errors occurred while finding shared libraries for your application:"
        )?;
        for err in self.errors.iter() {
            writeln!(
                f,
                "---------------------------------------------------------------------------"
            )?;
            writeln!(f, "{}", err)?;
            writeln!(
                f,
                "---------------------------------------------------------------------------\n\n"
            )?;
        }
        Ok(())
    }
}
