# Illustrative Passing By setup

This example is not required by Hannes and does not assume the application source
is present here. From your macOS application's Git root, run `hannes fang an`,
then copy `build.toml` into `.Hannes/checks/` and `product.toml`/`product.md` into
`.Hannes/reviews/`. Set `project = "Passing By"` in `.Hannes/config.toml`.
Edit the Xcode project, scheme, destination and review criteria to match the real
repository before enabling them. Both examples start disabled; `pass auf` still
validates their structure. Keep the generic `always-ok` or replace it with real
project checks. Commands execute on the user's machine, not inside a build service.
