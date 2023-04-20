// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::process::Command;

use crate::{
    value_utils::CliUtils, AppCommand, ExperimentListSource, ExperimentSource, LaunchableApp,
};
use anyhow::Result;
use serde_json::{json, Value};

pub(crate) fn process_cmd(cmd: &AppCommand) -> Result<bool> {
    let status = match cmd {
        AppCommand::Kill { app } => app.kill_app()?,
        AppCommand::Unenroll { app } => app.unenroll_all()?,
        AppCommand::Enroll {
            app,
            experiment,
            branch,
            preserve_targeting,
            rollouts,
        } => app.enroll(experiment, branch, preserve_targeting, rollouts)?,
        AppCommand::List { app, list } => list.ls(&app)?,
        _ => unimplemented!("No commands implemented yet"),
    };

    Ok(status)
}

impl LaunchableApp {
    fn exe(&self) -> Result<Command> {
        Ok(match self {
            Self::Android { device_id, .. } => {
                let mut cmd = Command::new("adb");
                if let Some(id) = device_id {
                    cmd.args(["-s", id]);
                }
                cmd
            }
            Self::IOS { .. } => {
                let mut cmd = Command::new("xcrun");
                cmd.arg("simctl");
                cmd
            }
        })
    }

    fn kill_app(&self) -> Result<bool> {
        Ok(match self {
            Self::Android { package_name, .. } => self
                .exe()?
                .arg("shell")
                .arg(format!("am force-stop {}", package_name))
                .spawn()?
                .wait()?
                .success(),
            Self::IOS { app_id, device_id } => {
                let _ = self
                    .exe()?
                    .args(["terminate", device_id, app_id])
                    .spawn()?
                    .wait();
                true
            }
        })
    }

    fn unenroll_all(&self) -> Result<bool> {
        let payload = json! {{ "data": [] }};
        Ok(match self {
            Self::Android { .. } => self.android_start(&payload)?.spawn()?.wait()?.success(),
            Self::IOS { .. } => self.ios_start(&payload)?.spawn()?.wait()?.success(),
        })
    }

    fn enroll(
        &self,
        experiment: &ExperimentSource,
        branch: &str,
        preserve_targeting: &bool,
        rollouts: &[ExperimentSource],
    ) -> Result<bool> {
        let experiment = Value::try_from(experiment)?;

        let payload = json! {{ "data": [experiment] }};
        Ok(match self {
            Self::Android { .. } => self.android_start(&payload)?.spawn()?.wait()?.success(),
            Self::IOS { .. } => self.ios_start(&payload)?.spawn()?.wait()?.success(),
        })
    }

    fn android_start(&self, json: &Value) -> Result<Command> {
        if let Self::Android {
            package_name,
            activity_name,
            ..
        } = self
        {
            let mut cmd = self.exe()?;
            // TODO add adb pass through args for debugger, wait for debugger etc.
            let sh = format!(
                r#"am start -n {}/{} \
        -a android.intent.action.MAIN \
        -c android.intent.category.LAUNCHER \
        --esn nimbus-cli \
        --ei version 1 \
        --es experiments '{}'"#,
                package_name, activity_name, json
            );
            cmd.arg("shell").arg(&sh);
            println!("adb shell \"{}\"", sh);
            Ok(cmd)
        } else {
            unreachable!();
        }
    }

    fn ios_start(&self, json: &Value) -> Result<Command> {
        if let Self::IOS { app_id, device_id } = self {
            let mut cmd = self.exe()?;
            cmd.args(["launch", device_id, app_id])
                .arg("--nimbus-cli")
                .args(["--version", "1"])
                .args(["--experiments", &json.to_string()]);

            let sh = format!(
                r#"xcrun simctl launch {} {} \
        --nimbus-cli \
        --version 1 \
        --experiments '{}'"#,
                device_id,
                app_id,
                json.to_string()
            );

            println!("{}", sh);
            Ok(cmd)
        } else {
            unreachable!()
        }
    }
}

impl TryFrom<&ExperimentSource> for Value {
    type Error = anyhow::Error;

    fn try_from(value: &ExperimentSource) -> Result<Value> {
        Ok(match value {
            ExperimentSource::FromList { slug, list } => {
                let value = Value::try_from(list)?;
                try_find_experiment(&value, &slug)?
            }
            _ => {
                unimplemented!("Feature file not implemented");
            }
        })
    }
}

fn try_find_experiment(value: &Value, slug: &str) -> Result<Value> {
    let array = try_extract_data_list(value)?;
    let exp = array
        .iter()
        .find(|exp| {
            if let Some(Value::String(this_slug)) = exp.get("slug") {
                &slug == this_slug
            } else {
                false
            }
        })
        .ok_or_else(|| anyhow::Error::msg(format!("No experiment with slug {}", slug)))?;

    Ok(exp.clone())
}

fn try_extract_data_list(value: &Value) -> Result<Vec<Value>> {
    assert!(value.is_object());
    let value = value
        .as_object()
        .ok_or_else(|| anyhow::Error::msg("JSON is not an object"))?;
    Ok(value
        .get("data")
        .ok_or_else(|| anyhow::Error::msg("No data property from JSON"))?
        .as_array()
        .ok_or_else(|| anyhow::Error::msg("data property is not an array in JSON"))?
        .to_vec())
}

impl TryFrom<&ExperimentListSource> for Value {
    type Error = anyhow::Error;

    fn try_from(value: &ExperimentListSource) -> Result<Value> {
        Ok(match value {
            ExperimentListSource::FromRemoteSettings {
                endpoint,
                is_preview,
            } => {
                use rs_client::{Client, ClientConfig};
                viaduct_reqwest::use_reqwest_backend();
                let collection_name = if *is_preview {
                    "nimbus-preview".to_string()
                } else {
                    "nimbus-mobile-experiments".to_string()
                };
                let config = ClientConfig {
                    server_url: Some(endpoint.clone()),
                    bucket_name: None,
                    collection_name,
                };
                let client = Client::new(config)?;

                let response = client.get_records()?;
                response.json::<Value>()?
            }
        })
    }
}

impl ExperimentListSource {
    fn ls(&self, _app: &LaunchableApp) -> Result<bool> {
        let value: Value = self.try_into()?;
        let array = try_extract_data_list(&value)?;
        for exp in array {
            let slug = exp.get_str("slug")?;
            let features: Vec<_> = exp
                .get_array("featureIds")?
                .iter()
                .flat_map(|f| f.as_str())
                .collect();
            let branches: Vec<_> = exp
                .get_array("branches")?
                .iter()
                .flat_map(|b| {
                    b.get("slug")
                        .expect("Expecting a branch with a slug")
                        .as_str()
                })
                .collect();
            println!("{} branches={:?} features={:?}", slug, branches, features);
        }
        Ok(true)
    }
}
