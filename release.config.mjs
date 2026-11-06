export default {
  branches: ["main"],
  tagFormat: "v${version}",
  plugins: [
    ["@semantic-release/commit-analyzer", {
      preset: "conventionalcommits",
      releaseRules: [
        { breaking: true, release: "major" },
        { type: "feat", release: "minor" },
        { type: "fix", release: "patch" },
        { type: "perf", release: "patch" },
        ...["build", "chore", "ci", "docs", "refactor", "test"].map(type => ({ type, release: false })),
      ],
    }],
    ["@semantic-release/release-notes-generator", { preset: "conventionalcommits" }],
    ["@semantic-release/changelog", { changelogFile: "CHANGELOG.md" }],
    "./scripts/semantic-release-plugin.mjs",
    ["@semantic-release/git", {
      assets: ["Cargo.toml", "Cargo.lock", "CHANGELOG.md"],
      message: "build(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}",
    }],
    ["@semantic-release/github", {
      assets: ["dist/*.tar.gz", "dist/SHA256SUMS", "README.md", "LICENSE"],
      successComment: false,
      failComment: false,
      releasedLabels: false,
    }],
  ],
};
