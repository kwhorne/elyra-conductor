//! Where the webview may read and write on disk.
//!
//! Every filesystem command takes a path the frontend chose, and the frontend
//! renders content we do not control — runbooks that travel with cloned
//! repositories, GitHub release notes, agent output. DOMPurify and the CSP are
//! what keep that content from running script; this module is the layer beneath
//! them, so that if they ever fail, script in the webview still cannot read
//! `~/.ssh` or append to `~/.zshrc`. It is deliberately coarse: everything under
//! the home directory, on mounted volumes and in the temp dir is allowed, minus a
//! short list of places that hold credentials or run code at login.
//!
//! Paths are resolved before they are judged — `..` lexically, symlinks through
//! the deepest existing ancestor — so `~/proj/link/id_rsa` with `link -> ~/.ssh`
//! is treated as what it really is.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Access {
    Read,
    Write,
}

/// Places under `$HOME` that hold credentials. Never read, never written.
const SECRET_DIRS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".azure",
    ".kube",
    ".docker",
    ".tauri",
    ".config/gh",
    ".config/gcloud",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".gem/credentials",
    ".password-store",
    ".local/share/keyrings",
    "Library/Keychains",
    "Library/Cookies",
    "Library/Application Support/com.apple.TCC",
];

/// Places under `$HOME` that run code at login or on the next shell or `git`
/// invocation. The editor may show them; nothing here may write them.
const LOGIN_DIRS: &[&str] = &[
    ".zshrc",
    ".zshenv",
    ".zprofile",
    ".zlogin",
    ".zlogout",
    ".bashrc",
    ".bash_profile",
    ".bash_login",
    ".profile",
    ".hushlogin",
    ".gitconfig",
    ".config/git",
    ".config/fish",
    ".config/zsh",
    ".oh-my-zsh",
    "Library/LaunchAgents",
    "Library/LaunchDaemons",
    ".local/bin",
    "bin",
];

pub struct Policy {
    pub home: PathBuf,
    pub roots: Vec<PathBuf>,
}

impl Policy {
    pub fn from_env() -> Result<Policy, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|h| h.is_absolute())
            .ok_or_else(|| "HOME is not set".to_string())?;
        let home = home.canonicalize().unwrap_or(home);
        let tmp = std::env::temp_dir();
        let roots = vec![
            home.clone(),
            PathBuf::from("/Volumes"),
            tmp.canonicalize().unwrap_or(tmp),
            PathBuf::from("/tmp"),
            PathBuf::from("/private/tmp"),
        ];
        Ok(Policy { home, roots })
    }
}

/// Check `raw` against the policy derived from the environment. Returns the
/// resolved real path on success; callers keep using `raw` for the operation
/// itself so listings and messages show the path the user typed.
pub fn check(raw: &str, access: Access) -> Result<PathBuf, String> {
    check_with(raw, access, &Policy::from_env()?)
}

pub fn check_with(raw: &str, access: Access, policy: &Policy) -> Result<PathBuf, String> {
    let real = resolve(raw)?;
    if !policy.roots.iter().any(|r| real.starts_with(r)) {
        return Err(format!(
            "{raw}: outside your home folder, mounted volumes and the temp dir — Conductor doesn't touch anything else"
        ));
    }
    if let Ok(rel) = real.strip_prefix(&policy.home) {
        let mut lists: Vec<&[&str]> = vec![SECRET_DIRS];
        if access == Access::Write {
            lists.push(LOGIN_DIRS);
        }
        // `Path::starts_with` compares whole components, so `.ssh` does not
        // match `.sshfoo` and `.config/gh` does not match `.config/ghost`.
        if let Some(d) = lists.iter().flat_map(|l| l.iter()).find(|d| rel.starts_with(d)) {
            let verb = if access == Access::Write { "writes to" } else { "reads" };
            return Err(format!("{raw}: Conductor never {verb} ~/{d}"));
        }
    }
    Ok(real)
}

/// Absolute, `.`/`..`-free, symlinks resolved. Non-existent trailing segments
/// are kept so a file about to be created is judged by where it will land.
fn resolve(raw: &str) -> Result<PathBuf, String> {
    real_path(&lexical(raw)?, 0)
}

fn lexical(raw: &str) -> Result<PathBuf, String> {
    let p = Path::new(raw);
    if !p.is_absolute() {
        return Err(format!("{raw}: not an absolute path"));
    }
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::RootDir => out.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err(format!("{raw}: escapes the filesystem root"));
                }
            }
            Component::Normal(s) => out.push(s),
            Component::Prefix(_) => return Err(format!("{raw}: unsupported path prefix")),
        }
    }
    Ok(out)
}

fn real_path(p: &Path, depth: u8) -> Result<PathBuf, String> {
    if depth > 16 {
        return Err(format!("{}: too many levels of symbolic links", p.display()));
    }
    let mut cur = p.to_path_buf();
    let mut tail: Vec<OsString> = Vec::new();
    loop {
        match std::fs::symlink_metadata(&cur) {
            Ok(md) if md.file_type().is_symlink() => {
                // Resolve by hand so a *dangling* link is still judged by its
                // target — `canonicalize` would refuse it, and refusing is not
                // the same as denying: the write would then create the target.
                let target = std::fs::read_link(&cur).map_err(|e| format!("{}: {e}", cur.display()))?;
                let target = if target.is_absolute() {
                    target
                } else {
                    cur.parent().unwrap_or(Path::new("/")).join(target)
                };
                let mut rebuilt = lexical(&target.to_string_lossy())?;
                for t in tail.iter().rev() {
                    rebuilt.push(t);
                }
                return real_path(&rebuilt, depth + 1);
            }
            Ok(_) => {
                let mut real = cur.canonicalize().map_err(|e| format!("{}: {e}", cur.display()))?;
                for t in tail.iter().rev() {
                    real.push(t);
                }
                return Ok(real);
            }
            Err(_) => match (cur.file_name(), cur.parent()) {
                (Some(name), Some(parent)) => {
                    tail.push(name.to_os_string());
                    cur = parent.to_path_buf();
                }
                _ => return Ok(p.to_path_buf()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("conductor-path-policy-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p.canonicalize().unwrap()
    }

    fn policy(home: &Path, volume: &Path) -> Policy {
        Policy { home: home.to_path_buf(), roots: vec![home.to_path_buf(), volume.to_path_buf()] }
    }

    fn s(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn relative_paths_are_rejected() {
        let home = scratch("rel");
        let pol = policy(&home, &home);
        assert!(check_with("Code/app/.env", Access::Read, &pol).is_err());
    }

    #[test]
    fn anything_outside_the_roots_is_rejected() {
        let home = scratch("outside");
        let pol = policy(&home, &home);
        let err = check_with("/etc/passwd", Access::Read, &pol).unwrap_err();
        assert!(err.contains("outside"), "{err}");
    }

    #[test]
    fn dot_dot_cannot_climb_out_of_home() {
        let home = scratch("dotdot");
        let pol = policy(&home, &home);
        let sneaky = format!("{}/Code/../../../../../../etc/passwd", s(&home));
        assert!(check_with(&sneaky, Access::Read, &pol).is_err());
        assert!(check_with("/../etc/passwd", Access::Read, &pol).is_err());
    }

    #[test]
    fn project_files_are_readable_and_writable_even_before_they_exist() {
        let home = scratch("project");
        let pol = policy(&home, &home);
        let env = format!("{}/Code/app/.env", s(&home));
        assert!(check_with(&env, Access::Write, &pol).is_ok());
        assert!(check_with(&env, Access::Read, &pol).is_ok());
        let note = format!("{}/Code/app/.conductor/notes/deploy.md", s(&home));
        assert!(check_with(&note, Access::Write, &pol).is_ok());
    }

    #[test]
    fn credential_dirs_are_never_read_and_prefixes_do_not_bleed() {
        let home = scratch("secrets");
        let pol = policy(&home, &home);
        let key = format!("{}/.ssh/id_ed25519", s(&home));
        let err = check_with(&key, Access::Read, &pol).unwrap_err();
        assert!(err.contains("never reads ~/.ssh"), "{err}");
        assert!(check_with(&format!("{}/.aws/credentials", s(&home)), Access::Read, &pol).is_err());
        assert!(check_with(&format!("{}/.tauri/x.key", s(&home)), Access::Read, &pol).is_err());
        assert!(check_with(&format!("{}/.config/gh/hosts.yml", s(&home)), Access::Read, &pol).is_err());
        // Same leading characters, different directory: allowed.
        assert!(check_with(&format!("{}/.sshfoo/notes.txt", s(&home)), Access::Read, &pol).is_ok());
        assert!(check_with(&format!("{}/.config/ghost/x", s(&home)), Access::Read, &pol).is_ok());
    }

    #[test]
    fn login_files_are_readable_but_not_writable() {
        let home = scratch("login");
        let pol = policy(&home, &home);
        for f in [".zshrc", ".gitconfig", "Library/LaunchAgents/com.evil.plist", ".local/bin/git", "bin/git"] {
            let p = format!("{}/{f}", s(&home));
            assert!(check_with(&p, Access::Read, &pol).is_ok(), "{f} should be readable");
            let err = check_with(&p, Access::Write, &pol).unwrap_err();
            assert!(err.contains("never writes to"), "{f}: {err}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_into_a_credential_dir_is_judged_by_its_target() {
        use std::os::unix::fs::symlink;
        let home = scratch("symlink");
        let pol = policy(&home, &home);
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::create_dir_all(home.join("Code/proj")).unwrap();
        symlink(home.join(".ssh"), home.join("Code/proj/keys")).unwrap();
        let via_link = format!("{}/Code/proj/keys/id_ed25519", s(&home));
        let err = check_with(&via_link, Access::Read, &pol).unwrap_err();
        assert!(err.contains("~/.ssh"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_to_a_login_file_cannot_be_written_through() {
        use std::os::unix::fs::symlink;
        let home = scratch("dangling");
        let pol = policy(&home, &home);
        std::fs::create_dir_all(home.join("Code/proj")).unwrap();
        // Target does not exist yet — writing through the link would create it.
        symlink(home.join(".zshrc"), home.join("Code/proj/rc")).unwrap();
        let via_link = format!("{}/Code/proj/rc", s(&home));
        let err = check_with(&via_link, Access::Write, &pol).unwrap_err();
        assert!(err.contains("~/.zshrc"), "{err}");
    }

    #[test]
    fn mounted_volumes_are_allowed_for_exports() {
        let home = scratch("home-for-volumes");
        let vol = scratch("volume");
        let pol = policy(&home, &vol);
        let export = format!("{}/Exports/orders.xlsx", s(&vol));
        assert!(check_with(&export, Access::Write, &pol).is_ok());
    }
}
