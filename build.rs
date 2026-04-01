#[cfg(windows)]
fn main() -> std::io::Result<()> {
    let mut res = winres::WindowsResource::new();
    res.set_icon("src/assets/icon.ico");
    res.compile()?;
    
    Ok(())
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    Ok(())
}