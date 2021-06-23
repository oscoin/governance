#!/usr/bin/env -S node --require ts-node/register/transpile-only

// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import * as fs from "fs/promises";
import * as path from "path";
import * as os from "os";
import execa from "execa";
import yargs from "yargs";
import standardVersion from "standard-version";
import prompts from "prompts";
import chalk from "chalk";
import * as semver from "semver";
import fetch from "node-fetch";

const currentVersion: string = require("../package.json").version;

function main() {
  if (!semver.gte(process.version, "14.14.0")) {
    throw new Error(
      "You’re using an outdated version of Node. This script requires at least v14.14.0"
    );
  }

  yargs
    .command(createRc)
    .command(createQaIssues)
    .command(publishRcBinaries)
    .command(publish)
    .command(announcements)
    .command(setLatestRelease)
    // Don’t show a version and the --version flag
    .version(false)
    .strict()
    // For `UserError` we don’t show the stack trace. We also don’t show the help
    // when an error is thrown.
    .fail((msg, err, yargs) => {
      if (err === undefined) {
        yargs.showHelp("error");
        console.error("");
        console.error(msg);
      } else if (err instanceof UserError) {
        console.error(err.message);
      } else {
        console.error(err);
      }
      process.exit(1);
    })
    .wrap(Math.min(100, yargs.terminalWidth()))
    .demandCommand().argv;
}

// Error that is shown without a stacktrace to the user
class UserError extends Error {
  constructor(public message: string) {
    super(message);
  }
}

type ReleaseType = "major" | "minor" | "patch";

interface StartOptions {
  type: ReleaseType;
  revision: string;
}

const createRc: yargs.CommandModule<unknown, StartOptions> = {
  command: "create-rc <type> [revision]",
  describe:
    "Create a release candidate branch and a commit that updates the version and changelog",
  builder: yargs => {
    return yargs
      .positional("type", {
        demandOption: true,
        choices: ["major", "minor", "patch"] as ReleaseType[],
        required: true,
      })
      .positional("revision", {
        default: "origin/master",
        describe: "The git revision to start the release from",
      });
  },
  handler: async options => {
    const newVersion = semver.inc(currentVersion, options.type);
    await runVerbose("git", [
      "checkout",
      "-b",
      `release-candidate/v${newVersion}`,
      options.revision,
    ]);
    await standardVersion({
      infile: "./CHANGELOG.md",
      silent: true,
      skip: { tag: true },
      sign: true,
      releaseAs: options.type,
    });
    console.log(
      `✔ standard-version --infile ./CHANGELOG.md --silent --sign --release-as ${options.releaseType}`
    );

    await promptContinue("Create release pull request?");
    await runVerbose("hub", ["pull-request", "--push", "--draft", "--no-edit"]);
  },
};

const createQaIssues: yargs.CommandModule<unknown, unknown> = {
  command: "create-qa-issues",
  describe: "Create issues for QA on the release candidate",
  handler: async () => {
    const version = await getReleaseCandidateVersion();

    const qaTemplate = await fs.readFile(
      path.resolve(__dirname, "..", "QA.md"),
      "utf8"
    );
    const qaIssueBody = qaTemplate.replace(/X\.X\.X/g, version);

    await promptContinue("Create QA issues?");

    await execa("hub", ["issue", "create", "--file", "-"], {
      stdio: ["pipe", "inherit", "inherit"],
      input: `QA: v${version} Linux\n\n${qaIssueBody}`,
    });

    await execa("hub", ["issue", "create", "--file", "-"], {
      stdio: ["pipe", "inherit", "inherit"],
      input: `QA: v${version} MacOS\n\n${qaIssueBody}`,
    });
  },
};

const publishRcBinaries: yargs.CommandModule<unknown, unknown> = {
  command: "publish-rc-binaries",
  describe: "Publish release candidate build artifacts",
  handler: async () => {
    const version = await getReleaseCandidateVersion();
    const buildStatuses = await getBuildStatuses();
    const linuxBuildUuid = getBuildkiteUuid(
      "buildkite/radicle-upstream/test-and-package-app-on-linux",
      buildStatuses
    );
    const macosBuildUuid = getBuildkiteUuid(
      "buildkite/radicle-upstream/test-and-package-app-on-macos",
      buildStatuses
    );

    const linuxBinary = `releases.radicle.xyz/radicle-upstream-${version}-rc.AppImage`;
    await runVerbose("gsutil", [
      "cp",
      `gs://builds.radicle.xyz/radicle-upstream/${linuxBuildUuid}/dist/radicle-upstream-${version}.AppImage`,
      `gs://${linuxBinary}`,
    ]);

    const macosBinary = `releases.radicle.xyz/radicle-upstream-${version}-rc.dmg`;
    await runVerbose("gsutil", [
      "cp",
      `gs://builds.radicle.xyz/radicle-upstream/${macosBuildUuid}/dist/radicle-upstream-${version}.dmg`,
      `gs://${macosBinary}`,
    ]);

    console.log("Release candidate binaries published as");
    console.log(`  https://${linuxBinary}`);
    console.log(`  https://${macosBinary}`);
  },
};

const publish: yargs.CommandModule<unknown, unknown> = {
  command: "publish",
  describe: "Publish the release candidate binaries and create a release tag",
  handler: async () => {
    const version = await getReleaseCandidateVersion();

    await runVerbose("gsutil", [
      "cp",
      // don't overwrite existing files
      "-n",
      `gs://releases.radicle.xyz/radicle-upstream-${version}-rc.AppImage`,
      `gs://releases.radicle.xyz/radicle-upstream-${version}.AppImage`,
    ]);
    await runVerbose("gsutil", [
      "cp",
      // don't overwrite existing files
      "-n",
      `gs://releases.radicle.xyz/radicle-upstream-${version}-rc.dmg`,
      `gs://releases.radicle.xyz/radicle-upstream-${version}.dmg`,
    ]);

    await runVerbose("git", ["tag", `v${version}`]);
    await runVerbose("git", ["push", "origin", "tag", `v${version}`]);
  },
};

const announcements: yargs.CommandModule<unknown, unknown> = {
  command: "announcements",
  describe: "Show templates for announcing the release",
  handler: async () => {
    const version = await getReleaseCandidateVersion();
    const post = `# Radicle Upstream v${version} is out! 🎉

>> highlight some of the changes here <<

You can find the complete list of changes in our [changelog][1].

Here are packages for all our supported platforms:

- [macOS][2]
- [Linux][3]

For more information on how to use Radicle, check out our [documentation][4].

For support, you can reach us in the [#support channel][5] of our Matrix chat or in the #help category of this forum.

If you encounter a bug, please [open an issue][6].

[1]: https://github.com/radicle-dev/radicle-upstream/blob/v${version}/CHANGELOG.md
[2]: https://releases.radicle.xyz/radicle-upstream-${version}.dmg
[3]: https://releases.radicle.xyz/radicle-upstream-${version}.AppImage
[4]: https://docs.radicle.xyz/docs/what-is-radicle.html
[5]: https://matrix.radicle.community/#/room/#support:radicle.community
[6]: https://github.com/radicle-dev/radicle-upstream/issues`;

    console.log(chalk.cyan.bold("❱ Discourse"));
    console.log(
      chalk.cyan("Post this to https://radicle.community/c/announcements")
    );
    console.log(post);
    console.log();

    console.log(chalk.cyan.bold("❱ Discord"));
    console.log(chalk.cyan("Post this to #general on discord"));
    const communityVersion = version.replace(/\./g, "-");
    console.log(`Radicle Upstream v${version} is out! 🎉
>> highlight some of the changes here <<
https://radicle.community/t/radicle-upstream-v${communityVersion}-is-out`);
  },
};

const setLatestRelease: yargs.CommandModule<unknown, unknown> = {
  command: "set-latest-release",
  describe: "",
  handler: async () => {
    const version = await getReleaseCandidateVersion();
    const versionDash = version.replace(/\./g, "-");
    const announcementUrl = `https://radicle.community/t/radicle-upstream-v${versionDash}-is-out`;
    const response = await fetch(
      `https://radicle.community/t/radicle-upstream-v${versionDash}-is-out`
    );
    if (!response.ok) {
      throw new UserError(
        `Announcement url ${announcementUrl} does not exist. Response status is ${response.status}`
      );
    }
    await withTempDir(async tempDir => {
      const fileName = "latest.json";
      const latestPath = path.join(tempDir, fileName);
      await fs.writeFile(
        latestPath,
        JSON.stringify(
          {
            version,
            announcementUrl,
          },
          null,
          2
        ),
        "utf8"
      );

      await runVerbose("gsutil", [
        "-h",
        "cache-control:no-cache",
        "cp",
        latestPath,
        `gs://releases.radicle.xyz/${fileName}`,
      ]);
    });
  },
};

interface BuildStatus {
  status: string;
  id: string;
  url: string;
}

// Return the Github build statuses for the current branch.
async function getBuildStatuses(): Promise<BuildStatus[]> {
  const result = await execa("hub", ["ci-status", "--format", "%S %t %U\n"], {
    // hub ci-status returns exit code 1 or 2 when some of the statuses are
    // pending or failed.
    reject: false,
  });

  if (result.exitCode > 2 || result.signal) {
    throw result;
  }

  const lines = result.stdout.split("\n");
  return lines.map(line => {
    const [status, id, url] = line.split(" ");
    return { status, id, url };
  });
}

// Return the Buildkite build UUID extracted from the build status with
// the given ID. Throws an error when the build status is not
// `success`.
function getBuildkiteUuid(
  buildId: string,
  buildStatuses: BuildStatus[]
): string {
  const maybeBuildStatus = buildStatuses.find(
    buildStatus => buildStatus.id === buildId
  );
  if (!maybeBuildStatus) {
    throw new UserError(`Build status not found: ${buildId}`);
  }

  if (maybeBuildStatus.status !== "success") {
    throw new UserError(
      `Invalid build status ${maybeBuildStatus} for ${buildId}`
    );
  }

  const urlRe = new RegExp(
    "https://buildkite.com/monadic/radicle-upstream/builds/\\d+#([a-z0-9-]+)"
  );
  const match = maybeBuildStatus.url.match(urlRe);
  if (!match) {
    throw new Error(
      `Failed to extract build UUID from URL ${maybeBuildStatus.url}`
    );
  }
  return match[1];
}

// Ensure that we are on a release candidate branch and return the version
// associated with that branch.
async function getReleaseCandidateVersion(): Promise<string> {
  const result = await execa("git", ["branch", "--show-current"]);
  const match = result.stdout.match(/^release-candidate\/v(\d+\.\d+\.\d+)$/);
  if (!match) {
    throw new UserError(
      "You are not on a release candidate branch.\n" +
        "Run `./scripts/release.ts start` or switch to an existing branch"
    );
  }
  return match[1];
}

// Show `message` to the user and ask them to confirm. Throws an error
// if the user does not confirm.
async function promptContinue(message: string) {
  const response = await prompts({
    type: "confirm",
    name: "value",
    message,
    initial: false,
  });
  if (response.value !== true) {
    throw new UserError("Aborted by user");
  }
}

// Run a command and print the command line to the output
async function runVerbose(command: string, args: string[] = []): Promise<void> {
  console.log(`* ${command} ${args.join(" ")}`);
  await execa(command, args, {
    stdio: "inherit",
  });
  console.log(`✔ ${command} ${args.join(" ")}`);
}

// Create a temporary directory and pass it to the callback. Ensure
// that the directory is removed when the callback finishes, even if it
// throws.
async function withTempDir(
  cb: (tempDir: string) => Promise<void>
): Promise<void> {
  const tempDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "radicle-dev-set-latest-release-")
  );
  try {
    await cb(tempDir);
  } finally {
    await fs.rm(tempDir, { recursive: true });
  }
}

main();
