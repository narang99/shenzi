use anyhow::Result;

pub fn run() -> Result<()> {
    // initiates a shenzi workspace
    crate::workspace::init_workspace()
}