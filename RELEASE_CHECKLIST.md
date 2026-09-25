## Release Checklist

- `VERSION=<new version>` (including a `v` at the start)
- Update version in [`Cargo.toml`](./Cargo.toml).
- Run `cargo build` to update [`Cargo.lock`](./Cargo.lock).
- Add changes since last release to [`CHANGELOG.md`](./CHANGELOG.md). (You
  should do this with every commit!)
  - Update the top of the CHANGELOG to say the new version number with
    the release date, then start a new section for `main`
- Commit all changes with commit message: `vX.Y.Z Release`
- Tag commit and push it to GitHub: `git tag $VERSION && git push origin $VERSION`
- Publish new version to crates.io: `cargo publish`
- Wait for the `release` workflow triggered by the tag to finish, then
  download its artifacts: `jless-x86_64-unknown-linux-gnu.zip`,
  `jless-aarch64-unknown-linux-gnu.zip`, and `jless-aarch64-apple-darwin.zip`
- Create GitHub release
  - Click "Create new release"
  - Select tag
  - Copy stuff from `CHANGELOG.md` to description
  - Attach the binaries downloaded above
- Update the [`website` branch](https://github.com/PaulJuliusMartinez/jless/tree/website)
  - Update [`releases_page.rb`](https://github.com/PaulJuliusMartinez/jless/blob/website/releases_page.rb) with the new release
  - Update [`user_guide_page.rb`](https://github.com/PaulJuliusMartinez/jless/blob/website/user_guide_page.rb) with any new commands
