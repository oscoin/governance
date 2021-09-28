// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import * as path from "path";

import * as commands from "cypress/support/commands";
import * as nodeManager from "cypress/support/nodeManager";

const commitMessage = "Adding something new";
const patchId = "my-patch/fix";
const patchTitle = "Title";
const patchDescription = "Description.";

context("patches", () => {
  it("shows the empty screen if there are no patches", () => {
    commands.resetProxyState();
    commands.onboardUser();
    commands.createProjectWithFixture();
    cy.visit("./public/index.html");
    commands.pick("project-list-entry-platinum").click();
    commands.pick("patches-tab").click();
    commands.pick("patches-tab", "counter").should("not.exist");
    commands
      .pickWithContent(
        ["empty-state"],
        "There are no patches to show at the moment. If you’re looking for someone’s patch, be sure to add their Device ID as a remote using the dropdown above."
      )
      .should("exist");
    commands
      .pickWithContent(["patch-modal-toggle"], "New Patch")
      .should("exist");
  });

  it("shows annotated patches", () => {
    commands.resetProxyState();
    commands.withTempDir(tempDirPath => {
      nodeManager.withOneOnboardedNode(
        { dataDir: tempDirPath, handle: "rudolfs" },
        node => {
          nodeManager.asNode(node);
          cy.then(() =>
            node.client.project.create({
              repo: {
                type: "new",
                path: tempDirPath,
                name: "new-project",
              },
              description: "",
              defaultBranch: "main",
            })
          );
          commands.pick("sidebar", "settings").click();
          commands.pick("sidebar", "profile").click();
          commands.pick("project-list-entry-new-project").should("exist");
          nodeManager.exec(
            `cd "${tempDirPath}/new-project"
            git checkout -b my-branch
            git commit --allow-empty -m "${commitMessage}"
            git tag -a radicle-patch/${patchId} -m "${patchTitle}\n\n${patchDescription}"
            git push --tag rad;`,
            node
          );
          commands.pick("sidebar", "profile").click();
          commands.pick("project-list-entry-new-project").click();
          commands.pick("patches-tab").click();
          commands
            .pickWithContent(["patches-tab", "counter"], "1")
            .should("exist");

          cy.log("verifying the contents of the patch list page");
          commands
            .pick(`patch-card-${patchId}`)
            .should("contain", patchId)
            .should("contain", "Opened")
            .should("contain", "rudolfs");
          commands
            .pick(`patch-card-${patchId}`, "compare-branches")
            .should("contain", "main")
            .should("contain", patchId);

          cy.log("checking the navigation");
          commands.pick(`patch-card-${patchId}`).click();
          commands.pick("patch-page").should("exist");
          commands
            .pickWithContent(
              ["patch-page", "history", "commit-group", "commit"],
              commitMessage
            )
            .click();
          commands.pick("commit-page").should("exist");
          commands.pick("commit-page", "back-button").click();
          commands.pick("patch-page").should("exist");
          commands.pick("patch-page", "back-button").click();
          commands.pick("patch-list").should("exist");

          cy.log("verifying the contents of the patch page");
          commands.pick(`patch-card-${patchId}`).click();
          commands
            .pickWithContent(["checkout-patch-modal-toggle"], "Checkout")
            .should("exist");
          commands
            .pickWithContent(["merge-patch-modal-toggle"], "Merge")
            .should("exist");
          commands.pickWithContent(["patch-page"], patchTitle).should("exist");
          commands
            .pickWithContent(["patch-page"], patchDescription)
            .should("exist");
          commands.pickWithContent(["patch-page"], "Opened").should("exist");
          commands.pickWithContent(["patch-page"], "rudolfs").should("exist");
          commands
            .pickWithContent(["patch-page", "compare-branches"], "main")
            .should("exist");
          commands
            .pickWithContent(["patch-page", "compare-branches"], patchId)
            .should("exist");

          cy.log("verify that only the single patch commit is displayed");
          commands
            .pick("patch-page", "history", "commit-group")
            .should("have.length", 1);
          commands
            .pick("patch-page", "history", "commit-group", "commit")
            .should("have.length", 1);
          commands
            .pick("patch-page", "history", "commit-group", "commit")
            .should("contain", commitMessage);
        }
      );
    });
  });

  it("shows patches without a message", () => {
    commands.resetProxyState();
    commands.withTempDir(tempDirPath => {
      nodeManager.withOneOnboardedNode(
        { dataDir: tempDirPath, handle: "rudolfs" },
        node => {
          nodeManager.asNode(node);
          commands.createEmptyProject(node.client, "new-project", tempDirPath);
          commands.pick("sidebar", "settings").click();
          commands.pick("sidebar", "profile").click();
          commands.pick("project-list-entry-new-project").should("exist");
          nodeManager.exec(
            `cd "${tempDirPath}/new-project"
            git checkout -b my-branch
            git commit --allow-empty -m "${commitMessage}"
            git tag radicle-patch/${patchId} -m ""
            git push --tag rad;`,
            node
          );
          commands.pick("sidebar", "profile").click();
          commands.pick("project-list-entry-new-project").click();
          commands.pickWithContent(["patches-tab", "counter"], "1").click();

          cy.log("verifying the contents of the patch list page");
          commands
            .pick(`patch-card-${patchId}`)
            .should("contain", patchId)
            .should("contain", "Opened")
            .should("contain", "rudolfs");
          commands
            .pick(`patch-card-${patchId}`, "compare-branches")
            .should("contain", "main")
            .should("contain", patchId);

          commands.pick(`patch-card-${patchId}`).click();
          commands
            .pickWithContent(["patch-page", "patch-title"], patchId)
            .should("exist");
        }
      );
    });
  });

  it(
    "replicates a patch from contributor to maintainer",
    // Project replication may take longer than the default timeout.
    { defaultCommandTimeout: 8000 },
    () => {
      const maintainer = {
        handle: "rudolfs",
        passphrase: "1111",
      };
      const contributor = {
        handle: "abbey",
        passphrase: "2222",
      };

      commands.withTempDir(tempDirPath => {
        nodeManager.withTwoOnboardedNodes(
          {
            dataDir: tempDirPath,
            node1User: maintainer,
            node2User: contributor,
          },
          (maintainerNode, contributorNode) => {
            nodeManager.connectTwoNodes(maintainerNode, contributorNode);
            nodeManager.asNode(maintainerNode);

            const maintainerProjectsDir = path.join(
              tempDirPath,
              "maintainer-projects"
            );
            cy.exec(`mkdir -p "${maintainerProjectsDir}"`);

            const projectName = "new-fancy-project.xyz";
            cy.log("Create a project via API");
            commands.createEmptyProject(
              maintainerNode.client,
              projectName,
              maintainerProjectsDir
            );

            cy.log("refresh the UI for the project to show up");
            commands.pick("sidebar", "settings").click();
            commands.pick("sidebar", "profile").click();
            commands.pick("project-list-entry-new-fancy-project.xyz").click();

            commands
              .pickWithContent(
                ["project-screen", "header"],
                "new-fancy-project"
              )
              .should("exist");

            const contributorProjectsDir = path.join(
              tempDirPath,
              "contributor-projects"
            );

            commands.pick("project-screen", "header", "radicleId").then(el => {
              const urn = el.attr("data");
              if (!urn) {
                throw new Error("Could not find URN");
              }

              nodeManager.asNode(contributorNode);

              cy.log("contributor follows the project");
              cy.then(() => contributorNode.client.project.requestSubmit(urn));
              commands.pick("following-tab").click();
              commands
                .pick(
                  "following-tab-contents",
                  "project-list-entry-new-fancy-project.xyz"
                )
                .should("exist");

              cy.log("contributor checks out the project");
              cy.exec(`mkdir -p "${contributorProjectsDir}"`);
              cy.then(() =>
                contributorNode.client.project.checkout(urn, {
                  path: contributorProjectsDir,
                  peerId: maintainerNode.peerId,
                })
              );
            });

            cy.log("the project is now under the project tab");
            commands.pick("sidebar", "profile").click();
            commands
              .pick("project-list-entry-new-fancy-project.xyz")
              .should("exist");

            cy.log("test patch replication from contributor to maintainer");
            cy.log("add a patch to the project from contributor's node");
            const patchCommitSubject =
              "Merge request replication from contributor to maintainer";
            const forkedProjectPath = path.join(
              contributorProjectsDir,
              projectName
            );
            const patchTag = "feature-1";
            const patchMessage = "This is an awesome feature";

            nodeManager.exec(
              `cd "${forkedProjectPath}"
            git checkout -b my-branch
            git commit --allow-empty -m "${patchCommitSubject}"
            git tag -a --message "${patchMessage}" radicle-patch/${patchTag} HEAD
            git push --tag rad`,
              contributorNode
            );

            cy.log("refresh the UI for the patch to show up");
            commands.pick("sidebar", "profile").click();
            commands.pick("project-list-entry-new-fancy-project.xyz").click();

            cy.log("contributor sees the patch");
            commands.pick("patches-tab").click();
            commands
              .pickWithContent(["patch-list"], patchMessage)
              .should("exist");

            cy.log("add contributor remote on maintainer's node");
            nodeManager.asNode(maintainerNode);

            commands.pick("project-list-entry-new-fancy-project.xyz").click();

            commands.pick("project-screen", "header", "radicleId").then(el => {
              const urn = el.attr("data");
              if (!urn) {
                throw new Error("Could not find URN");
              }

              cy.then(() =>
                maintainerNode.client.project.peerTrack(
                  urn,
                  contributorNode.peerId
                )
              );
            });

            cy.log("maintainer received the contributor's patch");
            commands
              .pickWithContent(["patches-tab", "counter"], "1")
              .should("exist");
            commands.pick("patches-tab").click();
            commands.pickWithContent(["patch-list"], patchMessage).click();

            cy.log(
              "maintainer can see the patch details & navigate to the commit"
            );
            commands
              .pickWithContent(["patch-page"], patchMessage)
              .should("exist");
            commands
              .pickWithContent(
                ["patch-page", "history", "commit-group", "commit"],
                patchCommitSubject
              )
              .click();
            commands
              .pickWithContent(["commit-page"], patchCommitSubject)
              .should("exist");
          }
        );
      });
    }
  );

  it.skip(
    "updates maintainer view when a patch has been received",
    // Project replication may take longer than the default timeout.
    { defaultCommandTimeout: 8000 },
    () => {
      const maintainer = {
        handle: "rudolfs",
        passphrase: "1111",
      };
      const contributor = {
        handle: "abbey",
        passphrase: "2222",
      };

      commands.withTempDir(tempDirPath => {
        nodeManager.withTwoOnboardedNodes(
          {
            dataDir: tempDirPath,
            node1User: maintainer,
            node2User: contributor,
          },
          (maintainerNode, contributorNode) => {
            nodeManager.connectTwoNodes(maintainerNode, contributorNode);
            nodeManager.asNode(maintainerNode);

            const maintainerProjectsDir = path.join(
              tempDirPath,
              "maintainer-projects"
            );
            cy.exec(`mkdir -p "${maintainerProjectsDir}"`);

            const projectName = "new-fancy-project.xyz";
            cy.log("Create a project via API");
            commands
              .createEmptyProject(
                maintainerNode.client,
                projectName,
                maintainerProjectsDir
              )
              .as("projectUrn");

            cy.log("refresh the UI for the project to show up");
            commands.pick("sidebar", "settings").click();
            commands.pick("sidebar", "profile").click();
            commands.pick("project-list-entry-new-fancy-project.xyz").click();

            commands
              .pickWithContent(
                ["project-screen", "header"],
                "new-fancy-project"
              )
              .should("exist");

            const contributorProjectsDir = path.join(
              tempDirPath,
              "contributor-projects"
            );

            nodeManager.asNode(contributorNode);
            cy.get<string>("@projectUrn").then(urn => {
              cy.log("contributor checks out the project");
              cy.then(() => contributorNode.client.project.requestSubmit(urn));
              commands.pick("following-tab").click();
              commands
                .pick(
                  "following-tab-contents",
                  `project-list-entry-${projectName}`
                )
                .should("exist");

              cy.exec(`mkdir -p "${contributorProjectsDir}"`);
              cy.then(() =>
                contributorNode.client.project.checkout(urn, {
                  path: contributorProjectsDir,
                  peerId: maintainerNode.peerId,
                })
              );
            });

            cy.log("maintainer tracks peer");
            nodeManager.asNode(maintainerNode);
            cy.get<string>("@projectUrn").then(urn => {
              cy.then(() =>
                maintainerNode.client.project.peerTrack(
                  urn,
                  contributorNode.peerId
                )
              );
            });

            commands.pick(`project-list-entry-${projectName}`).click();
            commands.pick("patches-tab").click();

            cy.log("add a patch to the project from contributor's node");
            const forkedProjectPath = path.join(
              contributorProjectsDir,
              projectName
            );
            const patchId = "feature-1";

            nodeManager.exec(
              `cd "${forkedProjectPath}"
            git checkout -b my-branch
            git commit --allow-empty -m "commit message"
            git tag -a --message "patch message" radicle-patch/${patchId} HEAD
            git push --tag rad`,
              contributorNode
            );

            commands.pick("patches-tab", "counter").should("contain", "1");
            commands.pickWithContent(["patch-list"], patchId).should("exist");

            cy.log("maintainer merges patch in background");
            nodeManager.asNode(contributorNode);
            commands.pick(`project-list-entry-${projectName}`).click();
            commands.pick("patches-tab").click();

            nodeManager.exec(
              `cd "${contributorProjectsDir}/${projectName}"
              git checkout main
              git pull rad "remotes/${contributorNode.peerId}/tags/radicle-patch/${patchId}"
              git push rad`,
              maintainerNode
            );

            commands
              .pickWithContent(
                ["patch-filter-tabs", "segmented-control-option"],
                "Closed"
              )
              .click();
            commands.pick(`patch-card-${patchId}`).should("exist");
          }
        );
      });
    }
  );
});
