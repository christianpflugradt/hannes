# Illustrative My_App setup

This example is not required by Hannes and does not assume the application source
is present here. From your macOS application's Git root, run `hannes fang an`,
then copy `build.toml` into `.hannes/checks/` and `product.toml`/`product.md` into
`.hannes/reviews/`. Set `project = "My_App"` in `.hannes/config.toml`.
Edit the Xcode project, scheme, destination and review criteria to match the real
repository before enabling them. Both examples start disabled; `pass auf` still
validates their structure. Keep the generic `always-ok` or replace it with real
project checks. Commands execute on the user's machine, not inside a build service.
