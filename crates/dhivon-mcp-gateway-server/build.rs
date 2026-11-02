use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(std::io::Error::other("UI assets must not contain symlinks"));
        }
        if kind.is_dir() {
            collect(root, &entry.path(), files)?;
        } else if kind.is_file() {
            let path = entry.path();
            if !path.starts_with(root) {
                return Err(std::io::Error::other("Invalid asset path"));
            }
            files.push(path);
        }
    }
    Ok(())
}
fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-env-changed=DHIVON_MCP_GATEWAY_WEB_ASSETS");
    let out = PathBuf::from(
        env::var_os("OUT_DIR")
            .ok_or_else(|| std::io::Error::other("Missing Cargo output directory"))?,
    );
    let Some(root) = env::var_os("DHIVON_MCP_GATEWAY_WEB_ASSETS") else {
        fs::write(
            out.join("web_assets.rs"),
            "pub const INDEX: Option<&str> = None; pub const ASSETS: &[(&str,&str,&[u8])] = &[];",
        )?;
        return Ok(());
    };
    let root = PathBuf::from(root).canonicalize()?;
    let index = root.join("index.html");
    if !index.is_file() {
        return Err(std::io::Error::other(
            "Build React assets before compiling the website service",
        ));
    }
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files)?;
    files.sort();
    if files.len() > 512 {
        return Err(std::io::Error::other("Too many UI assets"));
    }
    let mut source = format!(
        "pub const INDEX: Option<&str> = Some(include_str!({:?}));\npub const ASSETS: &[(&str,&str,&[u8])] = &[\n",
        index
    );
    let mut size = 0u64;
    for path in files {
        size = size
            .checked_add(fs::metadata(&path)?.len())
            .ok_or_else(|| std::io::Error::other("Invalid UI asset size"))?;
        if size > 40 * 1024 * 1024 {
            return Err(std::io::Error::other("UI assets exceed 40 MiB"));
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(std::io::Error::other)?
            .to_str()
            .ok_or_else(|| std::io::Error::other("Non-UTF8 asset path"))?
            .replace('\\', "/");
        if relative == "index.html" {
            continue;
        }
        let mime = match path.extension().and_then(|s| s.to_str()) {
            Some("js") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("woff2") => "font/woff2",
            Some("woff") => "font/woff",
            Some("ttf") => "font/ttf",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("ico") => "image/x-icon",
            _ => return Err(std::io::Error::other("Unsupported UI asset type")),
        };
        source.push_str(&format!(
            "({:?},{:?},include_bytes!({:?})),\n",
            relative, mime, path
        ));
    }
    source.push_str("];\n");
    fs::write(out.join("web_assets.rs"), source)
}
