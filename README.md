# DCO2

**DCO2** is a GitHub App that enforces the [Developer Certificate of Origin](https://developercertificate.org/) (DCO) on Pull Requests.

It aims to be a drop-in replacement for [dcoapp/app](https://github.com/dcoapp/app).

## Usage

To start using DCO2, you need to [configure the application](https://github.com/apps/dco-2) for your organization or repositories. To enforce the DCO check, you can enable [required status checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) in your branch protection settings.

## How it works

The [Developer Certificate of Origin](https://developercertificate.org) (DCO) is a lightweight way for contributors to certify that they wrote or otherwise have the right to submit the code they are contributing to the project.

Contributors *sign-off* that they adhere to [these requirements](https://developercertificate.org) by adding a `Signed-off-by` line to commit messages.

```text
This is my commit message

Signed-off-by: Joe Smith <joe.smith@example.com>
```

Git includes a `-s` command line option to append this line automatically to your commit message (provided you have configured your `user.name` and `user.email` in your git configuration):

```text
git commit -s -m 'This is my commit message'
```

Once installed, this application will create a check indicating whether or not commits in a Pull Request contain a valid `Signed-off-by` line.

## Contributing

Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for more details.

## Code of Conduct

This project follows the [CNCF Code of Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md).

## License

DCO2 is an Open Source project licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
