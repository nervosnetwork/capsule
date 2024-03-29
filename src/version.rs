use anyhow::{anyhow, Error};
use std::str::FromStr;
use std::string::ToString;

pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u16,
    pub commit_id: String,
    pub pre: String,
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // parse commit id
        let items: Vec<_> = s.split_ascii_whitespace().collect();
        if items.len() > 2 || items.is_empty() {
            return Err(anyhow!(
                "unexpected parts after split whitespaces: {}",
                items.len()
            ));
        }
        let commit_id = items.get(1).map(|s| s.to_string()).unwrap_or_default();
        // parse pre
        let items: Vec<_> = items[0].split('-').collect();
        if items.len() > 2 || items.is_empty() {
            return Err(anyhow!("unexpected parts after split '-': {}", items.len()));
        }
        let pre = items.get(1).map(|s| s.to_string()).unwrap_or_default();
        // parse versions
        let items: Vec<_> = items[0].split('.').collect();
        if items.len() > 3 || items.is_empty() {
            return Err(anyhow!("unexpected parts after split '.': {}", items.len()));
        }
        let major = items[0].parse()?;
        let minor = items[1].parse()?;
        let patch = items[2].parse()?;

        Ok(Self {
            major,
            minor,
            patch,
            pre,
            commit_id,
        })
    }
}
impl ToString for Version {
    fn to_string(&self) -> String {
        let mut version = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.pre.is_empty() {
            version.push('-');
            version.push_str(&self.pre);
        }
        version.push(' ');
        version.push_str(&self.commit_id);
        version.trim().to_string()
    }
}

impl Version {
    pub fn is_compatible(&self, version: &Version) -> bool {
        if self.major == 0 {
            version.major == 0 && self.minor == version.minor && self.patch >= version.patch
        } else {
            self.major == version.major
                && (self.minor, self.patch) >= (version.minor, version.patch)
        }
    }

    pub fn current() -> Self {
        let major = env!("CARGO_PKG_VERSION_MAJOR")
            .parse::<u8>()
            .expect("CARGO_PKG_VERSION_MAJOR parse success");
        let minor = env!("CARGO_PKG_VERSION_MINOR")
            .parse::<u8>()
            .expect("CARGO_PKG_VERSION_MINOR parse success");
        let patch = env!("CARGO_PKG_VERSION_PATCH")
            .parse::<u16>()
            .expect("CARGO_PKG_VERSION_PATCH parse success");
        let pre = env!("CARGO_PKG_VERSION_PRE").to_string();
        let commit_id = env!("COMMIT_ID").to_string();
        Self {
            major,
            minor,
            patch,
            pre,
            commit_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn test_version_compatible() {
        assert!("0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"0.9.1".parse().unwrap()));
        assert!("0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"0.9.0".parse().unwrap()));
        assert!(!"0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"0.9.2".parse().unwrap()));
        assert!(!"0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.2.0".parse().unwrap()));
        assert!(!"0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"0.8.0".parse().unwrap()));
        assert!(!"0.9.1"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"0.10.0".parse().unwrap()));

        assert!("1.0.2"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.0.2".parse().unwrap()));
        assert!("1.0.2"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.0.1".parse().unwrap()));
        assert!("1.1.0"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.0.9".parse().unwrap()));
        assert!(!"1.2.2"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.3.0".parse().unwrap()));
        assert!(!"1.2.2"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"1.3.3".parse().unwrap()));
        assert!(!"1.2.2"
            .parse::<Version>()
            .unwrap()
            .is_compatible(&"2.0.0".parse().unwrap()));
    }
}
