// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import path from "path";
import sveltePreprocess from "svelte-preprocess";
import webpack, { WebpackPluginInstance } from "webpack";
import TerserWebpackPlugin from "terser-webpack-plugin";
import HtmlWebpackPlugin from "html-webpack-plugin";
import TsconfigPathsPlugin from "tsconfig-paths-webpack-plugin";
import { LicenseWebpackPlugin } from "license-webpack-plugin";

// @ts-expect-error there are no typings for this module
import spdxExpressionParse from "spdx-expression-parse";
// @ts-expect-error there are no typings for this module
import spdxWhitelisted from "spdx-whitelisted";

interface Argv {
  mode?: "production" | "development";
}

const tsRule = {
  test: /\.ts$/,
  exclude: /node_modules/,
  use: {
    loader: "ts-loader",
    options: {
      compilerOptions: {
        noEmit: false,
        module: "es6",
      },
    },
  },
};

function electronMain(_env: unknown, argv: Argv): webpack.Configuration {
  const mode = argv.mode || "development";
  return {
    name: "main",
    entry: {
      bundle: "./native/index.ts",
    },
    mode,
    cache: {
      type: "filesystem",
    },
    externals: ["fsevents"],
    target: "electron-main",
    externalsPresets: { electronMain: true, node: true },
    module: {
      rules: [tsRule],
    },
    resolve: {
      extensions: [".ts", ".js"],
    },
    output: {
      path: path.resolve(__dirname, "native"),
    },
    plugins: [licensePlugin()],
    optimization: {
      minimize: false,
    },
  };
}

function ui(_env: unknown, argv: Argv): webpack.Configuration {
  const mode = argv.mode || "development";
  const isProduction = mode === "production";
  return {
    name: "ui",
    entry: {
      bundle: "./ui/index.ts",
    },
    mode,
    devtool: isProduction ? "source-map" : "eval-source-map",
    cache: {
      type: "filesystem",
    },
    performance: {
      hints: false,
    },
    target: "web",
    module: {
      rules: [
        {
          test: /\.svelte$/,
          use: {
            loader: "svelte-loader",
            options: {
              compilerOptions: { dev: !isProduction },
              preprocess: sveltePreprocess(),
            },
          },
        },
        tsRule,
      ],
    },
    resolve: {
      fallback: {
        crypto: require.resolve("crypto-browserify"),
        stream: require.resolve("stream-browserify"),
        assert: require.resolve("assert"),
      },
      extensions: [".svelte", ".ts", ".js"],
      plugins: [
        new TsconfigPathsPlugin({
          extensions: [".svelte", ".ts", ".js"],
        }),
      ],
      // This is neccessary to prevent multiple versions of the svelte runtime
      // being bundled when depending on libraries containing svelte
      // components.
      // See https://github.com/sveltejs/svelte-loader#resolvealias
      alias: {
        svelte: path.resolve("node_modules", "svelte"),
      },
      mainFields: ["svelte", "browser", "module", "main"],
    },
    output: {
      path: path.resolve(__dirname, "public"),
    },
    plugins: [
      licensePlugin(),
      new webpack.ProvidePlugin({
        Buffer: ["buffer", "Buffer"],
        process: "process",
      }),
      new HtmlWebpackPlugin({
        template: "ui/index.html",
        meta: {
          "Content-Security-Policy": {
            "http-equiv": "Content-Security-Policy",
            content: isProduction
              ? "script-src 'self'"
              : "script-src 'self' 'unsafe-eval'",
          },
        },
      }),
    ],
    optimization: {
      minimize: isProduction,
      minimizer: [
        new TerserWebpackPlugin({
          extractComments: false, // prevents TerserPlugin from extracting a [chunkName].js.LICENSE.txt file
          terserOptions: {
            format: {
              // Tell terser to remove all comments except for the banner added via LicenseWebpackPlugin.
              // This can be customized further to allow other types of comments to show up in the final js file as well.
              // See the terser documentation for format.comments options for more details.
              comments: (_astNode, comment) =>
                comment.value.startsWith("! licenses are at "),
            },
          },
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
        }) as any,
      ],
    },
  };
}

export default [ui, electronMain];

// Must only include licenses that are GPLv3 compatible. This is mostly
// sourced from http://www.gnu.org/licenses/license-list.html
const allowedLicenses = [
  // 0BSD is less restrictive than ISC https://opensource.org/licenses/0BSD
  "0BSD",
  // http://www.gnu.org/licenses/license-list.html#apache2
  "Apache-2.0",
  // http://www.gnu.org/licenses/license-list.html#FreeBSD
  "BSD-2-Clause",
  // http://www.gnu.org/licenses/license-list.html#ModifiedBSD
  "BSD-3-Clause",
  // http://www.gnu.org/licenses/license-list.html#ccby
  "CC-BY-3.0",
  "CC-BY-4.0",
  // http://www.gnu.org/licenses/license-list.html#CC0
  "CC0-1.0",
  "GPL-3.0-only",
  // http://www.gnu.org/licenses/license-list.html#ISC
  "ISC",
  // http://www.gnu.org/licenses/license-list.html#LGPLv3
  "LGPL-3.0",
  // Named "Expat" on the GNU license overview
  // http://www.gnu.org/licenses/license-list.html#Expat
  "MIT",
  // http://www.gnu.org/licenses/license-list.html#MPL-2.0
  "MPL-2.0",
  // http://www.gnu.org/licenses/license-list.html#Unlicense
  "Unlicense",
  // http://www.gnu.org/licenses/license-list.html#WTFPL
  "WTFPL",
  // http://www.gnu.org/licenses/license-list.html#ZLib
  // "Zlib",
].map(x => spdxExpressionParse(x));

function licensePlugin(): WebpackPluginInstance {
  const plugin = new LicenseWebpackPlugin({
    stats: {
      warnings: false,
    },
    chunkIncludeExcludeTest: {
      include: ["bundle"],
    },
    addBanner: true,
    renderBanner: (filename, _modules) => {
      return `/*! licenses are at ${filename} */`;
    },
    additionalModules: [
      {
        name: "radicle-upstream",
        directory: __dirname,
      },
    ],
    licenseTypeOverrides: {
      // twemojji is licensed under MIT and CC-BY-4.0 but uses a
      // non-standard `license` field so that it cannot be parse
      // properly. https://github.com/twitter/twemoji/pull/499
      twemoji: "MIT AND CC-BY-4.0",
    },
    unacceptableLicenseTest: licenseName => {
      if (licenseName) {
        return !spdxWhitelisted(
          spdxExpressionParse(licenseName),
          allowedLicenses
        );
      } else {
        return true;
      }
    },
    excludedPackageTest: packageName => {
      // These packages have fake `package.json` files in
      // subdirectories. We don’t want to pick those up.
      return (
        packageName.startsWith("@apollo/client/") ||
        packageName.startsWith("ts-invariant/")
      );
    },
  });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return plugin as any;
}
