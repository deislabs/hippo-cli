# Hippo Client

`hippo` is an **experimental** client for the [Hippo
PaaS](https://github.com/deislabs/hippo).

The `hippo` tool interacts directly with the Hippo API. Its primary purpose is
to interact with the various endpoints provided by the
[hippo-openapi](https://github.com/fermyon/hippo-openapi) project.

Users seeking to build, deploy, and run applications should look at
[spin](https://github.com/fermyon/spin/).

## Using the Hippo Client

Authentication is handled through `hippo login`, which logs into Hippo. With
`hippo login`, the Hippo URL is specified in the `--url` flag. Hippo requires
authentication: if `--username` or `--password` are not provided, the CLI will
prompt for that information.

Logging out can be performed with `hippo logout`, which logs out of Hippo.

If you want to skip server TLS verification, pass the `-k` flag to `hippo
login`. This can be useful if you are running development services with
self-signed certificates.

**Note: the `-k` and `--danger-accept-invalid-certs` flags are a security risk.
Do not use them in production.**

## Building from source

* Known link failure on WSL: workaround is to build once with `RUSTFLAGS='-C
opt-level=0' cargo build` (after which plain `cargo build` seems to work)

## Contributing

This project welcomes contributions and suggestions.  Most contributions require
you to agree to a Contributor License Agreement (CLA) declaring that you have
the right to, and actually do, grant us the rights to use your contribution. For
details, visit https://cla.microsoft.com.

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
