// Windows Credential Manager 封装（通过 keyring crate）
//
// 服务名固定 "albumsync"，username 为 "user@host:port" 形式，
// 保证多设备同 username 不冲突（虽然 v0.1 只支持 1 设备）

use keyring::Entry;

use crate::error::{AlbumError, Result};

const SERVICE: &str = "albumsync";

fn key_for(username: &str, host: &str, port: u16) -> String {
    format!("{username}@{host}:{port}")
}

pub fn save_password(username: &str, host: &str, port: u16, password: &str) -> Result<()> {
    let entry = Entry::new(SERVICE, &key_for(username, host, port))
        .map_err(|e| AlbumError::Keyring(e.to_string()))?;
    entry
        .set_password(password)
        .map_err(|e| AlbumError::Keyring(e.to_string()))?;
    Ok(())
}

pub fn load_password(username: &str, host: &str, port: u16) -> Result<String> {
    let entry = Entry::new(SERVICE, &key_for(username, host, port))
        .map_err(|e| AlbumError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| AlbumError::Keyring(format!("读取凭据失败: {e}")))
}

pub fn delete_password(username: &str, host: &str, port: u16) -> Result<()> {
    let entry = Entry::new(SERVICE, &key_for(username, host, port))
        .map_err(|e| AlbumError::Keyring(e.to_string()))?;
    // 已不存在视为成功（幂等）。keyring 不导出错误变体，靠字符串识别。
    if let Err(e) = entry.delete_credential() {
        let msg = e.to_string();
        if msg.contains("no entry") || msg.to_lowercase().contains("not found") {
            return Ok(());
        }
        return Err(AlbumError::Keyring(msg));
    }
    Ok(())
}
