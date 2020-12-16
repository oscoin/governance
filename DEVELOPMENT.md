# How do we work?

Our workflow is to put changes in feature branches which we submit for review
on GitHub as pull requests. Ideally a pull request is small and changes only
one aspect of the code at a time. After a pull request is reviewed by at least
one peer and passes all tests, it can be squash-merged into master.

💡 *We require all commits to be signed for a branch to be merged into
master. Learn more on setting up [commit signing][cs].*

To automate our release process as much as possible we're using
[Standard Version][sv]. Commits on master should be formatted according to
the [conventional commits specification][cc].

Here are a couple of examples:
```
  fix: fix clippy on CI (#430)
  refactor(ui): improve cypress spec reliability (#429)
  style(ui): icon refresh (#411)
  chore(release): 0.0.11 (#417)
  test(ui): add missing project creation specs (#404)
  feat(proxy): improve session (#380)
```

When a release is performed, a section in [CHANGELOG.md][ch] is automatically
generated with all the changes from these commit messages.


## UI

The UI is written in JavaScript, [Svelte][se] is our [component language][cl]
of choice and [Electron][el] wraps it all together into a native desktop
experience. The UI code is split into `/native` and `/ui`.

For dependency management and script execution we use `yarn`. Code formatting
is dictated by [prettier][pr] and linting is provided by [eslint][es]. Both
linting and formatting are enforced locally on a pre-commit basis with
[husky][hu] and [lint-staged][ls].

Additionally we run the same checks as separate build steps on our CI, just to
make sure only properly formatted and lint-free code lands into master.


### Running Upstream

You'll have to install some external dependencies to be able to compile the
proxy as well as the UI.

On macOS:
```
xcode-select --install
sudo xcodebuild -license
brew install yarn pkgconfig nettle
```

On Linux:
  - [Autoconf](https://www.gnu.org/software/autoconf)
  - [Clang](https://clang.llvm.org)
  - [Git](https://git-scm.com)
  - [GMP](https://gmplib.org)
  - [GNU M4](https://www.gnu.org/software/m4)
  - [Nettle](http://www.lysator.liu.se/~nisse/nettle)
  - [OpenSSL](https://www.openssl.org)
  - [Yarn](https://yarnpkg.com)

1. Get Upstream: `git clone git@github.com:radicle-dev/radicle-upstream.git`.
2. Install dependencies: `cd radicle-upstream && yarn install`.
3. Start Upstream in development mode: `yarn start`.


### Feature flagging

UI features that are experimental or under construction that find their way
into the main branch must be placed behind the feature flag, to make them
inaccessible for the general public.

We do that by using `native > ipc.ts > isExperimental` as a feature flag to
enable or disable said features accordingly to the mode in which we are running
the app.

See the [scripts](#scripts) section below to learn which commands to use to
toggle this flag accordingly to your current workflow.

The feature flag is only available in development mode. It is always disabled
in production.


### Running tests

Before running UI end-to-end tests locally you'll need to check out the latest
test fixtures which are included in this repository as a git submodule:

```sh
./scripts/test-setup.sh
```

💡 *You'll have to run the previous commands every time there are any updates
to the test fixture repository.*

We use [Cypress](https://www.cypress.io/) for integration tests and
[Jest](jestjs.io) for unit tests. You can find integration tests in the
`cypress/` directory and unit tests next to the modules they correspond to.

- To run all ui tests call: `yarn test`.
- To troubleshoot integration tests via the Cypress GUI, run:
  `yarn test:integration:debug`.
- To isolate a single integration test for debugging purposes, use
  the [`.only` method][on].
- To develop unit tests in watch mode, run: `yarn test:unit:watch`


### Building an Upstream package for your platform

You can build and package Upstream with: `yarn dist`. The generated package
will be in: `dist/` as `radicle-upstream-X.X.X.{dmg|AppImage}`.


### Scripts

To get a list of all available script commands, run: `yarn run`.
Here is a list of the most commonly used ones:

```sh
yarn start                  # Start Upstream in development mode
yarn start:experimental     # Start Upstream in experimental mode, showing
                            # unfinished features

yarn test                   # Run all ui tests
yarn test:integration       # Run only integration tests
yarn test:unit              # Run only unit tests
yarn test:integration:debug # Show the Cypress GUI, handy for visual debugging
yarn test:unit:watch        # Run Jest tests in watch mode

yarn dist                   # Bundles Upstream into an installable package

yarn release                # Start a two-step process to cut a new release,
                            # for more details have a look at ../DEVELOPMENT.md

yarn prettier:check         # Check UI code formatting
yarn prettier:write         # Auto-format UI code
yarn lint                   # Check UI code for linting errors
yarn reset:state            # Delete all local state: identity keys, monorepo
                            # and saved preferences
```


### Design System

The overall look of Upstream is governed by a style guide which is continuously
being improved and extended. This style guide is translated into code forming
the design system. The design system contains all design primitives which, in
turn, can be composed to create rich user experiences.

Most of the components defined by the design system can be conveniently seen on
one page within Upstream by pressing <kbd>shift</kbd> + <kbd>D</kbd>. This will
bring up the Design System Guide modal.

The purpose of the Design System Guide is to showcase all available primitives
and components. Having them all on a single screen allows us to see how changes
to components affect all variations at a glance. Therefore newly created
components should always be added to the Guide, explaining all the different
variations and use cases.


#### File structure

In Svelte everything is a component, so to be able to build a complex
application and still be able to navigate the code and make changes quickly, we
organize our components in groups defined by use-case, re-usability and
complexity. Currently you'll find the following types of components in the
`DesignSystem` directory:

  - `Primitive`: simple, yet highly reusable components like typography,
    buttons, form elements, spacing, positioning and other utilities.

    Components of this type are usually just wrappers around standard HTML
    elements with custom styling.

    There are currently two ways of organizing primitives:

      - as all-in-one components where the type of the component is passed down
        via a `variant` prop. This is for components which have a very similar
        markup, but whose styling differs across variants.  Examples in this
        category are: buttons, typography and positioning helpers.

      - as namespaced components, where the component markup is very different
        across variants, for example: form elements and icons.

    To decide which way to write a new primitive component, start by looking at
    how it's going to be used in code and then optimise for ergonomics.

    All public primitives are exported via a central `index.js` file, which
    makes consumption straightforward:

    ```html
    <script>
      import { Button, Title, Icon, Input } from "../DesignSystem/Primitive";
    </script>

    <Icon.House />
    <Button variant="secondary">OK</Button>
    ```

  - `Component`: reusable low-to-high complexity components.

    Sub-folders in `DesignSystem/Component` should only be created for breaking
    up larger components into smaller fragments. If a component is broken up in
    fragments, make sure to only export the component which is intended for
    public use.

    ```html
    <script>
      import { RadicleLogo } from "../DesignSystem/Component";
    </script>

    <RadicleLogo />
    ```

Next to `DesignSystem`, you'll find a directory called `Screens`. Screens bring
together components from the Design System forming what a user in the UI sees
as a whole screen. More complex screens, similar to components, can be broken
down into multiple fragments. In this case the screen will contain data
fetching and routing logic for the fragments. Fragments should be placed in a
directory named after the screen, like so:

```sh
.
├── RegisterProject                    # fragment directory
│   ├── ConfirmTransactionStep.svelte
│   ├── PickNameStep.svelte
│   ├── PickWalletStep.svelte
│   └── TransactionSummaryStep.svelte
└── RegisterProject.svelte             # screen
```

Finally, our file and directory naming rules are as follows:

  - Svelte components and directories containing components - PascalCase;
  - everything else, including `*.js` files and folders - camelCase;
  - all folders in `/ui` should be named in singular form as they represent a
    type, not content.


#### Styling

The main entry point of the electron renderer is `public/index.html`. This is
the file where any global styling which is not managed by Svelte should be
imported.

To avoid extra wrappers for positioning and spacing, and to allow style
overrides, components expose a `style` prop:

```html
  <Component style="margin-right: 24px"/>
```


#### Typography

The design system provides a constrained set of typographic styles. This
consists of a set of styled headers, a set of styled paragraphs and a set of
modifiers. These also overlap with the components we have in our design system
in Figma, where the design of the app exists. All classes are prefixed with
`typo-` so this might be helpful if you have any autocomplete in your editor.

For the headers you can just use `<h1>` up to `<h5>`, if you want to apply the
same styles to other html elements you can use the matching classes
`typo-header-1` to `typo-header-5` (use `<h1>` to `<h5>` where you can).

For text we you can use the classes that start with `typo-text`. These come
in 2 sizes, the normal one and `typo-text-small`. Check out
[typography.css](./public/typography.css) to get an idea of the possible
combinations. All the ones we're using in Figma are represented here.

The modifiers give us some flexibility and allow us to create classes for
certain css functionality we use over and over. Such as,
`typo-overflow-ellipsis` and `typo-all-caps`. These should be self-explanatory.

We also added a set of modifiers that allow you to add the font-family as a
class where you need it, here again we would recommend not doing that as most
styles should fit into one of the two categories above.

The only place in the app where we're not using this is in `<Markdown />`,
since the library we use doesn't allow us to overwrite the styles without using
global declarations. If you have any questions or improvements, open an issue
and we're happy to help you along.

#### Colors

The design system supports multiple color palettes via themes which can be
changed in the Settings screen.

Throughout the codebase we use only CSS variables. Raw color codes should not
be used so changes to global styling can be applied in one central place:
`public/colors.css`.

Read more about the colors used in Upstream in the [Color System post][cg].


## Proxy

All of Upstream's business logic tying together the radicle code collaboration
is provided to the UI via an HTTP API by a rust binary called the proxy. It
uses [warp][wa] to serve a RESTish JSON API.

For dependency management and execution of common tasks we use [Cargo][co]. To
get up to speed with common functionality and manifest file intricacies consult
the exhaustive [Cargo Book][cb].

The proxy binary's lifecycle is managed by the main renderer of the UI in:
`native/main.js`. When running `yarn dist` it is bundled together into an
application package by [electron-builder][eb].


### Running the proxy in stand-alone mode

To be able to build the proxy first install all required dependencies from the
[Running Upstream](#running-upstream) section.

To start the proxy binary, run: `cd proxy && cargo run`.
After that the API docs are served under `http://127.0.0.1:17246/docs`.


### Testing

The proxy and UI share the same test fixtures, if you haven't done it already,
set up the test fixtures like so:

```sh
git submodule update --init --remote
git submodule foreach "git fetch --all"
```

💡 *You'll have to run the submodule commands every time there are any updates
to the test fixture repository.*

Then run tests as usual: `cargo test --all-features --all-targets`.

We strive for two kinds of tests: classic unit tests contained in
implementation files and integration tests. The integration tests are meant to
assert correctness of the API provided by the proxy, these can be found under
`proxy/tests`. To find out where to place and how to lay out tests, check the
Rust book [test chapter][rt].


### File structure

The API exposes the application's domain logic. Therefore we try to treat it as
a thin layer exposing well-typed entities. The heavy lifting is done in the
modules named after the protocols we consume - [radicle-link][rl] through it
[radicle-surf][rs], for code collaboration. By isolating concerns this way, we
hope to enable ease-of-contribution to downstream teams. Empowering them to
reflect changes in their public APIs easily with code contributions to Upstream.


## CI setup

Our CI infrastructure runs on [Buildkite][bk]. The build process is run for
every commit which is pushed to GitHub. When tests pass, the build process
uploads the Upstream binary as a build artifact. If the UI end-to-end tests
fail, screenshots of the failing tests are uploaded instead of the binary.

All relevant configuration can be found here:

```sh
radicle-upstream/.buildkite
.
├── Dockerfile
├── pipeline.yaml
└── run.sh
```


### Docker image updates

We use a Docker image with all system dependencies pre-installed to reduce
build times. If you need to update this image, proceed as follows:

1. Install [Google Cloud SDK][gc].

2. Authenticate with Google Cloud: `gcloud auth configure-docker`, pick
   `[1] opensourcecoin` when asked for which project to use.

3. Prepare a new docker image with all the necessary dependencies by editing:
   `.buildkite/Dockerfile`.

4. Get the current image version from `pipeline.yaml` and build a new Docker
   image (remember to bump the version):

    ```sh
    cd .buildkite
    docker build . -t gcr.io/opensourcecoin/radicle-upstream:0.2.1
    ```

5. Push the new image version to Google Cloud:

   `docker push gcr.io/opensourcecoin/radicle-upstream:0.2.1`

6. Update the image version in `pipeline.yaml`:

   ```yaml
   DOCKER_IMAGE: 'gcr.io/opensourcecoin/radicle-upstream:0.2.1'
   ```

7. Commit changes to `Dockerfile` and `pipeline.yaml`. Pushing the changes will
   create a new branch and build the updated image.

## Releases

### Prerequisites

#### GitHub `hub` CLI tool

Please install the [`hub`][hb] CLI tool, we use it in our release automation
script to:
  - create a pull-request off of a release branch;
  - to merge the release branch into master;
  - to close the pull-request.

Then you'll have to create a _Personal access token_ for it in the
[GitHub Developer settings][gs] page and authenticate the CLI tool once
by running any command that does a request to GitHub, like so: `hub api`.
You'll be asked to provide your GitHub login and the access token.

#### Apple notarization

To allow macOS Gatekeeper [to recognise][so] our Upstream packages as genuine,
which allows the user to install and open Upstream without unnecessary
[security warnings][sw], we have to [sign and notarize][sn] our macOS packages.

For this we need:
  - a paid Apple developer account registered to Monadic
  - an Apple ID token for allowing the notarization script to run on behalf of
    our developer account
    - [Account Manage][ma] -> APP-SPECIFIC PASSWORDS -> Generate password…
  - a valid "Developer ID Application" certificate
    - [Certificates Add][ca] -> Developer ID Application
      **Note:** this can only be created via the company account holder

Once you've created the _Developer ID Application_ certificate, download it
locally and add it to your keychain by double clicking on the file.


## Preparing a release

To perform a release run: `git checkout master && yarn release` and follow the
instructions.

Once the release PR branch is merged into master, a build will be triggered on
Buildkite, this will build Upstream for both Linux and macOS (unsigned).

<details>
<summary>Commands to prepare a release</summary>

```sh
$ git checkout master
$ yarn release

Cutting release v0.0.11:

  ✔ git checkout master
  ✔ git branch release-v0.0.11 && git checkout release-v0.0.11
  ✔ yarn run standard-version
  ✔ git push origin release-v0.0.11
  ✔ hub pull-request -p --no-edit

Now fix up CHANGELOG.md if necessary and update QA.md
to cover the latest changes in functionality.

When everything is in shape, ask a peer to review the
pull request, but don't merge it via the GitHub UI:

  👉 https://github.com/radicle-upstream/pull/417

Finally, complete the release by running:

  👉 yarn release:finalize v0.0.11 417


$ yarn release:finalize v0.0.11 417

Finalizing release v0.0.11:

  ✔ hub api -XPUT "repos/radicle-dev/radicle-upstream/pulls/417/merge"
  ✔ git checkout master && git pull
  ✔ git tag v0.0.11 ed968ee61ec30a18653b621f645a6abe354d2d16
  ✔ git push --tags

Release v0.0.11 successfully completed! 👏 🎉 🚀
```
</details>

## Quality assurance

We already have an extensive end-to-end test suite which covers most features
and a good amount of edge cases, however it is impossible to eliminate every bug
and regression this way, that's why we perform a QA procedure before publishing
a release.

After a release is cut, we create a GitHub issue for every supported platform
which contains a QA checklist. Before publishing packages for a wider audience
someone from the team goes through the checklist and manually tests the app,
afterwards the team can evaluate whether the release is up to our standards.

**Title:** `QA: vX.X.X macOS`\
**Body** [QA.md][qa]

**Title:** `QA: vX.X.X Linux`\
**Body:** [QA.md][qa]

## Publishing a release

Once a release has passed QA, it is ready to be published. This involves a
couple of manual steps since the macOS release has to be signed and notarized
on a developer's **macOS** machine.

If you haven't already, please set up an Apple developer account and signing
certificates, see [Apple notarization][an] for more details.

To build, sign and notarize a macOS dmg package of the latest version run the
following commands:

```
cd radicle-upstream
git checkout vX.X.X

CSC_NAME="Monadic GmbH (XXXXXXXXXX)" \
APPLE_ID="XXXXXXX@monadic.xyz" \
APPLE_ID_PASSWORD="XXXX-XXXX-XXXX-XXXX" \
yarn dist
```

Where:
  - `CSC_NAME` is the name of the signing certificate
  - `APPLE_ID` is your Apple developer ID
  - `APPLE_ID_PASSWORD` is the app specific token generated from your Apple ID

**Note**: building a release might take a while, especially the notarization
step, because it has to upload the final package to the Apple notarization
server. Make sure you're on a stable internet connection.

Once the package is notarized you can upload it to the web using
[`gsutil`][gs]:

```bash
gsutil cp dist/radicle-upstream-X.X.X.dmg gs://releases.radicle.xyz
```

To be able to upload packages to the GCS bucket you will need the appropriate
permissions. Reach out to a co-worker if you don’t have them.

After this you also need to obtain and upload the Linux package.

FIXME(rudolfs): this doesn't work, we don't build images off of tags.

```bash
curl -sSLO https://builds.radicle.xyz/radicle-upstream/vX.X.X/radicle-upstream-X.X.X.AppImage
gsutil cp radicle-upstream-X.X.X.AppImage gs://releases.radicle.xyz
```

After all the packages are uploaded, update the links to those binaries on the
[radicle.xyz download][rd] and [docs.radicle.xyz/docs/getting-started][gs]
pages and rebuild/deploy the websites.

The final step is to announce the new release on our public channels (**make
sure to update all the versions and links**):

  - https://radicle.community/c/announcements
    - subject:

          Radicle Upstream vX.X.X is out! 🎉

    - body text:

          # Radicle Upstream vX.X.X is out! 🎉

          You can find all the changelog for this release [here][1].

          Here are packages for all our supported platforms:

          - [macOS][2]
          - [Linux][3]

          For more information on how to use Radicle, check out our
          [documentation][4].

          For support, you can reach us in the [#support channel][5] of our Matrix
          chat or in the #help category of this forum.

          If you encounter a bug, please [open an issue][6].

          [1]: https://github.com/radicle-dev/radicle-upstream/blob/master/CHANGELOG.md#XXX-XXXX-XX-XX
          [2]: https://releases.radicle.xyz/radicle-upstream-X.X.X.dmg
          [3]: https://releases.radicle.xyz/radicle-upstream-X.X.X.AppImage
          [4]: https://docs.radicle.xyz/docs/what-is-radicle.html
          [5]: https://matrix.radicle.community/#/room/#support:radicle.community
          [6]: https://github.com/radicle-dev/radicle-upstream/issues

  - Post the following two lines on:
    https://matrix.radicle.community/#/room/#general:radicle.community

        Radicle Upstream vX.X.X is out! 🎉
        https://radicle.community/t/radicle-upstream-vX-X-X-is-out

## Checklist
To avoid missing a step when performing a release, here's an ordered checklist
for all of the required steps:

- [ ] cut the release
  - [ ] fix up `CHANGELOG.md` if there are any mistakes
  - [ ] wait for the release PR to pass CI
  - [ ] get 2 approvals for the release PR
  - [ ] finalize the release
- [ ] build and notarize macOS package
- [ ] wait for release commit to build Linux packages on CI
- [ ] upload Linux and macOS packages to releases.radicle.xyz
- [ ] create macOS and Linux QA issues in the Upstream repo
- [ ] wait until macOS and Linux QA is performed and passes
- [ ] update radicle.xyz download links
  - [ ] deploy radicle.xyz
- [ ] update docs.radicle.xyz download links
  - [ ] deploy docs.radicle.xyz
- [ ] announce new release on radicle.community
- [ ] announce new release on the matrix #general:radicle.community channel


[an]: #apple-notarization
[bk]: https://buildkite.com/monadic/radicle-upstream
[ca]: https://developer.apple.com/account/resources/certificates/add
[cb]: https://doc.rust-lang.org/cargo/
[cc]: https://www.conventionalcommits.org/en/v1.0.0
[cg]: https://radicle.community/t/color-system/166
[ch]: CHANGELOG.md
[cl]: https://gist.github.com/Rich-Harris/0f910048478c2a6505d1c32185b61934
[co]: https://github.com/rust-lang/cargo
[cs]: https://help.github.com/en/github/authenticating-to-github/signing-commits
[eb]: https://github.com/electron-userland/electron-builder
[el]: https://www.electronjs.org
[gc]: https://cloud.google.com/sdk/docs/quickstart-macos
[gg]: https://cloud.google.com/storage/docs/gsutil_install
[gp]: https://console.cloud.google.com/storage/browser/builds.radicle.xyz/releases/radicle-upstream
[gs]: https://github.com/radicle-dev/radicle-docs/blob/master/docs/getting-started.md
[gt]: https://github.com/settings/tokens
[hb]: https://github.com/github/hub
[hu]: https://github.com/typicode/husky
[ls]: https://github.com/okonet/lint-staged
[ma]: https://appleid.apple.com/account/manage
[on]: https://docs.cypress.io/guides/core-concepts/writing-and-organizing-tests.html#Excluding-and-Including-Tests
[pr]: https://prettier.io
[qa]: QA.md
[rd]: https://github.com/radicle-dev/radicle.xyz/blob/master/pages/downloads.html.mustache
[rl]: https://github.com/radicle-dev/radicle-link
[rs]: https://github.com/radicle-dev/radicle-surf/
[rt]: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
[se]: https://svelte.dev
[sn]: https://developer.apple.com/documentation/xcode/notarizing_macos_software_before_distribution
[so]: https://support.apple.com/en-us/HT202491
[sv]: https://github.com/conventional-changelog/standard-version
[sw]: https://support.apple.com/en-gb/guide/mac-help/mh40616/mac
[tp]: https://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html
[wa]: https://github.com/seanmonstar/warp
