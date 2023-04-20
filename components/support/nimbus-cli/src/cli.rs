// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    #[arg(short, long, value_name = "APP")]
    pub(crate) app: String,

    #[arg(short, long, value_name = "CHANNEL")]
    pub(crate) channel: String,

    #[command(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Subcommand, Clone)]
pub(crate) enum CliCommand {
    /// Enroll into an experiment or a rollout
    Enroll {
        #[arg(value_name = "SLUG")]
        experiment: String,
        #[arg(short, long, value_name = "BRANCH")]
        branch: String,

        #[arg(value_name = "SLUG")]
        rollouts: Vec<String>,

        #[arg(short, long, default_value = "false")]
        preserve_targeting: bool,
    },

    /// Unenroll from all experiments and rollouts
    Unenroll,

    /// Create a rollout to test a particular feature configuration
    TestFeature {
        /// The feature id
        feature_id: String,
        /// A JSON files containing the feature configuration
        feature_file: Vec<PathBuf>,
    },

    /// Load and apply experiments from a file
    ApplyFile { recipes_file: PathBuf },

    List {
        /// A server slug e.g. preview, release, stage, stage/preview
        server: Option<String>,
    },
}
