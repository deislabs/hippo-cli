# Hippo Client

`hippo` is an **experimental** client for the [Hippo PaaS](https://github.com/deislabs/hippo) and [Bindle](https://github.com/deislabs/bindle).

The `hippo` tool processes an application's `HIPPOFACTS` (Hippo
artifacts) file and generates a bindle that it can either push directly
or can later be uploaded using `bindle push`.

## The HIPPOFACTS file

HIPPOFACTS is a TOML file with the following structure:

```toml
[bindle]
name = "birdsondemand"
version = "1.2.3"
description = "provides birds as a service"  # optional
authors = ["Joan Q Programmer"]              # optional

[[handler]]
route = "/birds/flightless"
name = "bin/penguin.wasm"
files = ["photo/adelie.png", "photo/rockhopper.png", "stock/*.jpg"]

[[handler]]
route = "/birds/irritable/fighty"
name = "bin/cassowary.wasm"
# files key is optional

[[handler]]
route = "/birds/naughty"
name = "bin/kea.wasm"
files = ["stock/kea.jpg", "stock/wipers.jpg"]
```

The `bindle` section is copied directly to `invoice.toml`, _except_ that in development
mode a prerelease segment is appended to the version to make it unique.

Each `handler` table is processed as follows:

* A group for the handler is added to the invoice
* The `name` value is looked up in the file system, and a parcel is entered into the invoice
  for the corresponding file. The parcel's `conditions.requires` is set to the handler group.
* If the handler has a `files` key, all patterns in that array are matched against the file
  system, and a parcel is entered into the invoice for the corresponding file.  The parcel
  `label.name` is the relative path of the file to the `HIPPOFACTS` file.The parcel's
  `conditions.memberOf` is set to a list of _all_ handler groups that contained patterns that
  the file matched - this may be more than one if multiple handler file patterns matched the
  same file.

For example, given the following file structure:

```
|- HIPPOFACTS
|- src
|  |- main.rs
|  |- utils.rs
|- bin
|  |- cassowary.wasm
|  |- kea.wasm
|  |- kokako.wasm
|  |- penguin.wasm
|- photo
|  |- adelie.png
|  |- emperor.png
|  |- rockhopper.png
|- stock
   |- kea.jpg
   |- little-blue.jpg
   |- little-blue.png
   |- wipers.jpg
```

the previous `HIPPOFACTS` would create the following invoice (omitting some details
and adding comments):

```toml
bindleVersion = '1.0.0'

[bindle]
name = 'birdsondemand'
version = '1.2.3-ivan-2021.05.31.16.49.09.990'
description = 'provides birds as a service'
authors = ['Joan Q Programmer']

# Parcels representing handler WASM modules have a `requires` attribute
# and a `wagi.route` feature

[[parcel]]
[parcel.label]
sha256 = '0a4346f806b28b3ce94905c3ac56fcd5ee2337d8613161696aba52eb0c3551cc'
name = 'bin/penguin.wasm'
[parcel.label.feature.wagi]
file = 'false'
route = '/birds/flightless'
[parcel.conditions]
requires = ['bin/penguin.wasm-files']

[[parcel]]
[parcel.label]
sha256 = '1f71511371129511321c45be058c60e23cf9ba898d8a3f3309555985b5027490'
name = 'bin/cassowary.wasm'
[parcel.label.feature.wagi]
file = 'false'
route = '/birds/irritable/fighty'
[parcel.conditions]
requires = ['bin/cassowary.wasm-files']

[[parcel]]
[parcel.label]
sha256 = 'bab02c178882085bf20defd15c0e8971edd95488a1ecb4a6273e6afcfb3c4030'
name = 'bin/kea.wasm'
[parcel.label.feature.wagi]
file = 'false'
route = '/birds/naughty'
[parcel.conditions]
requires = ['bin/kea.wasm-files']

# Parcels derived from `files` patterns have a `memberOf` attribute and a
# `wagi.file` feature of "true"

[[parcel]]
[parcel.label]
sha256 = 'e99f19705a23cbeeeade5d2b4f8b83fff09beb093552e82073cdb302ee10eb76'
name = 'photo/adelie.png'
[parcel.label.feature.wagi]
file = 'true'
[parcel.conditions]
memberOf = ['bin/penguin.wasm-files']

[[parcel]]
[parcel.label]
sha256 = 'e8f7b60dfe5ee560edd1ac616463a0682a0e7c57a5ce2a8fe5c0990e500d0ac5'
name = 'photo/rockhopper.png'
[parcel.label.feature.wagi]
file = 'true'
[parcel.conditions]
memberOf = ['bin/penguin.wasm-files']

[[parcel]]
[parcel.label]
sha256 = '843baaf5a63cbc38d4d4c00036b95e435254eece7480fb717c8a17dcdc2aeefc'
name = 'stock/little-blue.jpg'
[parcel.label.feature.wagi]
file = 'true'
[parcel.conditions]
memberOf = ['bin/penguin.wasm-files']

# Some files are matched by more than one handler's patterns

[[parcel]]
[parcel.label]
sha256 = '6451ab5be799a6aa52ce8b8a084a12066bb2dd8e1a73a692627bb96b4b9a72f0'
name = 'stock/wipers.jpg'
[parcel.label.feature.wagi]
file = 'true'
[parcel.conditions]
memberOf = [
    'bin/penguin.wasm-files',
    'bin/kea.wasm-files',
]

[[parcel]]
[parcel.label]
sha256 = '93c3a391d842e3b8032d560db4870b5426c5c05a9f2a60b187e567ae69d8e658'
name = 'stock/kea.jpg'
[parcel.label.feature.wagi]
file = 'true'
[parcel.conditions]
memberOf = [
    'bin/penguin.wasm-files',
    'bin/kea.wasm-files',
]

# Group per handler

[[group]]
name = 'bin/penguin.wasm-files'

[[group]]
name = 'bin/cassowary.wasm-files'

[[group]]
name = 'bin/kea.wasm-files'
```

`hippo` does not currently support Bindle's `parcel.label.feature`
or `signature` features.  It does not yet support push options other than the server URL (e.g. auth).

### External handlers

It is sometimes useful to have routes handled by 'library' parcels that perform common
functions like serving static assets from the filesystem. To do this, rather than
copying the Wasm handler locally, you can reference it directly from your HIPPOFACTS.
To do this:

* The library parcel but have an annotation named `wagi_handler_id`. This is the ID by
  which HIPPOFACTS will refer to it - this decouples the reference from volatile details
  like parcel name or SHA.
* Instead of a `name` in your `handler` table, you specify a `bindleId` and `handlerId`.

For example:

```toml
[[handler]]
external.bindleId = "deislabs/fileserver/1.0.0"
external.handlerId = "static"
route = "/images"
files = ["birds/*.jpg"]
```

The Hippo client will locate the specified `wagi_handler_id` in the given bindle, and create a
parcel in your invoice that points to the same blob but with a `requires` condition for
the handler group. It also creates parcels for any parcels that the handler `requires`
in its original bindle.

### Reusable library modules

To create a 'library' parcel that can be referenced as an external handler, provide an
`export` section.  This is similar to a local handler except instead of a `route` it
has an `id` - this is the name by which it can be referenced (it gets mapped to the
`wagi_handler_id` annotation).  For example:

```toml
[[export]]
name = "gallery.wasm"
id = "gallery"
files = ["cache/*.db"]
```

**NOTE:** If your spec consists only of exports, you should not notify Hippo, because
there will be no application mapped to the resultant bindle. Pass `-a bindle` to
push to the Bindle server but not register it with Hippo.

### Conditions

Sometimes a module, or its associated files, may appear in different places depending on
your build settings. For example, many languages support "debug" and "release" builds which
put their output in different directories.

To address this, you can make a HIPPOFACTS entry conditional, using the optional `condition`
property:

```toml
[[handler]]
route = "/"
condition = "$build_mode != 'release'"
name = "out/debug/birdbattle.wasm"

[[handler]]
route = "/"
condition = "$build_mode == 'release'"
name = "out/release/birdbattle.wasm"
```

A condition contains three parts:

* The _condition variable_. Variable references begin with a `$` and may contain alphanumeric
  characters and underscores.
* The _comparison operator_. This must be `==` (true if the variable matches the value) or
  `!=` (true if the variable _doesn't_ match the value).
* The _match value_. Values must be enclosed in single quotes, and may contain alphanumeric
  charaters, underscores, hyphens and periods.

When you run `hippo prepare`, `hippo bindle` or `hippo push`, only entries whose `condition`
evaluates to `true` are included in the generated bindle. By default, no variables are set,
but you can set them using the `-c`/`--build-condition` option. You can provide this
option multiple times. Each occurrence must have a value of the form `<name>=<value>`.
If a variable isn't set, it doesn't match any value (even an empty value).

For example, with the above fragment:

* `hippo push .` would create a parcel from `out/debug/birdbattle.wasm` and map it to the `/` route.
  (The `!=` condition is true because `build_mode` doesn't have a value.)
* `hippo push . -c build_mode=debug` would also create a parcel from `out/debug/birdbattle.wasm` and
  map it to the `/` route. (The `!=` condition is true because, although `build_mode` has a value,
  that value doesn't match `'release'`.)
* `hippo push . -c build_mode=release` would create a parcel from `out/release/birdbattle.wasm` (note the
  path) and map it to the `/` route. (The `==` condition is true: the value of `build_mode` is `release`.)
* `hippo push . -c buildmode=release` would create a parcel from `out/debug/birdbattle.wasm`, because the
  `buildmode` setting (without an underscore) doesn't get picked up for the refernce `$build_mode` (with
  an underscore). `hippo` will not warn you if you do this - it is not an error to set a variable that
  isn't used in the spec.
* `hippo push . -c build_mode release` is an error - you must have the equals sign and no spaces
  between the name and value.

The condition applies to the entire entry. For example, if there are files asociated with the
entry, they will appear in the bindle only if the condition was true.

## Running the Hippo Client

As a developer you can run `hippo push .` in your `HIPPOFACTS` directory to
assemble all matching files and publish them as a bindle. In this mode,
`hippo`:

* Mangles the version with a prerelease segment
* Stages to a temporary directory
* Pushes to the Bindle server
* Notifies Hippo that a new bindle version is available

The Bindle server is specified in the `BINDLE_URL` environment variable.
(If you don't want to set the environment variable, pass the `-s` argument with the URL.)

The Hippo URL is specified in the `HIPPO_URL` environment variable. Hippo
requires authentication: pass the username in `HIPPO_USERNAME` and the password in
`HIPPO_PASSWORD`. (The equivalent command line options are `--hippo-url`, `--hippo-username`
and `--hippo-password`.)

If you want to review the proposed bindle rather than pushing it, pass `hippo prepare -d <staging_dir> .`.
This will stage the bindle to the specified directory but _not_ push it. If you want to push the
generated bindle but not notify Hippo, use `hippo bindle .`.

In a CI environment you can supply the `-v production` option to suppress version mangling.
This will create and upload the bindle with the version from `HIPPOFACTS`, without the
prerelease segment.

If you want to skip server verification, pass the `-k` flag. This can be useful if you are running
development services with self-signed certificates. **This is a security risk: do not use it in production.**

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
