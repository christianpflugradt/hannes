import { execFileSync } from "node:child_process";

export function verifyRelease(_config, context) {
  const env = context.env ?? process.env;
  const execution = { cwd: context.cwd, env, encoding: "utf8" };
  if (context.nextRelease.version !== env.EXPECTED_RELEASE_VERSION) {
    throw new Error("Release version changed after planning; refuse mismatched binaries.");
  }
  const head = execFileSync("git", ["rev-parse", "HEAD"], execution).trim();
  const remote = execFileSync("git", ["ls-remote", "origin", "refs/heads/main"],
    execution).trim().split(/\s+/)[0];
  if (head !== env.GITHUB_SHA || remote !== head) {
    throw new Error("main changed during the release; rerun from its latest commit.");
  }
  execFileSync("python3", ["scripts/verify-release.py", context.nextRelease.version], {
    ...execution, stdio: "inherit",
  });
}

export function prepare(_config, context) {
  execFileSync("python3", ["scripts/set-release-version.py", context.nextRelease.version], {
    cwd: context.cwd, env: context.env ?? process.env, stdio: "inherit",
  });
}
