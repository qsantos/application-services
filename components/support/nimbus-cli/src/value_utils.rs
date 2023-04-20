// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Result;
use serde_json::Value;

pub(crate) trait CliUtils {
    fn get_str<'a>(&'a self, key: &str) -> Result<&'a str>;
    fn get_array<'a>(&'a self, key: &str) -> Result<&'a Vec<Value>>;
}

impl CliUtils for Value {
    fn get_str<'a>(&'a self, key: &str) -> Result<&'a str> {
        let v = self
            .get(key)
            .ok_or_else(|| anyhow::Error::msg("Expected a string in the JSONObject"))?
            .as_str()
            .ok_or_else(|| anyhow::Error::msg("value is not a string"))?;

        Ok(v)
    }

    fn get_array<'a>(&'a self, key: &str) -> Result<&'a Vec<Value>> {
        let v = self
            .get(key)
            .ok_or_else(|| anyhow::Error::msg("Expected an array in the JSONObject"))?
            .as_array()
            .ok_or_else(|| anyhow::Error::msg("value is not a array"))?;
        Ok(v)
    }
}
