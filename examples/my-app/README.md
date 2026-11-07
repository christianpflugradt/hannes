# Illustrative My_App setup

From a macOS application's Git root, run `hannes fang an`, then copy the
`checks/build/` and `reviews/product/` directories into the corresponding
`.hannes/` directories. Set `project = "My_App"` in config.toml.
Customize the Xcode project, scheme, destination and prompt before enabling.
Both example Rules start disabled and remain validated. They run on the user's
machine with its permissions. The build Check requires macOS and Xcode.
