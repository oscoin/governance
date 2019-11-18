import { GraphQLServer } from "graphql-yoga";
const util = require("util");
import path from "path";
const exec = util.promisify(require("child_process").exec);

const typeDefs = `
  type Info {
    mode: String!,
    isDirectory: Boolean!,
    lastCommit: String!,
    size: Int!,
    name: String!
  }

  type Entry {
    path: String,
    info: Info
  }

  type Query {
    ls(projectId: String!, head: String!, prefix: String!): [Entry]!
    cat(projectId: String!, head: String!, path: String!): String!
    branches(projectId: String!): [String]!
    tags(projectId: String!): [String]!
  }
`;

const debug = false;

async function ls(projectId, head, prefix) {
  debug && console.log(`projectId: ${projectId}`);
  debug && console.log(`head: ${head}`);
  debug && console.log(`prefix: ${prefix}`);

  const repoBasePath = path.resolve(__dirname, "../");
  debug && console.log(`repoBasePath: ${repoBasePath}`);

  const command = `git ls-tree --long ${head} ${repoBasePath}${prefix}`;
  debug && console.log(`command: ${command}`);
  const { stdout } = await exec(command);

  const relativePrefix = prefix.replace(/^\//, "");

  return stdout
    .split("\n") // split into rows
    .filter(el => el !== "") // throw out empty rows
    .map(row => {
      const [
        mode,
        treeOrBlob,
        lastCommit,
        sizeOrDash,
        nameWithPath
      ] = row.split(/\s+/);

      debug && console.log(`nameWithPath: ${nameWithPath}`);
      const name = nameWithPath.replace(new RegExp(`^${relativePrefix}`), "");
      debug && console.log(`name: ${name}`);
      debug && console.log("\n");

      return {
        path: prefix + name,
        info: {
          mode: mode,
          isDirectory: treeOrBlob === "tree",
          lastCommit: lastCommit,
          size: sizeOrDash === "-" ? 0 : parseInt(sizeOrDash),
          name: name
        }
      };
    })
    .sort(function(a, b) {
      // sort directories first, then files alphabetically
      if (a.info.isDirectory && !b.info.isDirectory) return -1;
      if (!a.info.isDirectory && b.info.isDirectory) return 1;
      if (a.info.toLowerCase > b.info.toLowerCase) return 1;
    });
}

async function cat(projectId, head, path) {
  debug && console.log(`projectId: ${projectId}`);
  debug && console.log(`head: ${head}`);
  debug && console.log(`path: ${path}`);

  const command = `git show ${head}:${path}`;
  debug && console.log(`command: ${command}`);
  const { stdout } = await exec(command);

  return stdout;
}

async function branches(_projectId) {
  const command = 'git branch -a --format="%(refname)"';
  debug && console.log(`command: ${command}`);

  const { stdout } = await exec(command);
  debug && console.log(stdout);

  return stdout
    .split("\n") // split into rows
    .filter(el => el !== ""); // throw out empty rows
}

async function tags(_projectId) {
  const command = "git tag -l";
  debug && console.log(`command: ${command}`);

  const { stdout } = await exec(command);
  debug && console.log(stdout);

  return stdout
    .split("\n") // split into rows
    .filter(el => el !== ""); // throw out empty rows
}

const resolvers = {
  Query: {
    ls: (_, { projectId, head, prefix }) => ls(projectId, head, prefix),
    cat: (_, { projectId, head, path }) => cat(projectId, head, path),
    branches: (_, { projectId }) => branches(projectId),
    tags: (_, { projectId }) => tags(projectId)
  }
};

const server = new GraphQLServer({ typeDefs, resolvers });
server.start(() => console.log("Server is running on http://localhost:4000"));
