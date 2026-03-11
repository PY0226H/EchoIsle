use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{AppError, ChatFile};
use sha1::{Digest, Sha1};

impl ChatFile {
    pub fn new(filename: &str, data: &[u8]) -> Self {
        let hash = Sha1::digest(data);
        Self {
            ext: filename.rsplit('.').next().unwrap_or("txt").to_string(),
            hash: hex::encode(hash),
        }
    }

    pub fn url(&self) -> String {
        format!("/files/{}", self.hash_to_path())
    }

    pub fn path(&self, base_dir: &Path) -> PathBuf {
        base_dir.join(self.hash_to_path())
    }

    // split hash into 3 parts, first 2 with 3 chars
    fn hash_to_path(&self) -> String {
        let (part1, part2) = self.hash.split_at(3);
        let (part2, part3) = part2.split_at(3);
        format!("{}/{}/{}.{}", part1, part2, part3, self.ext)
    }
}

impl FromStr for ChatFile {
    type Err = AppError;

    // convert /files/339/807/e635afbeab088ce33206fdf4223a6bb156.png to ChatFile
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(s) = s.strip_prefix("/files/") else {
            return Err(AppError::ChatFileError(format!(
                "Invalid chat file path: {}",
                s
            )));
        };

        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return Err(AppError::ChatFileError(format!(
                "File path {} does not valid",
                s
            )));
        }

        let Some((part3, ext)) = parts[2].split_once('.') else {
            return Err(AppError::ChatFileError(format!(
                "Invalid file name: {}",
                parts[2]
            )));
        };

        let hash = format!("{}{}{}", parts[0], parts[1], part3);
        Ok(Self {
            ext: ext.to_string(),
            hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_file_new_should_work() {
        let file = ChatFile::new("test.txt", b"hello world");
        assert_eq!(file.ext, "txt");
        assert_eq!(file.hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }
}
