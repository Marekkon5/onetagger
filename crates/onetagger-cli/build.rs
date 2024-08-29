fn main() {
    // Set Windows icon
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("..\\..\\assets\\icon.ico");
        res.compile().unwrap();
    }
}