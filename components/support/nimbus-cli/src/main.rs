// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod cli;
mod cmd;
mod value_utils;

use clap::Parser;
use cli::{Cli, CliCommand};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
// use clap::{load_yaml, App, ArgMatches};
fn main() -> Result<()> {
    let cmds = get_command_from_cli(std::env::args_os(), &std::env::current_dir()?)?;
    for c in cmds {
        let success = cmd::process_cmd(&c)?;
        if !success {
            bail!("Failed");
        }
    }
    Ok(())
}

fn get_command_from_cli<I, T>(_args: I, _cwd: &Path) -> Result<Vec<AppCommand>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse();

    let app = LaunchableApp::try_from(&cli)?;
    let cmd = AppCommand::try_from(&app, &cli)?;

    Ok(if let AppCommand::List { .. } = cmd {
        vec![cmd]
    } else {
        vec![AppCommand::Kill { app: app.clone() }, cmd]
    })
}

#[derive(Clone)]
enum LaunchableApp {
    Android {
        package_name: String,
        activity_name: String,
        device_id: Option<String>,
    },
    IOS {
        device_id: String,
        app_id: String,
    },
}

impl TryFrom<&Cli> for LaunchableApp {
    type Error = anyhow::Error;
    fn try_from(value: &Cli) -> Result<Self> {
        let app = value.app.as_str();
        let channel = value.channel.as_str();
        let prefix = match app {
            "fenix" => Some("org.mozilla"),
            "focus_android" => Some("org.mozilla"),
            "firefox_ios" => Some("org.mozilla.ios"),
            "focus_ios" => Some("org.mozilla.ios"),
            _ => None,
        };

        let suffix = match app {
            "fenix" => Some(match channel {
                "developer" => "fenix.debug",
                "nightly" => "fenix",
                "beta" => "firefox_beta",
                "release" => "firefox",
                _ => bail!(format!("Application {} has no channel '{}'. Try one of developer, nightly, beta or release", app, channel)),
            }),
            "focus_android" => Some(match channel {
                "developer" => "focus.debug",
                "nightly" => "focus.nightly",
                "beta" => "focus.beta",
                "release" => "focus",
                _ => bail!(format!("Application {} has no channel '{}'. Try one of developer, nightly, beta or release", app, channel)),
            }),
            "firefox_ios" => Some(match channel {
                "developer" => "Fennec",
                "beta" => "FirefoxBeta",
                "release" => "Firefox",
                _ => bail!(format!("Application {} has no channel '{}'. Try one of developer, beta or release", app, channel)),
            }),
            "focus_ios" => Some(match channel {
                "developer" => "Focus",
                "beta" => "Focus",
                "release" => "Focus",
                _ => bail!(format!("Application {} has no channel '{}'. Try one of developer, beta or release", app, channel)),
            }),
            _ => None,
        };

        Ok(match (app, prefix, suffix) {
            ("fenix", Some(prefix), Some(suffix)) => Self::Android {
                package_name: format!("{}.{}", prefix, suffix),
                activity_name: ".App".to_string(),
                device_id: None,
            },
            ("focus_android", Some(prefix), Some(suffix)) => Self::Android {
                package_name: format!("{}.{}", prefix, suffix),
                activity_name: "org.mozilla.focus.activity.MainActivity".to_string(),
                device_id: None,
            },
            ("firefox_ios", Some(prefix), Some(suffix)) => Self::IOS {
                app_id: format!("{}.{}", prefix, suffix),
                device_id: "booted".to_string(),
            },
            ("focus_ios", Some(prefix), Some(suffix)) => Self::IOS {
                app_id: format!("{}.{}", prefix, suffix),
                device_id: "booted".to_string(),
            },
            _ => unimplemented!(),
        })
    }
}

enum AppCommand {
    Unenroll {
        app: LaunchableApp,
    },

    Enroll {
        app: LaunchableApp,
        experiment: ExperimentSource,
        branch: String,
        preserve_targeting: bool,
        rollouts: Vec<ExperimentSource>,
    },

    TestFeature {
        app: LaunchableApp,
        experiment: ExperimentSource,
    },

    List {
        app: LaunchableApp,
        list: ExperimentListSource,
    },

    Kill {
        app: LaunchableApp,
    },
}

impl AppCommand {
    fn try_from(app: &LaunchableApp, cli: &Cli) -> Result<Self> {
        let app = app.clone();
        Ok(match &cli.command {
            CliCommand::Enroll {
                experiment,
                branch,
                preserve_targeting,
                ..
            } => {
                let experiment = ExperimentSource::try_from(experiment.as_str())?;
                // let rollouts = rollouts.iter().map(|s| ExperimentSource::try_from(s.as_str())?).collect();
                let branch = branch.to_owned();
                let preserve_targeting = *preserve_targeting;
                Self::Enroll {
                    app,
                    experiment,
                    branch,
                    preserve_targeting,
                    rollouts: Default::default(),
                }
            }
            CliCommand::TestFeature {
                feature_id,
                feature_file,
            } => {
                let experiment = ExperimentSource::FromFeatureFiles {
                    feature_id: feature_id.clone(),
                    files: feature_file.clone(),
                };
                AppCommand::TestFeature { app, experiment }
            }
            CliCommand::Unenroll => AppCommand::Unenroll { app },
            CliCommand::List { server } => {
                let list = server.to_owned().unwrap_or_default();
                let list = list.as_str().try_into()?;
                AppCommand::List { app, list }
            }
            _ => unimplemented!(),
        })
    }
}

#[derive(Debug)]
enum ExperimentSource {
    FromList {
        slug: String,
        list: ExperimentListSource,
    },

    FromFeatureFiles {
        feature_id: String,
        files: Vec<PathBuf>,
    },
}

#[derive(Debug)]
enum ExperimentListSource {
    FromRemoteSettings { endpoint: String, is_preview: bool },
}

impl ExperimentListSource {
    fn try_from_pair(server: &str, preview: &str) -> Result<Self> {
        let stage = "https://settings.stage.mozaws.net";
        let release = "https://firefox.settings.services.mozilla.com";
        let is_preview = preview == "preview";

        let endpoint = match server {
            "" | "release" => release,
            "stage" => stage,
            _ => bail!("Only stage or release currently supported"),
        }
        .to_string();

        Ok(Self::FromRemoteSettings {
            endpoint,
            is_preview,
        })
    }
}

impl TryFrom<&str> for ExperimentListSource {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        let tokens: Vec<&str> = value.splitn(3, '/').collect();
        let tokens = tokens.as_slice();
        Ok(match tokens {
            [""] => Self::try_from_pair("", "")?,
            ["preview"] => Self::try_from_pair("", "preview")?,
            [server] => Self::try_from_pair(server, "")?,
            [server, "preview"] => Self::try_from_pair(server, "preview")?,
            _ => bail!(format!("Can't unpack '{}' into an experiment; try preview, release, stage, or stage/preview", value)),
        })
    }
}

impl TryFrom<&str> for ExperimentSource {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        let tokens: Vec<&str> = value.splitn(3, '/').collect();
        let tokens = tokens.as_slice();
        Ok(match tokens {
            [slug] => Self::FromList {
                slug: slug.to_string(),
                list: ExperimentListSource::try_from_pair("", "")?,
            },
            ["preview", slug] => Self::FromList {
                slug: slug.to_string(),
                list: ExperimentListSource::try_from_pair("", "preview")?,
            },
            [server, slug] => Self::FromList {
                slug: slug.to_string(),
                list: ExperimentListSource::try_from_pair(server, "")?,
            },
            [server, "preview", slug] => Self::FromList {
                slug: slug.to_string(),
                list: ExperimentListSource::try_from_pair(server, "preview")?,
            },
            _ => bail!(format!(
                "Can't unpack '{}' into an experiment; try preview/SLUG or stage/SLUG, or stage/preview/SLUG",
                value
            )),
        })
    }
}
