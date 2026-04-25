fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "scamp");
        res.set("FileDescription", "A small animated cat that lives in your terminal");
        if let Err(e) = res.compile() {
            eprintln!("warning: failed to embed Windows icon resource: {}", e);
        }
    }
}
