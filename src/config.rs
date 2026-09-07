use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value, value};

pub fn hook_command(executable: &Path) -> Result<String, String> {
    let s = executable.to_str().ok_or("executable path must be UTF-8")?;
    if !executable.is_absolute() || s.contains(['\0', '\n', '\r']) {
        return Err("executable path must be absolute and contain no line breaks".into());
    }
    Ok(format!(
        "\"{}\" report",
        s.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

pub fn edit_config(input: &str, command: &str, install: bool) -> Result<String, String> {
    if command.is_empty() {
        return Err("command is empty".into());
    }
    let mut doc = input
        .parse::<DocumentMut>()
        .map_err(|e| format!("invalid TOML: {e}"))?;
    if doc.get("hooks").is_none() {
        if !install {
            return Ok(input.into());
        }
        doc["hooks"] = Item::Table(Table::new());
    }
    let hooks = doc["hooks"]
        .as_table_like_mut()
        .ok_or("hooks must be a table")?;
    let Some(item) = hooks.get_mut("session_start") else {
        if !install {
            return Ok(input.into());
        }
        hooks.insert("session_start", value(command));
        return Ok(doc.to_string());
    };
    let remove_key = match item
        .as_value_mut()
        .ok_or("session_start must be a string or array")?
    {
        Value::String(s) => {
            if s.value() == command {
                if install {
                    return Ok(input.into());
                }
                true
            } else {
                if !install {
                    return Ok(input.into());
                }
                let decor = s.decor().clone();
                let mut old = Value::String(s.clone());
                *old.decor_mut() = Default::default();
                let mut array = toml_edit::Array::new();
                array.push_formatted(old);
                array.push(command);
                *array.decor_mut() = decor;
                *item = value(array);
                false
            }
        }
        Value::Array(array) => {
            if array.iter().any(|v| !v.is_str()) {
                return Err("session_start array must contain only strings".into());
            }
            let present = array.iter().any(|v| v.as_str() == Some(command));
            if install {
                if present {
                    return Ok(input.into());
                }
                array.push(command);
            } else {
                if !present {
                    return Ok(input.into());
                }
                array.retain(|v| v.as_str() != Some(command));
            }
            array.is_empty()
        }
        _ => return Err("session_start must be a string or array".into()),
    };
    if remove_key {
        hooks.remove("session_start");
    }
    Ok(doc.to_string())
}

fn snapshot(path: &Path) -> Result<Option<(fs::Metadata, Vec<u8>)>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() && meta.nlink() == 1 => {
            let data = fs::read(path).map_err(|e| e.to_string())?;
            Ok(Some((meta, data)))
        }
        Ok(_) => Err("config must be a regular non-symlink file with one hard link".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn unchanged(path: &Path, initial: &Option<(fs::Metadata, Vec<u8>)>) -> Result<(), String> {
    let current = snapshot(path)?;
    let same = match (initial, current) {
        (None, None) => true,
        (Some((a, bytes)), Some((b, now))) => {
            a.dev() == b.dev()
                && a.ino() == b.ino()
                && a.mode() == b.mode()
                && a.uid() == b.uid()
                && a.gid() == b.gid()
                && bytes == &now
        }
        _ => false,
    };
    if same {
        Ok(())
    } else {
        Err("config changed concurrently; retry after edits finish".into())
    }
}

pub fn update(path: &Path, command: &str, install: bool) -> Result<bool, String> {
    let path = std::path::absolute(path).map_err(|e| e.to_string())?;
    let parent = path.parent().ok_or("config needs a parent directory")?;
    for ancestor in parent.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("config parent must not be a symlink".into());
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
            _ => (),
        }
    }
    let initial = snapshot(&path)?;
    if initial.is_none() && !install {
        return Ok(false);
    }
    let input = initial
        .as_ref()
        .map(|(_, data)| std::str::from_utf8(data))
        .transpose()
        .map_err(|_| "config must be UTF-8")?
        .unwrap_or("");
    let output = edit_config(input, command, install)?;
    if output == input {
        return Ok(false);
    }
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let _lock = LockGuard::acquire(&lock_path(&path))?;
    unchanged(&path, &initial)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(output.as_bytes())
        .map_err(|e| e.to_string())?;
    if let Some((meta, _)) = &initial {
        let temp_meta = temp.as_file().metadata().map_err(|e| e.to_string())?;
        if temp_meta.uid() != meta.uid() || temp_meta.gid() != meta.gid() {
            return Err("cannot preserve config ownership".into());
        }
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(meta.mode()))
            .map_err(|e| e.to_string())?;
    }
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    unchanged(&path, &initial)?;
    if initial.is_none() {
        temp.persist_noclobber(&path).map_err(|e| e.to_string())?;
    } else {
        temp.persist(&path).map_err(|e| e.to_string())?;
    }
    File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(true)
}

fn lock_path(path: &Path) -> PathBuf {
    path.with_extension("toml.lock")
}
struct LockGuard {
    path: PathBuf,
}
impl LockGuard {
    fn acquire(path: &Path) -> Result<Self, String> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("cannot acquire config lock: {e}"))?;
        Ok(Self { path: path.into() })
    }
}
impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn install_absent() {
        assert_eq!(
            edit_config("name = \"x\"\n", "run", true).unwrap(),
            "name = \"x\"\n\n[hooks]\nsession_start = \"run\"\n"
        );
    }
    #[test]
    fn preserves_array_and_removes_exact() {
        let s = "[hooks]\n# keep\nsession_start = [\"a\", \"ab\"]\n";
        let out = edit_config(s, "a", false).unwrap();
        assert!(out.contains("session_start") && out.contains("ab"));
        assert!(out.contains("# keep"));
    }
    #[test]
    fn rejects_types() {
        assert!(edit_config("[hooks]\nsession_start = 1\n", "x", true).is_err());
    }
    #[test]
    fn update_atomic() {
        let d = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let p = d.path().join("config.toml");
        fs::write(&p, "[hooks]\n").unwrap();
        assert!(update(&p, "x", true).unwrap());
        assert!(!update(&p, "x", true).unwrap());
    }
    #[test]
    fn absent_remove_is_identical_and_install_creates() {
        let d = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let p = d.path().join("new.toml");
        assert!(!update(&p, "x", false).unwrap_or(true));
        assert!(update(&p, "x", true).unwrap());
        assert!(p.exists());
    }
    #[test]
    fn nonexistent_removal_is_byte_identical() {
        let s = "[hooks]\nsession_start = [\"a\"] # inline\n";
        assert_eq!(edit_config(s, "missing", false).unwrap(), s);
    }
    #[test]
    fn scalar_inline_comment_survives_install_noop() {
        let s = "[hooks]\nsession_start = \"x\" # keep\n";
        assert_eq!(edit_config(s, "x", true).unwrap(), s);
    }
    #[test]
    fn quotes_spaces_and_apostrophes() {
        assert_eq!(
            hook_command(Path::new("/tmp/my plugin/it's")).unwrap(),
            "\"/tmp/my plugin/it's\" report"
        );
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_config_and_parent() {
        use std::os::unix::fs::symlink;
        let d = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let real = d.path().join("real");
        fs::write(&real, "").unwrap();
        let link = d.path().join("link");
        symlink(&real, &link).unwrap();
        assert!(update(&link, "x", true).is_err());
        let parent = d.path().join("parent");
        symlink(d.path(), &parent).unwrap();
        assert!(update(&parent.join("x"), "x", true).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn preserves_permissions() {
        let d = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let p = d.path().join("x");
        fs::write(&p, "").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o640)).unwrap();
        update(&p, "x", true).unwrap();
        assert_eq!(
            fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }
    #[test]
    fn existing_lock_fails_and_cleans_up_after_success() {
        let d = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let p = d.path().join("x");
        fs::write(&p, "").unwrap();
        let l = lock_path(&p);
        File::create(&l).unwrap();
        assert!(update(&p, "x", true).is_err());
        fs::remove_file(&l).unwrap();
        update(&p, "x", true).unwrap();
        assert!(!l.exists());
    }
}
