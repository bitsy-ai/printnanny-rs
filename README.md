# 🦀 Print Nanny Command-line Interface

## Badges

## CI/CD Configuration

There are two main CI files in this template.

- [`mean_bean_ci.yml`](./.github/workflows/mean_bean_ci.yml) — Handles push & pull request build and testing.
- [`mean_bean_deploy.yml`](./.github/workflows/mean_bean_deploy.yml) — Handles deploying to GitHub Releases on tag push.

  - Update the [`BIN`](https://github.com/XAMPPRocky/mean-bean-ci-template/blob/master/.github/workflows/mean_bean_deploy.yml#L10) variable to be your binary's name before deploying.
  - You can delete `mean_bean_deploy.yml` if don't want any deployments.

### Customising Build & Test
You can customise the build and testing in [`ci/build.bash`](./ci/build.bash) and [`ci/test.bash`](./ci/test.bash)
respectively.

### Known Issues

- Currently you cannot rebuild a deployed release if you have already built that target, without manually deleting the release first. This will be fixed soon.


### Credits

Created from [XAMPPRocky/mean-bean-ci-template](https://github.com/XAMPPRocky/mean-bean-ci-template)

This is a template for GitHub Actions meant as a successor to [`japaric/trust`](https://github.com/japaric/trust)
for handling [`XAMPPRocky/tokei`](https://github.com/XAMPPRocky/tokei)'s CI and
deployment. I decided to make it a template to be able share it across
projects and with the community.
