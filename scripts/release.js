#!/usr/bin/env node

const util = require("util");
const exec = util.promisify(require("child_process").exec);

const VERSION_MATCH = /bumping version in package.json from (.*) to (.*)/;
const PULL_REQUEST_MATCH =
  "https://github.com/radicle-dev/radicle-upstream/pull/(.*)";

const SV_COMMAND = "yarn run standard-version --infile ./CHANGELOG.md";

async function main() {
  console.log();

  const svResult = await exec(`${SV_COMMAND} --dry-run`);
  const toVersion = `v${svResult.stdout.match(VERSION_MATCH)[2]}`;

  try {
    await exec("hub --version");
  } catch (error) {
    if (error.stderr.match("command not found")) {
      console.log("Please install missing dependencies:");
      console.log("  - https://github.com/github/hub");
    } else {
      throw error;
    }
  }

  if (process.argv[2] !== "--finalize") {
    console.log(`Cutting release ${toVersion}:\n`);

    await exec("git checkout master");
    console.log("  ✔ git checkout master");

    await exec(
      `git branch release-${toVersion} && git checkout release-${toVersion}`
    );
    console.log(
      `  ✔ git branch release-${toVersion} && git checkout release-${toVersion}`
    );

    await exec(SV_COMMAND);
    console.log(`  ✔ ${SV_COMMAND}`);

    await exec(`git push origin release-${toVersion}`);
    console.log(`  ✔ git push origin release-${toVersion}`);

    const prResult = await exec("hub pull-request -p --no-edit");
    console.log("  ✔ hub pull-request -p --no-edit");

    const prUrl = prResult.stdout.split("\n").slice(-2)[0];
    const pullRequestId = prUrl.match(PULL_REQUEST_MATCH)[1];

    console.log();
    console.log("Now fix up CHANGELOG.md if necessary and update QA.md");
    console.log("to cover the latest changes in functionality.");
    console.log();
    console.log("When everything is in shape, ask a peer to review the");
    console.log("pull request, but don't merge it via the GitHub UI:");
    console.log();
    console.log(`  👉 ${prUrl}`);
    console.log();
    console.log("Finally, complete the release by running:");
    console.log();
    console.log(`  👉 yarn release --finalize ${toVersion} ${pullRequestId}`);
  } else {
    const toVersion = process.argv[4];
    const pullRequestId = process.argv[5];

    if (toVersion === undefined || pullRequestId === undefined) {
      console.log("This command should not be run stand-alone.");
      console.log("You should run `yarn release` and follow the instructions.");
      console.log();
      return;
    }

    console.log(`Finalizing release ${toVersion}:`);
    console.log();

    const mergeResult = await exec(
      `hub api -XPUT "repos/radicle-dev/radicle-upstream/pulls/${pullRequestId}/merge" --raw-field 'merge_method=squash'`
    );
    console.log(
      `  ✔ hub api -XPUT "repos/radicle-dev/radicle-upstream/pulls/${pullRequestId}/merge"`
    );
    const releaseCommitSHA = JSON.parse(mergeResult.stdout).sha;

    await exec("git checkout master && git pull");
    console.log("  ✔ git checkout master && git pull");

    await exec(`git tag ${toVersion} ${releaseCommitSHA}`);
    console.log(`  ✔ git tag ${toVersion} ${releaseCommitSHA}`);

    await exec(`git push --tags`);
    console.log(`  ✔ git push --tags`);
    console.log();
    console.log(`Release ${toVersion} successfully completed! 👏 🎉 🚀`);
    console.log();
  }

  console.log();
}

main();
