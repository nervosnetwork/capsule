use super::ONE_CKB;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct HumanCapacity(pub u64);

impl From<u64> for HumanCapacity {
    fn from(value: u64) -> HumanCapacity {
        HumanCapacity(value)
    }
}

impl From<HumanCapacity> for u64 {
    fn from(value: HumanCapacity) -> u64 {
        value.0
    }
}

impl Deref for HumanCapacity {
    type Target = u64;
    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl FromStr for HumanCapacity {
    type Err = String;
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts = input
            .trim_end_matches("(CKB)")
            .trim()
            .split('.')
            .collect::<Vec<_>>();
        let mut capacity = ONE_CKB
            * parts
                .first()
                .ok_or_else(|| "Missing input".to_owned())?
                .parse::<u64>()
                .map_err(|err| err.to_string())?;
        if let Some(shannon_str) = parts.get(1) {
            let shannon_str = shannon_str.trim();
            if shannon_str.len() > 8 {
                return Err(format!(
                    "decimal part too long: {} {}",
                    shannon_str.len(),
                    shannon_str
                ));
            }
            let mut shannon = shannon_str.parse::<u32>().map_err(|err| err.to_string())?;
            for _ in 0..(8 - shannon_str.len()) {
                shannon *= 10;
            }
            capacity += u64::from(shannon);
        }
        Ok(capacity.into())
    }
}

impl fmt::Display for HumanCapacity {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let ckb_part = self.0 / ONE_CKB;
        let shannon_part = self.0 % ONE_CKB;
        let shannon_part_string = format!("{:0>8}", shannon_part);
        let mut base = 10;
        let mut suffix_zero = 7;
        for i in 0..8 {
            if shannon_part % base > 0 {
                suffix_zero = i;
                break;
            }
            base *= 10;
        }
        if f.alternate() {
            write!(
                f,
                "{}.{} (CKB)",
                ckb_part,
                &shannon_part_string[..(8 - suffix_zero)]
            )
        } else {
            write!(
                f,
                "{}.{}",
                ckb_part,
                &shannon_part_string[..(8 - suffix_zero)]
            )
        }
    }
}
