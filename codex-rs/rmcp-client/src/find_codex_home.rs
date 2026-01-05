use dirs::home_dir;
use std::path::PathBuf;

/// This was copied from codex-core but codex-core depends on this crate.
/// TODO: move this to a shared crate lower in the dependency tree.
///
///
/// Returns the path to the Codex configuration directory.
///
/// In this fork, the directory can be specified by `CODEXX_HOME` (preferred)
/// or `CODEX_HOME`. If neither is set, defaults to `~/.codexx`.
///
/// - If `CODEXX_HOME` or `CODEX_HOME` is set, the value will be canonicalized and this
///   function will Err if the path does not exist.
/// - If neither `CODEXX_HOME` nor `CODEX_HOME` is set, this function does not verify that the
///   directory exists.
pub(crate) fn find_codex_home() -> std::io::Result<PathBuf> {
    // Prefer `CODEXX_HOME` to keep this fork isolated from upstream Codex when
    // both are installed on the same machine.
    if let Ok(val) = std::env::var("CODEXX_HOME")
        && !val.is_empty()
    {
        return PathBuf::from(val).canonicalize();
    }

    // Honor the `CODEX_HOME` environment variable when it is set to allow users
    // (and tests) to override the default location.
    if let Ok(val) = std::env::var("CODEX_HOME")
        && !val.is_empty()
    {
        return PathBuf::from(val).canonicalize();
    }

    let mut p = home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;
    p.push(".codexx");
    Ok(p)
}
