use hot_reload::{plugins::DynPlugin, App};
use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

fn prompt(message: &str) -> io::Result<String> {
    let stdin = io::stdin();
    let stderr = io::stderr();

    write!(stderr.lock(), "{} ", message)?;

    let mut line = String::new();
    stdin.read_line(&mut line)?;

    line.remove(line.len() - 1); // remove the newline
    Ok(line)
}

#[cfg(target_os = "windows")]
const LOADABLE_LIB_EXTENSION: &[u8] = b"dll";

#[cfg(any(target_os = "linux", target_os = "openbsd"))]
const LOADABLE_LIB_EXTENSION: &[u8] = b"so";

fn main() -> anyhow::Result<()> {
    let plugin_path = {
        let answer = prompt("What plugin should be loaded?")?;
        PathBuf::from(answer)
    };

    let file_extension = plugin_path.extension().map(|ext| ext.as_encoded_bytes());
    eprintln!("{:?}", file_extension.map(std::str::from_utf8));
    if file_extension == Some(LOADABLE_LIB_EXTENSION) {
        load_plugin(plugin_path.as_path())
    } else {
        eprintln!(
            "Please supply a path to a .so on linux/bsd or a .dll on windows, not {}.",
            plugin_path.display()
        );
        std::process::exit(1)
    }
}

fn load_plugin(plugin_path: &Path) -> anyhow::Result<()> {
    assert!(plugin_path.is_file());

    let mut app = App::default();
    let dyn_plugin = DynPlugin::from_library_path(plugin_path);

    app.register_number(1)
        .add_plugin(dyn_plugin)
        .register_number(100);

    eprintln!("{:?}", app);
    Ok(())
}
