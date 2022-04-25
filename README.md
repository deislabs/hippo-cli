# Hippo Client

`hippo` is an **experimental** client for the [Hippo
PaaS](https://github.com/deislabs/hippo).

The `hippo` tool interacts directly with the Hippo API. Its primary purpose is
to interact with the various endpoints provided by the
[hippo-openapi](https://github.com/fermyon/hippo-openapi) project.

Users seeking to build, deploy, and run applications should look at
[spin](https://github.com/fermyon/spin/).

## Using the Hippo Client

### Logging in

```console
$ hippo login
Enter username: bacongobbler
Enter password: [hidden]
Logged in as bacongobbler
```

Authentication is handled through `hippo login`, which logs into Hippo. With
`hippo login`, the Hippo URL is specified in the `--url` flag. Hippo requires
authentication: if `--username` or `--password` are not provided, the CLI will
prompt for that information.

Logging out can be performed with `hippo logout`, which logs out of Hippo.

```console
$ hippo logout
Logged out
```

If you want to skip server TLS verification, pass the `-k` flag to `hippo
login`. This can be useful if you are running development services with
self-signed certificates.

**Note: the `-k` and `--danger-accept-invalid-certs` flags are a security risk.
Do not use them in production.**

### Creating an Application

```console
$ hippo app add helloworld helloworld
Added App helloworld (ID = 'e4a30d14-4536-4f4a-81d5-80e961e7710c')
IMPORTANT: save this App ID for later - you will need it to update and/or delete the App
```

### Creating a Channel

```console
$ hippo channel add latest e4a30d14-4536-4f4a-81d5-80e961e7710c
Added Channel latest (ID = '685ff7d8-7eef-456f-ad5a-4c5c39975588')
IMPORTANT: save this Channel ID for later - you will need it to update and/or delete the Channel
```

If not specified, Hippo to deploys the latest revision. This can be changed by
either providing a different `--range-rule`, or by specifying a `--revision-id`.

By default, Hippo will bind the channel to a domain with the address
`<channel_name>.<app_name>.<platform_domain>`. In this case,
`latest.helloworld.hippofactory.local`. If you want to change this domain,
use the `--domain` flag.

### Creating a Revision

If you pushed a bindle to bindle-server called `helloworld/1.0.0`:

```console
$ hippo revision add helloworld 1.0.0
Added Revision 1.0.0
```

If any applications use that storage ID, all its channels will be re-evaluated
to determine if they need to be re-schedule the new revision to the job
scheduler.

### Adding an Environment Variable

```console
$ hippo env add HELLO world 685ff7d8-7eef-456f-ad5a-4c5c39975588
Added Environment Variable HELLO (ID = 'c97f9855-d998-4dac-889b-11b553f53bea')
IMPORTANT: save this Environment Variable ID for later - you will need it to update and/or delete the Environment Variable
```

## Building from source

```console
cargo build --release
```

## Contributing

This project welcomes contributions and suggestions.  Most contributions require
you to agree to a Contributor License Agreement (CLA) declaring that you have
the right to, and actually do, grant us the rights to use your contribution. For
details, visit <https://cla.microsoft.com>.

When you submit a pull request, a CLA-bot will automatically determine whether
you need to provide a CLA and decorate the PR appropriately (e.g., label,
comment). Simply follow the instructions provided by the bot. You will only need
to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of
Conduct](https://opensource.microsoft.com/codeofconduct/). For more information
see the [Code of Conduct
FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or contact
[opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional
questions or comments.
