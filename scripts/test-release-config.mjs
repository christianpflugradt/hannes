import assert from "node:assert/strict";
import { test } from "node:test";
import { analyzeCommits } from "@semantic-release/commit-analyzer";
import { generateNotes } from "@semantic-release/release-notes-generator";
import config from "../release.config.mjs";

const logger = { log() {}, error() {} };
const options = config.plugins[0][1];
for (const [message, expected] of [
  ["feat(cli): add a command", "minor"],
  ["fix: correct a verdict", "patch"],
  ["perf: reduce repeated context", "patch"],
  ["feat!: remove a command", "major"],
  ["refactor(config)!: change configuration", "major"],
  ["fix: change format\n\nBREAKING CHANGE: old reports need migration", "major"],
  ["chore(deps): update rust crate toml to v1.2.0", null],
  ["chore(deps)!: drop an older runtime", "major"],
  ...["build", "chore", "ci", "docs", "refactor", "test"].map(type => [`${type}: maintenance`, null]),
]) {
  test(`release rule: ${message.split("\n")[0]}`, async () => {
    assert.equal(await analyzeCommits(options, {
      cwd: process.cwd(), commits: [{ hash: "abc", message }], logger,
    }), expected);
  });
}
test("notes preserve features, fixes and breaking changes", async () => {
  const notes = await generateNotes(config.plugins[1][1], {
    cwd: process.cwd(), logger,
    options: { repositoryUrl: "https://github.com/christianpflugradt/hannes.git" },
    commits: [{ hash: "a".repeat(40), message: "feat!: change review format" }],
    lastRelease: { gitTag: "v1.0.0" },
    nextRelease: { gitTag: "v2.0.0", version: "2.0.0" },
  });
  assert.match(notes, /BREAKING CHANGES/);
  assert.match(notes, /change review format/);
});
test("release is main-only with private tooling and no npm publication", () => {
  assert.deepEqual(config.branches, ["main"]);
  assert.ok(!config.plugins.some(plugin => JSON.stringify(plugin).includes("@semantic-release/npm")));
  const github = config.plugins.at(-1)[1];
  assert.equal(github.successComment, false);
  assert.equal(github.failComment, false);
});
