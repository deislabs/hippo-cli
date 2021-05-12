# hippofactory

The `hippofactory` tool processes an application's `HIPPOFACTS` (Hippo
artifacts) file and generates a bindle that it can either push directly
or can later be uploaded using `bindle push`.

## The HIPPOFACTS file

HIPPOFACTS is a TOML file with the following structure:

```toml
[bindle]
name = "weather"
version = "1.2.3"
description = "erodes stone over long periods of time"  # optional
authors = ["Joan Q Programmer"]                         # optional

[files]
# NOTE: the grouping is currently ignored - all matches in any section
# are entered as parcels in the invoice
server = [
    "out/*.wasm"
]
client = [
    "scripts/*.js"
]
```

The `bindle` section is copied directly to `invoice.toml`, _except_ that in development
mode a prerelease segment is appended to the version to make it unique.

Each entry in the `files` section is matched against the file system, and a
parcel is entered into `invoice.toml` for each match. The parcel label name is
the relative path of the file to the `HIPPOFACTS` file.

For example, given the following file structure:

```
|- HIPPOFACTS
|- src
|  |- main.rs
|  |- utils.rs
|- out
|  |- program.wasm
|- scripts
   |- animation.js
   |- manifest.json
   |- utils.js
```

the previous `HIPPOFACTS` would enter parcels with the names `out/program.wasm`, `scripts/animation.js`
and `scripts/utils.js`.

`hippofactory` does not currently support Bindle's `parcel.label.feature`, `parcel.conditions`, `group`
or `signature` features.  It does not yet support push options other than the server URL (e.g. auth).

## Running hippofactory

As a developer you can run `hippofactory .` in your `HIPPOFACTS` directory to assemble all matching
files and push them to the Bindle server specified in the `BINDLE_SERVER_URL` environment variable.
(If you don't want to set the environment variable, pass the `-s` argument with the URL.)

In this mode, `hippofactory`:

* Mangles the version with a prerelease segment
* Stages to a temporary directory
* Pushes to the Bindle server

If you want to review the proposed bindle rather than pushing it, pass `--prepare -d <staging_dir>`.
This will stage the bindle to the specified directory but _not_ push it.  (If you later want
to push it, you can do so using the separate `bindle` tool.)

In a CI environment you can supply the `-v production` option to suppress version mangling.
This will create and upload the bindle with the version from `HIPPOFACTS`, without the
prerelease segment.

## Building from source

* Known link failure on WSL: workaround is to build once with `RUSTFLAGS='-C opt-level=0' cargo build`
(after which plain `cargo build` seems to work)

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.microsoft.com.

When you submit a pull request, a CLA-bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., label, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.
